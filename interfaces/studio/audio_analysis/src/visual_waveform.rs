// Copyright (c) Meta Platforms, Inc. and affiliates.
//
// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

use serde::Deserialize;
use serde::Serialize;
use typeshare::typeshare;

use crate::AmplitudeAnalysisSettings;
use crate::AmplitudeAnalyzer;

/// A time and amplitude pair
///
/// See [VisualWaveform].
#[allow(missing_docs)]
#[derive(Deserialize, Serialize)]
#[typeshare]
pub struct TimeAmplitude {
    pub time: f32,
    pub amplitude: f32,
}

/// A structure for providing visual waveform data
#[derive(Default, Deserialize, Serialize)]
#[typeshare]
pub struct VisualWaveform {
    /// The waveform's amplitude envelope
    pub envelope: Vec<TimeAmplitude>,
}

impl VisualWaveform {
    /// Builds a new [VisualWaveform] by analyzing the provided monophonic audio data
    pub fn new(input: &[f32], sample_rate: f32) -> Self {
        // Run an amplitude analyzer with default settings to get waveform data
        let amplitude_analyzer = AmplitudeAnalyzer::new(
            input,
            sample_rate,
            AmplitudeAnalysisSettings {
                time_between_updates: 5.0e-3,    // 5ms
                envelope_attack_time: 1.0e-4,    // 0.1ms
                envelope_hold_time: 1.5e-3,      // 1.5ms
                envelope_release_time: 100.0e-3, // 100ms
                rms_windowing_time: 1.0e-4,      // 0.1ms
            },
        );

        Self {
            envelope: amplitude_analyzer
                .map(|event| TimeAmplitude {
                    time: event.time,
                    amplitude: event.amplitude,
                })
                .collect(),
        }
    }
}
