// (c) Meta Platforms, Inc. and affiliates. Confidential and proprietary.

use std::cmp::max;
use std::cmp::min;
use std::time::Duration;

use haptic_data::HapticData;
use serde::Deserialize;
use serde::Serialize;
use typeshare::typeshare;

use crate::Acf;
use crate::ContinuousOscillatorSettings;
use crate::EmphasisFrequencySettings;
use crate::EmphasisOscillatorSettings;
use crate::EmphasisShape;
use crate::RenderMode;
use crate::StreamingEventReader;
use crate::StreamingRenderer;
use crate::error::Error;

/// The rendered Android waveform, in exactly the format expected by the Android API
/// VibrationEffect::createWaveform()
///
/// See <https://developer.android.com/reference/android/os/VibrationEffect#createWaveform(long[],%20int[],%20int)>
/// for more details.
pub struct Waveform {
    /// The amplitude of each sample, in the range [0, 255]
    pub amplitudes: Vec<i32>,
    /// The duration of each amplitude value, in milliseconds
    pub timings_ms: Vec<i64>,
}

/// Settings for rendering haptic data into an Android waveform
#[derive(Clone, Copy, Deserialize, Serialize)]
#[typeshare]
pub struct WaveformRenderSettings {
    /// The gain, overriding the gain in the ACF
    pub gain: f32,
    /// The duration of each sample
    pub sample_duration: Duration,
    /// The minimum and maximum duration of an emphasis. The actual duration of the emphasis depends
    /// on the emphasis frequency: At frequency 0.0, the minimum duration is used, and at frequency
    /// 1.0, the maximum duration is used. Between those values there is linear interpolation.
    /// These values override the corresponding values in the ACF.
    pub min_emphasis_duration: Duration,
    /// See min_emphasis_duration
    pub max_emphasis_duration: Duration,
    /// The ducking of the continous envelope [0.0 - 1.0], overriding the value from the ACF
    pub emphasis_ducking: f32,
}

impl Default for WaveformRenderSettings {
    fn default() -> Self {
        Self {
            gain: ANDROID_ACF.continuous.gain,
            sample_duration: Duration::from_millis(10),
            min_emphasis_duration: Duration::from_secs_f32(
                ANDROID_ACF.emphasis.frequency_min.duration_ms / 1000.0,
            ),
            max_emphasis_duration: Duration::from_secs_f32(
                ANDROID_ACF.emphasis.frequency_max.duration_ms / 1000.0,
            ),
            emphasis_ducking: ANDROID_ACF.continuous.emphasis_ducking,
        }
    }
}

/// This is a hardcoded constant instead of
/// json5::from_str(include_str!("../acfs/AndroidWaveform.acf")), to avoid the json5 dependency.
/// The json5 dependency adds more than 100kb extra to the Twilight Android APK size.
const ANDROID_ACF: Acf = Acf {
    continuous: ContinuousOscillatorSettings {
        gain: 1.0,
        emphasis_ducking: 0.5,
        frequency_min: 55.0,
        frequency_max: 200.0,
    },
    emphasis: EmphasisOscillatorSettings {
        gain: 1.0,
        fade_out_percent: 0.0,
        frequency_min: EmphasisFrequencySettings {
            output_frequency: 55.0,
            duration_ms: 36.4,
            shape: EmphasisShape::Sine,
        },
        frequency_max: EmphasisFrequencySettings {
            output_frequency: 165.0,
            duration_ms: 12.1,
            shape: EmphasisShape::Square,
        },
    },
};

/// Renders haptic data into an Android waveform
pub fn render_waveform(
    haptic_data: &HapticData,
    render_settings: WaveformRenderSettings,
) -> std::result::Result<Waveform, Error> {
    let sample_duration = render_settings.sample_duration.as_secs_f32();
    let sample_rate = 1.0 / sample_duration;

    let mut acf = ANDROID_ACF.clone();
    acf.continuous.gain = render_settings.gain;
    acf.emphasis.gain = render_settings.gain;
    acf.emphasis.frequency_min.duration_ms =
        render_settings.min_emphasis_duration.as_secs_f32() * 1000.0;
    acf.emphasis.frequency_max.duration_ms =
        render_settings.max_emphasis_duration.as_secs_f32() * 1000.0;
    acf.continuous.emphasis_ducking = render_settings.emphasis_ducking;

    let amp_envelope = &haptic_data.signals.continuous.envelopes.amplitude;
    let clip_duration = match (amp_envelope.first(), amp_envelope.last()) {
        (Some(first), Some(last)) => last.time - first.time,
        _ => {
            // Shouldn't get here due to previous validation
            return Err(Error::InvalidHapticData(
                "Not enough amplitude points".to_string(),
            ));
        }
    };
    let sample_count = (clip_duration * sample_rate).ceil();

    let mut amplitudes: Vec<i32> = Vec::with_capacity(sample_count as usize);
    let mut timings_ms: Vec<i64> = Vec::with_capacity(sample_count as usize);
    let mut event_reader = StreamingEventReader::new(haptic_data);
    let mut renderer = StreamingRenderer::new(acf, sample_rate, RenderMode::AmpCurve);

    let mut samples_processed = 0.0;
    while samples_processed < sample_count {
        let current_time = sample_duration * samples_processed;
        let remaining_time = clip_duration - current_time;

        let sample_f32 = renderer.process(current_time, &mut event_reader);
        let sample_i32 = (sample_f32 * 255.0) as i32;
        // Prevent 0 values, as they turn the motor off, which produces an audible glitch, and it
        // might take some time to turn it on again.
        let sample_i32 = max(sample_i32, 1);
        // Prevent values higher than 255, which will make Android reject the waveform. This can
        // happen due to renderer bug T169571496.
        let sample_i32 = min(sample_i32, 255);
        let sample_duration_ms = min(
            (sample_duration * 1000.0) as i64,
            (remaining_time * 1000.0) as i64,
        );
        amplitudes.push(sample_i32);
        timings_ms.push(sample_duration_ms);

        samples_processed += 1.0;
    }

    Ok(Waveform {
        amplitudes,
        timings_ms,
    })
}

#[cfg(test)]
mod tests {
    use haptic_data::test_utils::TestClip;
    use haptic_data::test_utils::amp_bp;
    use haptic_data::test_utils::emphasis_bp;

    use super::*;

    // Check that for a 3s clip and a sample duration of 1s, 3 samples of 1s duration each are
    // produced (and not 4, which would be an off-by-one error)
    #[test]
    fn long_sample_duration() {
        let clip = TestClip {
            amplitude: &[amp_bp(0.0, 0.0), amp_bp(3.0, 1.0)],
            frequency: &[],
        };

        let waveform = render_waveform(
            &clip.into(),
            WaveformRenderSettings {
                gain: 1.0,
                sample_duration: Duration::from_millis(1000),
                ..Default::default()
            },
        )
        .unwrap();

        let expected_timings = vec![1000, 1000, 1000];
        let expected_amplitudes = vec![1, 127, 255];
        assert_eq!(expected_timings, waveform.timings_ms);
        assert_eq!(expected_amplitudes, waveform.amplitudes);
    }

    // Clip duration is 3.5s, and sample duration is 1s. Last sample should only be 500ms long.
    #[test]
    fn clip_duration_not_multiple_of_sample_duration() {
        let clip = TestClip {
            amplitude: &[amp_bp(0.0, 0.0), amp_bp(3.5, 1.0)],
            frequency: &[],
        };

        let waveform = render_waveform(
            &clip.into(),
            WaveformRenderSettings {
                gain: 1.0,
                sample_duration: Duration::from_millis(1000),
                ..Default::default()
            },
        )
        .unwrap();

        let expected_timings = vec![1000, 1000, 1000, 500];
        let expected_amplitudes = vec![1, 101, 204, 255];
        assert_eq!(expected_timings, waveform.timings_ms);
        assert_eq!(expected_amplitudes, waveform.amplitudes);
    }

    // Checks that amplitude values never exceed 255, even with gain 1.0 and emphasis
    #[test]
    fn max_amplitude() {
        let clip = TestClip {
            amplitude: &[emphasis_bp(0.0, 1.0, 1.0, 1.0), amp_bp(0.05, 1.0)],
            frequency: &[],
        };

        let waveform = render_waveform(
            &clip.into(),
            WaveformRenderSettings {
                gain: 1.0,
                sample_duration: Duration::from_millis(5),
                ..Default::default()
            },
        )
        .unwrap();

        assert!(waveform.amplitudes.iter().all(|&amp| amp <= 255));

        let expected_timings = vec![5, 5, 5, 5, 5, 5, 5, 5, 5, 5];
        let expected_amplitudes = vec![255, 255, 255, 255, 255, 255, 255, 255, 255, 255];
        assert_eq!(expected_timings, waveform.timings_ms);
        assert_eq!(expected_amplitudes, waveform.amplitudes);
    }

    // Verifies that setting a gain scales both the continuous and emphasis amplitudes
    #[test]
    fn gain_scales_amplitude() {
        let clip = TestClip {
            amplitude: &[
                amp_bp(0.0, 1.0),
                emphasis_bp(0.5, 1.0, 1.0, 1.0),
                amp_bp(1.0, 1.0),
            ],
            frequency: &[],
        };

        let waveform = render_waveform(
            &clip.into(),
            WaveformRenderSettings {
                gain: 0.1,
                sample_duration: Duration::from_millis(10),
                ..Default::default()
            },
        )
        .unwrap();

        // We should see a maximum of 0.2 (continuous and emphasis added together)
        let max_allowed = (0.2 * 255.0) as i32;
        assert!(waveform.amplitudes.iter().all(|&amp| amp <= max_allowed));
    }
}
