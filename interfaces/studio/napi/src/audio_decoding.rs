// Copyright (c) Meta Platforms, Inc. and affiliates.
//
// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

use haptic_audio_analysis::MultiChannelBehavior;
use haptic_audio_analysis::PreprocessingSettings;
use haptic_audio_analysis::VisualWaveform;
use haptic_audio_analysis::load_audio_data;
use haptic_audio_analysis::preprocess_audio;
use napi::bindgen_prelude::*;
use napi_derive::napi;

use crate::helpers::napi_error_from_dyn_error;

/// Monophonic audio data in 32 bit float format, with its associated sample rate
#[napi]
pub struct AudioData {
    pub(crate) data: Vec<f32>,
    pub(crate) sample_rate: usize,
}

/// Decodes audio file data
///
/// The input should contain the complete data from the audio file.
///
/// Multi channel audio will be summed to mono.
///
/// Support for decoding audio data comes from the Symphonia library.
/// The following formats are currently supported:
///   - WAV
///   - Ogg Vorbis
///   - FLAC
///   - MP3
#[napi]
pub fn decode_audio_data(data: Uint8Array, extension: Option<String>) -> Result<AudioData> {
    // using downmix as default behavior
    let result = load_audio_data(&data, extension.as_deref(), &MultiChannelBehavior::Downmix)
        .map_err(napi_error_from_dyn_error)?;

    Ok(AudioData {
        data: result.data,
        sample_rate: result.sample_rate,
    })
}

/// Provides the default analysis settings used by `run_audio_to_haptics_analysis`
#[napi]
pub fn default_preprocessing_settings() -> Result<serde_json::Value> {
    let settings = serde_json::to_value(PreprocessingSettings::default())?;
    Ok(settings)
}

/// Processes audio data in place with gain and normalization settings
#[napi]
pub fn preprocess_audio_data(data: &mut AudioData, settings: serde_json::Value) -> Result<()> {
    let settings: PreprocessingSettings = serde_json::from_value(settings)?;
    let verbose = false;
    preprocess_audio(&mut data.data, settings, verbose);
    Ok(())
}

/// Provides a waveform overview intended for visual display
#[napi]
pub fn generate_waveform_overview(audio_data: &AudioData) -> Result<serde_json::Value> {
    let waveform = VisualWaveform::new(&audio_data.data, audio_data.sample_rate as f32);
    let waveform = serde_json::to_value(&waveform)?;
    Ok(waveform)
}
