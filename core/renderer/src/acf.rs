// (c) Meta Platforms, Inc. and affiliates. Confidential and proprietary.

use serde::Deserialize;
use serde::Serialize;
use typeshare::typeshare;

use crate::continuous_oscillator_settings::ContinuousOscillatorSettings;
use crate::emphasis_oscillator_settings::EmphasisOscillatorSettings;

/// The 'Actuator Configuration Format' settings
#[derive(Default, Deserialize, Serialize, Copy, Clone)]
#[typeshare]
pub struct Acf {
    /// The settings used for rendering the continuous signal
    pub continuous: ContinuousOscillatorSettings,
    /// The settings used for rendering emphasis events
    pub emphasis: EmphasisOscillatorSettings,
}
