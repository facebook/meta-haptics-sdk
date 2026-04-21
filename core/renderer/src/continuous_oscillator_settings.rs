// (c) Meta Platforms, Inc. and affiliates. Confidential and proprietary.

use serde::Deserialize;
use serde::Serialize;
use typeshare::typeshare;

use crate::renderer_c::HapticRendererContinuousOscillatorSettings;

/// The settings used by the haptic renderer's ContinuousOscillator
///
/// See [crate::Acf].
#[derive(Copy, Clone, Default, Deserialize, Serialize)]
#[typeshare]
pub struct ContinuousOscillatorSettings {
    /// The gain of the continuous ContinuousOscillator
    ///
    /// Range: 0->1
    pub gain: f32,

    /// The amount of ducking to apply to the continuous signal when emphasis is active
    /// This is an additional gain applied to the continuous signal, for example
    /// a value of '1' means 'no ducking', while '0' means 'drop to silence while
    /// an emphasis event is active'.
    pub emphasis_ducking: f32,

    /// The minimum frequency in Hz of the continuous oscillator.
    #[serde(default)]
    pub frequency_min: f32,

    /// The maximum frequency in Hz of the continuous oscillator.
    #[serde(default)]
    pub frequency_max: f32,
}

impl From<ContinuousOscillatorSettings> for HapticRendererContinuousOscillatorSettings {
    fn from(val: ContinuousOscillatorSettings) -> Self {
        HapticRendererContinuousOscillatorSettings {
            gain: val.gain,
            emphasis_ducking: val.emphasis_ducking,
            frequency_min: val.frequency_min,
            frequency_max: val.frequency_max,
        }
    }
}
