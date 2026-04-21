// Copyright (c) Meta Platforms, Inc. and affiliates.
//
// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

use std::mem::MaybeUninit;

use itertools::Itertools;
use itertools::PeekingNext;

use crate::Acf;
use crate::RenderMode;
use crate::StreamingEvent;
use crate::StreamingEventType;
use crate::StreamingRamp;
use crate::renderer_c::HapticRenderer;
use crate::renderer_c::haptic_renderer_init;
use crate::renderer_c::haptic_renderer_process;
use crate::renderer_c::haptic_renderer_reset;
use crate::renderer_c::haptic_renderer_start_amplitude_ramp;
use crate::renderer_c::haptic_renderer_start_emphasis;
use crate::renderer_c::haptic_renderer_start_frequency_ramp;

/// Renders sparse haptic events into an `f32` data stream intended for actuator playback
///
/// The renderer contains oscillators that respond to continuous and emphasis events,
/// and produces rendered `f32` samples one at a time as output, with the output range depending
/// on the [`RenderMode`] that's being used.
///
/// See [`StreamingRenderer::process`] for more information.
pub struct StreamingRenderer {
    renderer: HapticRenderer,
}

impl StreamingRenderer {
    /// Makes a new StreamingRenderer with the given ACF and render settings
    pub fn new(acf: Acf, sample_rate: f32, render_mode: RenderMode) -> Self {
        let mut renderer = MaybeUninit::uninit();
        let renderer = unsafe {
            haptic_renderer_init(
                renderer.as_mut_ptr(),
                sample_rate,
                render_mode.into(),
                &mut acf.continuous.into() as _,
                &mut acf.emphasis.into() as _,
            );
            renderer.assume_init()
        };

        Self { renderer }
    }

    /// Resets the renderer to its initial settings
    pub fn reset(&mut self) {
        unsafe {
            haptic_renderer_reset(&mut self.renderer);
        }
    }

    /// Provides a sample of output
    ///
    /// Any events that need to be consumed (i.e. the current time is at or after the event time)
    /// will be consumed, affecting the state of the renderer's oscillators.
    pub fn process<I>(&mut self, current_time: f32, events: &mut I) -> f32
    where
        I: PeekingNext<Item = StreamingEvent>,
    {
        for next_event in events.peeking_take_while(|event| event.time <= current_time) {
            match next_event.event {
                StreamingEventType::AmplitudeRamp(StreamingRamp {
                    target, duration, ..
                }) => unsafe {
                    haptic_renderer_start_amplitude_ramp(&mut self.renderer, target, duration);
                },
                StreamingEventType::FrequencyRamp(StreamingRamp {
                    target, duration, ..
                }) => unsafe {
                    haptic_renderer_start_frequency_ramp(&mut self.renderer, target, duration);
                },
                StreamingEventType::Emphasis {
                    amplitude,
                    frequency,
                } => unsafe {
                    haptic_renderer_start_emphasis(&mut self.renderer, amplitude, frequency);
                },
            }
        }

        unsafe { haptic_renderer_process(&mut self.renderer) }
    }
}

#[cfg(test)]
mod tests {
    use haptic_data::HapticData;
    use haptic_data::test_utils::TestClip;
    use haptic_data::test_utils::amp_bp;
    use haptic_data::test_utils::emphasis_bp;

    use crate::Acf;
    use crate::ContinuousOscillatorSettings;
    use crate::EmphasisFrequencySettings;
    use crate::EmphasisOscillatorSettings;
    use crate::EmphasisShape;
    use crate::RenderMode;
    use crate::StreamingEventReader;
    use crate::StreamingRenderer;
    use crate::test_utils::approx_compare_slices;

    const GENERIC_ACF: Acf = Acf {
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

    // Renders the clip with AmpCurve mode
    fn render_clip(haptic_data: HapticData, sample_duration: f32) -> Vec<f32> {
        let sample_rate: f32 = 1.0 / sample_duration;
        let acf = GENERIC_ACF;
        let amp_envelope = &haptic_data.signals.continuous.envelopes.amplitude;
        let clip_duration = amp_envelope.last().unwrap().time - amp_envelope.first().unwrap().time;
        let sample_count = (clip_duration * sample_rate).ceil();

        let mut amplitudes: Vec<f32> = Vec::new();
        let mut event_reader = StreamingEventReader::new(&haptic_data);
        let mut renderer = StreamingRenderer::new(acf, sample_rate, RenderMode::AmpCurve);

        let mut samples_processed = 0.0;
        while samples_processed < sample_count {
            let current_time = sample_duration * samples_processed;
            let sample = renderer.process(current_time, &mut event_reader);
            amplitudes.push(sample);
            samples_processed += 1.0;
        }

        amplitudes
    }

    #[test]
    fn basic() {
        let clip = TestClip {
            amplitude: &[amp_bp(0.0, 0.0), amp_bp(1.0, 1.0)],
            frequency: &[],
        };

        let amplitudes = render_clip(clip.into(), 0.1);
        let expected_amplitudes = vec![
            0.0, 0.1111, 0.2222, 0.3333, 0.4444, 0.5555, 0.6666, 0.7777, 0.8888, 1.0,
        ];
        approx_compare_slices(&amplitudes, &expected_amplitudes);
    }

    #[test]
    fn redundant_breakpoints() {
        // The two clips have the same amplitude curve, but the second clip has a redundant
        // breakpoint. A redundant breakpoint shouldn't make a difference in rendering output.
        let basic_clip = TestClip {
            amplitude: &[amp_bp(0.0, 0.0), amp_bp(1.0, 1.0)],
            frequency: &[],
        };
        let clip_with_redundant_breakpoints = TestClip {
            amplitude: &[amp_bp(0.0, 0.0), amp_bp(0.5, 0.5), amp_bp(1.0, 1.0)],
            frequency: &[],
        };

        let amplitudes_basic_clip = render_clip(basic_clip.into(), 0.1);
        let _amplitudes_redundant_clip = render_clip(clip_with_redundant_breakpoints.into(), 0.1);
        let expected_amplitudes = vec![
            0.0, 0.1111, 0.2222, 0.3333, 0.4444, 0.5555, 0.6666, 0.7777, 0.8888, 1.0,
        ];
        approx_compare_slices(&amplitudes_basic_clip, &expected_amplitudes);
    }

    // Tests that emphasis is rendered at all
    #[test]
    fn emphasis() {
        // 0.2s long clip with constant amplitude, emphasis exactly in the middle
        let clip = TestClip {
            amplitude: &[
                amp_bp(0.0, 0.5),
                emphasis_bp(0.1, 0.5, 1.0, 1.0),
                amp_bp(0.2, 0.5),
            ],
            frequency: &[],
        };

        let _amplitudes = render_clip(clip.into(), 0.02);
    }
}
