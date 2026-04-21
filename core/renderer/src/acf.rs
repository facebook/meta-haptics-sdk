// Copyright (c) Meta Platforms, Inc. and affiliates.
//
// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

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
