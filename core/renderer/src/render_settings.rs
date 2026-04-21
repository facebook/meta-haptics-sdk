// (c) Meta Platforms, Inc. and affiliates. Confidential and proprietary.

use serde::Deserialize;
use serde::Serialize;
use typeshare::typeshare;

use crate::renderer_c::HapticRendererMode;
use crate::renderer_c::HapticRendererMode_HAPTIC_RENDERER_MODE_AMP_CURVE;
use crate::renderer_c::HapticRendererMode_HAPTIC_RENDERER_MODE_SYNTHESIS;

/// Settings that define the output of the `HapticRenderer`
#[derive(Clone, Copy, Deserialize, Serialize)]
#[typeshare]
pub struct RenderSettings {
    /// The type of rendering that should take place, e.g. 'synthesis' or 'amp_curve'.
    pub render_mode: RenderMode,
    /// The format of the rendered output, e.g. 'wav', 'raw', or 'csv', defaults to 'wav'.
    pub output_format: OutputFormat,
    /// The sample rate of the renderer's audio output, defaults to 44.1kHz
    pub sample_rate: u32,
    /// The sample format of the renderer's audio output, defaults to 16-bit integer
    ///
    /// Note that this field is only used when calling the HapticRenderer::write_to_\[x\] functions.
    /// It's ignored when calling `HapticRenderer::output`, where a format can be chosen freely.
    pub sample_format: SampleFormat,
}

impl Default for RenderSettings {
    fn default() -> Self {
        Self {
            render_mode: RenderMode::Synthesis,
            output_format: OutputFormat::Wav,
            sample_rate: 44100,
            sample_format: SampleFormat::Signed16,
        }
    }
}

/// The format of rendered samples, see `RenderSettings`
#[derive(Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
#[typeshare]
pub enum SampleFormat {
    /// 8-bit unsigned integer
    Unsigned8,
    /// 16-bit signed integer
    Signed16,
    /// 16-bit signed integer
    Signed24,
    /// 32-bit signed integer
    Signed32,
    /// 32-bit floating point
    Float32,
}

/// The data format of the rendered output, see `RenderSettings`
#[derive(Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
#[typeshare]
pub enum OutputFormat {
    /// Raw
    ///
    /// The output values will be written out as raw data,
    /// without any context like sample rate or bit depth.
    Raw,
    /// WAV
    ///
    /// The output values will be prefixed with a .wav header
    Wav,
    /// CSV
    ///
    /// The values will be written out as text
    Csv,
}

impl SampleFormat {
    /// The number of bits represented by the format
    pub fn bits(&self) -> u16 {
        use SampleFormat::*;

        match self {
            Unsigned8 => 8,
            Signed16 => 16,
            Signed24 => 24,
            Signed32 | Float32 => 32,
        }
    }

    /// Whether the format uses integer or floating point values
    pub fn integer_or_float(&self) -> IntegerOrFloat {
        match self {
            SampleFormat::Float32 => IntegerOrFloat::Float,
            _ => IntegerOrFloat::Integer,
        }
    }
}

/// See `SampleFormat::integer_or_float`
#[typeshare]
#[allow(missing_docs)]
pub enum IntegerOrFloat {
    Integer,
    Float,
}

#[cfg(feature = "wav")]
impl From<IntegerOrFloat> for hound::SampleFormat {
    fn from(format: IntegerOrFloat) -> Self {
        match format {
            IntegerOrFloat::Integer => hound::SampleFormat::Int,
            IntegerOrFloat::Float => hound::SampleFormat::Float,
        }
    }
}

/// The rendering mode, see `RenderSettings`
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[typeshare]
pub enum RenderMode {
    /// Synthesis
    ///
    /// The output will be synthesized PCM audio data in the range -1.0..=1.0 that can directly
    /// drive an actuator.
    #[serde(rename = "synthesis")]
    Synthesis,

    /// Amplitude Curve
    ///
    /// The output will be a curve in the range 0..=1.0 that represents the intensity of an actuator
    /// over time.
    #[serde(rename = "amp_curve")]
    AmpCurve,
}

impl From<RenderMode> for HapticRendererMode {
    fn from(val: RenderMode) -> Self {
        match val {
            RenderMode::Synthesis => HapticRendererMode_HAPTIC_RENDERER_MODE_SYNTHESIS,
            RenderMode::AmpCurve => HapticRendererMode_HAPTIC_RENDERER_MODE_AMP_CURVE,
        }
    }
}
