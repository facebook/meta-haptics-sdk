// (c) Meta Platforms, Inc. and affiliates. Confidential and proprietary.

use serde::Deserialize;
use serde::Serialize;
use typeshare::typeshare;

use crate::renderer_c::HapticRendererEmphasisFrequencySettings;
use crate::renderer_c::HapticRendererEmphasisOscillatorSettings;
use crate::renderer_c::HapticRendererEmphasisShape;
use crate::renderer_c::HapticRendererEmphasisShape_HAPTIC_RENDERER_EMPHASIS_SHAPE_SAW;
use crate::renderer_c::HapticRendererEmphasisShape_HAPTIC_RENDERER_EMPHASIS_SHAPE_SINE;
use crate::renderer_c::HapticRendererEmphasisShape_HAPTIC_RENDERER_EMPHASIS_SHAPE_SQUARE;
use crate::renderer_c::HapticRendererEmphasisShape_HAPTIC_RENDERER_EMPHASIS_SHAPE_TRIANGLE;

/// The settings used by the haptic renderer's EmphasisOscillator
///
/// See [crate::Acf].
#[derive(Copy, Clone, Default, Deserialize, Serialize)]
#[typeshare]
pub struct EmphasisOscillatorSettings {
    /// The gain of the emphasis EmphasisOscillator
    ///
    /// Range: 0->1
    pub gain: f32,

    /// The amount that the emphasis signal should fade out while it's active.
    /// The emphasis event will start at maximum amplitude and then fade to a percentage of
    /// the starting amplitude over its duration.
    /// For example, a value of '100' means "fade to silence over the event's duration",
    /// while '0' means "don't fade out, stay at maximum amplitude".
    pub fade_out_percent: f32,

    /// The settings to use when the emphasis frequency is at its minimum
    pub frequency_min: EmphasisFrequencySettings,

    /// The settings to use when the emphasis frequency is at its maximum
    pub frequency_max: EmphasisFrequencySettings,
}

impl From<EmphasisOscillatorSettings> for HapticRendererEmphasisOscillatorSettings {
    fn from(val: EmphasisOscillatorSettings) -> Self {
        HapticRendererEmphasisOscillatorSettings {
            gain: val.gain,
            fade_out_percent: val.fade_out_percent,
            frequency_min: val.frequency_min.into(),
            frequency_max: val.frequency_max.into(),
        }
    }
}

/// The settings to use for either minimum or maximum emphasis frequency
///
/// See [EmphasisOscillatorSettings].
#[derive(Copy, Clone, Default, Deserialize, Serialize)]
#[typeshare]
pub struct EmphasisFrequencySettings {
    /// The output frequency in hertz of the emphasis oscillator
    #[serde(default)]
    pub output_frequency: f32,

    /// The duration in milliseconds of the emphasis event
    #[serde(default)]
    pub duration_ms: f32,

    /// The shape of the emphasis oscillator's output.
    #[serde(default)]
    pub shape: EmphasisShape,
}

impl From<EmphasisFrequencySettings> for HapticRendererEmphasisFrequencySettings {
    fn from(val: EmphasisFrequencySettings) -> Self {
        HapticRendererEmphasisFrequencySettings {
            output_frequency: val.output_frequency,
            duration_ms: val.duration_ms,
            shape: val.shape.into(),
        }
    }
}

/// The oscillator shape to use when rendering an emphasis event
///
/// See [EmphasisFrequencySettings].
#[derive(Copy, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[typeshare]
pub enum EmphasisShape {
    /// A descending saw shape
    #[default]
    Saw,
    /// A Sine wave
    Sine,
    /// A rectangular wave starting with 'up'
    Square,
    /// A triangular wave, starting with the rising section
    Triangle,
}

impl From<EmphasisShape> for HapticRendererEmphasisShape {
    fn from(val: EmphasisShape) -> Self {
        match val {
            EmphasisShape::Saw => HapticRendererEmphasisShape_HAPTIC_RENDERER_EMPHASIS_SHAPE_SAW,
            EmphasisShape::Sine => HapticRendererEmphasisShape_HAPTIC_RENDERER_EMPHASIS_SHAPE_SINE,
            EmphasisShape::Square => {
                HapticRendererEmphasisShape_HAPTIC_RENDERER_EMPHASIS_SHAPE_SQUARE
            }
            EmphasisShape::Triangle => {
                HapticRendererEmphasisShape_HAPTIC_RENDERER_EMPHASIS_SHAPE_TRIANGLE
            }
        }
    }
}
