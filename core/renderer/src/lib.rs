// (c) Meta Platforms, Inc. and affiliates. Confidential and proprietary.

//! # haptic_renderer
//!
//! Contains `HapticRenderer`, which can convert `HapticData` into a fixed-time series of values,
//! e.g. a PCM stream or an amplitude curve (see `RenderMode`).
//!
//! Support for rendering to .wav data is provided by the [Hound](https://crates.io/crates/hound)
//! crate. If .wav support isn't required then the dependency can be avoided by opting out of the
//! "wav" feature.

#![deny(missing_docs)]
#![deny(warnings)]

mod acf;
#[cfg(feature = "android")]
mod android_constant_intensity;
#[cfg(feature = "android")]
mod android_waveform;
mod continuous_oscillator_settings;
mod emphasis_oscillator_settings;
mod error;
mod haptic_file_renderer;
mod render_settings;
mod renderer_c;
mod streaming_event;
mod streaming_event_reader;
mod streaming_renderer;

pub mod test_utils;

pub use crate::acf::Acf;
#[cfg(feature = "android")]
pub use crate::android_constant_intensity::ConstantIntensityVibration;
#[cfg(feature = "android")]
pub use crate::android_constant_intensity::render_constant_intensity;
#[cfg(feature = "android")]
pub use crate::android_waveform::Waveform;
#[cfg(feature = "android")]
pub use crate::android_waveform::WaveformRenderSettings;
#[cfg(feature = "android")]
pub use crate::android_waveform::render_waveform;
pub use crate::continuous_oscillator_settings::ContinuousOscillatorSettings;
pub use crate::emphasis_oscillator_settings::EmphasisFrequencySettings;
pub use crate::emphasis_oscillator_settings::EmphasisOscillatorSettings;
pub use crate::emphasis_oscillator_settings::EmphasisShape;
pub use crate::error::Error;
pub use crate::error::Result;
pub use crate::haptic_file_renderer::HapticFileRenderer;
pub use crate::render_settings::OutputFormat;
pub use crate::render_settings::RenderMode;
pub use crate::render_settings::RenderSettings;
pub use crate::render_settings::SampleFormat;
pub use crate::streaming_event::StreamingEvent;
pub use crate::streaming_event::StreamingEventType;
pub use crate::streaming_event::StreamingRamp;
pub use crate::streaming_event_reader::StreamingEventReader;
pub use crate::streaming_renderer::StreamingRenderer;
