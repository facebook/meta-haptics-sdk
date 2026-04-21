// Copyright (c) Meta Platforms, Inc. and affiliates.
//
// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

//! # haptic_dsp
//!
//! Low-level DSP components used in HapticsSDK.
//!
//! ## Coding conventions
//!
//! * Time values are expressed in seconds, unless otherwise specified.
//!   * e.g. `delay_time` is in seconds, `delay_time_in_ms` would be an exception.

#![deny(missing_docs)]
#![deny(warnings)]

mod accumulator;
mod conversion;
mod delay;
mod envelope_follower;
mod float;
mod interpolation;
mod linspace;
mod rms;
#[cfg(feature = "spectral")]
mod spectral;
pub mod test_utils;

pub use accumulator::Accumulator;
pub use conversion::db_to_amplitude;
pub use delay::FixedDelayLine;
pub use envelope_follower::EnvelopeFollower;
pub use envelope_follower::RmsEnvelopeFollower;
pub use float::flush_f32_to_zero;
pub use interpolation::lerp;
pub use linspace::Linspace;
pub use linspace::linspace;
pub use rms::WindowedMovingRms;
#[cfg(feature = "spectral")]
pub use spectral::*;
