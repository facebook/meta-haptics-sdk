// Copyright (c) Meta Platforms, Inc. and affiliates.
//
// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

use std::io::Cursor;

use haptic_data::HapticData;
use haptic_renderer::Acf;
use haptic_renderer::HapticFileRenderer;
use haptic_renderer::RenderSettings;
use haptic_renderer::WaveformRenderSettings;
use haptic_renderer::render_waveform;
use napi::bindgen_prelude::*;
use napi_derive::napi;
use serde::Deserialize;
use serde::Serialize;
use typeshare::typeshare;

/// Like android_waveform::Waveform, only that the timings are i32 instead of i64, which typeshare
/// has trouble with: <https://github.com/1Password/typeshare/issues/24>
#[derive(Clone, Deserialize, Serialize)]
#[typeshare]
struct Waveform {
    amplitudes: Vec<i32>,
    timings_ms: Vec<i32>,
}

impl From<haptic_renderer::Waveform> for Waveform {
    fn from(waveform: haptic_renderer::Waveform) -> Self {
        Self {
            amplitudes: waveform.amplitudes,
            timings_ms: waveform
                .timings_ms
                .into_iter()
                .map(|timing| timing as i32)
                .collect(),
        }
    }
}

/// Provides the default ACF used by [render_haptic_data_to_audio]
#[napi]
pub fn default_acf() -> Result<serde_json::Value> {
    let acf = serde_json::to_value(Acf::default())?;
    Ok(acf)
}

/// Provides the default renderer settings used by [render_haptic_data_to_audio]
#[napi]
pub fn default_renderer_settings() -> Result<serde_json::Value> {
    let render_settings = serde_json::to_value(RenderSettings::default())?;
    Ok(render_settings)
}

/// Renders the provided haptic data to data that can be serialized to an audio file
///
/// The intended use is that the caller will serialize the resulting data to a file without
/// modification.
#[napi]
pub fn render_haptic_data_to_audio(
    haptic_data: serde_json::Value,
    acf: serde_json::Value,
    render_settings: serde_json::Value,
) -> Result<Uint8Array> {
    let haptic_data: HapticData = serde_json::from_value(haptic_data)?;
    let acf: Acf = serde_json::from_value(acf)?;
    let render_settings: RenderSettings = serde_json::from_value(render_settings)?;

    let mut output_buffer = Cursor::new(Vec::new());
    let mut renderer =
        HapticFileRenderer::new(&haptic_data, acf, render_settings).map_err(|error| {
            Error::from_reason(format!("Error while initializing renderer: {error}"))
        })?;

    renderer
        .write_to_buffer(&mut output_buffer)
        .map_err(|error| Error::from_reason(format!("Error while rendering output: {error}")))?;

    Ok(Uint8Array::from(output_buffer.into_inner()))
}

/// Renders haptic data into an Android Vibrator waveform
#[napi]
pub fn render_haptic_data_to_waveform(
    haptic_data: serde_json::Value,
    render_settings: serde_json::Value,
) -> Result<serde_json::Value> {
    let haptic_data: HapticData = serde_json::from_value(haptic_data)?;
    let render_settings: WaveformRenderSettings = serde_json::from_value(render_settings)?;
    let waveform = render_waveform(&haptic_data, render_settings)
        .map_err(|error| Error::from_reason(format!("Error while rendering waveform: {error}")))?;
    let waveform: Waveform = waveform.into();
    let waveform = serde_json::to_value(&waveform)?;
    Ok(waveform)
}

/// Returns the default render settings for render_haptic_data_to_waveform()
#[napi]
pub fn default_waveform_render_settings() -> Result<serde_json::Value> {
    let render_settings = serde_json::to_value(WaveformRenderSettings::default())?;
    Ok(render_settings)
}
