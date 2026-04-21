// Copyright (c) Meta Platforms, Inc. and affiliates.
//
// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

use haptic_audio_analysis::OfflineAnalysisSettings;
use haptic_audio_analysis::audio_to_haptics;
use napi::bindgen_prelude::*;
use napi_derive::napi;

use crate::audio_decoding::AudioData;
use crate::helpers::napi_error_from_dyn_error;

/// Provides the default analysis settings used by `run_audio_to_haptics_analysis`
#[napi]
pub fn default_analysis_settings() -> Result<serde_json::Value> {
    let settings = serde_json::to_value(OfflineAnalysisSettings::default())?;
    Ok(settings)
}

/// Runs analysis on the provided audio with the provided settings, producing haptic data
///
/// See [`default_analysis_settings`].
#[napi]
pub fn run_audio_to_haptics_analysis(
    data: &AudioData,
    settings: serde_json::Value,
) -> Result<serde_json::Value> {
    let settings: OfflineAnalysisSettings = serde_json::from_value(settings)?;
    let validate_output = true;
    let verbose = false;

    // Analyse the audio data to produce haptic data
    let result = audio_to_haptics(
        &data.data,
        data.sample_rate as f32,
        settings,
        validate_output,
        verbose,
    )
    .map_err(napi_error_from_dyn_error)?;

    let result = serde_json::to_value(&result)?;
    Ok(result)
}
