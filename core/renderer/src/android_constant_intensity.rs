// Copyright (c) Meta Platforms, Inc. and affiliates.
//
// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

use std::time::Duration;

use haptic_data::HapticData;

use crate::error::Error;

/// Duration used for transient-only clips on Samsung devices, where the continuous amplitude
/// envelope is zero but emphasis events are present. A short fixed duration produces a brief click
/// rather than a long constant vibration spanning the full envelope.
const TRANSIENT_ONLY_DURATION_SECS: f32 = 0.030;

/// A vibration with a constant intensity
pub struct ConstantIntensityVibration {
    /// The duration of the vibration
    pub duration: Duration,
    /// The intensity of the vibration, from 0.0 to 1.0
    pub amplitude: f32,
}

/// Renders haptic data into a constant intensity vibration
pub fn render_constant_intensity(
    haptic_data: &HapticData,
) -> std::result::Result<ConstantIntensityVibration, Error> {
    let amplitude_points = &haptic_data.signals.continuous.envelopes.amplitude;

    let (first, last) = match (amplitude_points.first(), amplitude_points.last()) {
        (Some(first), Some(last)) if amplitude_points.len() >= 2 => (first, last),
        _ => {
            return Err(Error::InvalidHapticData(
                "Not enough amplitude points".to_string(),
            ));
        }
    };

    let total_duration = last.time - first.time;
    if total_duration <= 0.0 {
        return Err(Error::InvalidHapticData(
            "Amplitude envelope has zero or negative duration".to_string(),
        ));
    }

    // Calculate the time-weighted average amplitude.
    // For linearly interpolated segments, the average amplitude over a segment
    // from (t0, a0) to (t1, a1) is (a0 + a1) / 2.
    // We then weight each segment's average by its duration.
    let mut weighted_sum = 0.0;
    for window in amplitude_points.windows(2) {
        let (bp0, bp1) = (&window[0], &window[1]);
        let segment_duration = bp1.time - bp0.time;
        let segment_average = (bp0.amplitude + bp1.amplitude) / 2.0;
        weighted_sum += segment_average * segment_duration;
    }

    let mut average_amplitude = weighted_sum / total_duration;
    let mut vibration_duration = total_duration;

    // If the continuous amplitude is effectively zero, check for transients (emphasis events).
    // Transient-only clips have amplitude/frequency envelopes at 0 but contain emphasis events
    // that should still produce haptic output. Use a short fixed duration to produce a brief
    // click rather than a long constant vibration spanning the full envelope.
    if average_amplitude <= f32::EPSILON {
        let max_emphasis_amplitude = amplitude_points
            .iter()
            .filter_map(|bp| bp.emphasis.as_ref())
            .map(|e| e.amplitude)
            .fold(0.0_f32, f32::max);

        if max_emphasis_amplitude > 0.0 {
            average_amplitude = max_emphasis_amplitude;
            vibration_duration = TRANSIENT_ONLY_DURATION_SECS;
        }
    }

    Ok(ConstantIntensityVibration {
        duration: Duration::from_secs_f32(vibration_duration),
        amplitude: average_amplitude.clamp(0.0, 1.0),
    })
}

#[cfg(test)]
mod tests {
    use assert_approx_eq::assert_approx_eq;
    use haptic_data::test_utils::TestClip;
    use haptic_data::test_utils::amp_bp;
    use haptic_data::test_utils::emphasis_bp;

    use super::*;

    #[test]
    fn constant_amplitude() {
        let clip = TestClip {
            amplitude: &[amp_bp(0.0, 0.5), amp_bp(1.0, 0.5)],
            frequency: &[],
        };

        let result = render_constant_intensity(&clip.into()).unwrap();

        assert_approx_eq!(result.duration.as_secs_f32(), 1.0);
        assert_approx_eq!(result.amplitude, 0.5);
    }

    #[test]
    fn linear_ramp_up() {
        let clip = TestClip {
            amplitude: &[amp_bp(0.0, 0.0), amp_bp(2.0, 1.0)],
            frequency: &[],
        };

        let result = render_constant_intensity(&clip.into()).unwrap();

        assert_approx_eq!(result.duration.as_secs_f32(), 2.0);
        assert_approx_eq!(result.amplitude, 0.5);
    }

    #[test]
    fn non_uniform_segments() {
        // Weighted average = (0.5 * 1.0 + 0.75 * 2.0) / 3.0 = 2.0 / 3.0 ≈ 0.6667
        let clip = TestClip {
            amplitude: &[amp_bp(0.0, 0.0), amp_bp(1.0, 1.0), amp_bp(3.0, 0.5)],
            frequency: &[],
        };

        let result = render_constant_intensity(&clip.into()).unwrap();

        assert_approx_eq!(result.duration.as_secs_f32(), 3.0);
        assert_approx_eq!(result.amplitude, 2.0 / 3.0);
    }

    #[test]
    fn insufficient_amplitude_points() {
        let clip = TestClip {
            amplitude: &[amp_bp(0.0, 0.5)],
            frequency: &[],
        };

        let result = render_constant_intensity(&clip.into());
        assert!(result.is_err());
    }

    #[test]
    fn zero_duration() {
        let clip = TestClip {
            amplitude: &[amp_bp(0.0, 0.5), amp_bp(0.0, 0.5)],
            frequency: &[],
        };

        let result = render_constant_intensity(&clip.into());
        assert!(result.is_err());
    }

    #[test]
    fn transient_only_clip() {
        // Amplitude envelope is all zeros but there is a transient (emphasis) event.
        // The constant vibration should use the transient amplitude and a short fixed
        // duration (30ms) instead of the full envelope duration.
        let clip = TestClip {
            amplitude: &[
                amp_bp(0.0, 0.0),
                emphasis_bp(0.016, 0.0, 1.0, 1.0),
                amp_bp(0.032, 0.0),
            ],
            frequency: &[],
        };

        let result = render_constant_intensity(&clip.into()).unwrap();

        assert_approx_eq!(result.duration.as_secs_f32(), TRANSIENT_ONLY_DURATION_SECS);
        assert_approx_eq!(result.amplitude, 1.0);
    }

    #[test]
    fn transient_only_long_clip_uses_fixed_duration() {
        // A long clip with amplitude 0 and transients should still use the short fixed
        // duration (30ms) rather than the full envelope duration.
        let clip = TestClip {
            amplitude: &[
                amp_bp(0.0, 0.0),
                emphasis_bp(1.0, 0.0, 0.8, 1.0),
                amp_bp(5.0, 0.0),
            ],
            frequency: &[],
        };

        let result = render_constant_intensity(&clip.into()).unwrap();

        assert_approx_eq!(result.duration.as_secs_f32(), TRANSIENT_ONLY_DURATION_SECS);
        assert_approx_eq!(result.amplitude, 0.8);
    }

    #[test]
    fn transient_with_nonzero_amplitude_uses_average() {
        // When continuous amplitude is nonzero, it should still use the weighted average
        // and not be overridden by transients.
        let clip = TestClip {
            amplitude: &[
                amp_bp(0.0, 0.5),
                emphasis_bp(0.5, 0.5, 1.0, 1.0),
                amp_bp(1.0, 0.5),
            ],
            frequency: &[],
        };

        let result = render_constant_intensity(&clip.into()).unwrap();

        assert_approx_eq!(result.duration.as_secs_f32(), 1.0);
        assert_approx_eq!(result.amplitude, 0.5);
    }
}
