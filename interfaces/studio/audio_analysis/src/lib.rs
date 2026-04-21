// (c) Meta Platforms, Inc. and affiliates. Confidential and proprietary.

//! # haptic_audio_analysis
//!
//! Support for generating haptics by analyzing audio data
//!
//! ## Library Conventions
//!
//! Time values are expressed in seconds, unless otherwise specified.
//!   - e.g. `amplitude_hold_time` is in seconds, `amplitude_hold_time_in_ms` would
//!     be an exception.

#![deny(missing_docs)]
#![deny(warnings)]

mod amplitude_analyzer;
mod audio_loading;
mod audio_preprocessing;
mod breakpoint_reduction;
mod emphasis_detection;
mod frequency_analyzer;
mod offline_analysis;
mod spectrum_analysis;
mod visual_waveform;

pub use amplitude_analyzer::AmplitudeAnalysisSettings;
pub use amplitude_analyzer::AmplitudeAnalyzer;
pub use audio_loading::MultiChannelBehavior;
pub use audio_loading::load_audio_data;
pub use audio_preprocessing::PreprocessingSettings;
pub use audio_preprocessing::preprocess_audio;
pub use emphasis_detection::EmphasisAnalysisSettings;
pub use frequency_analyzer::FrequencyAnalysisSettings;
pub use offline_analysis::OfflineAnalysisSettings;
pub use offline_analysis::audio_to_haptics;
pub use spectrum_analysis::SpectrumAnalysisSettings;
pub use visual_waveform::TimeAmplitude;
pub use visual_waveform::VisualWaveform;
