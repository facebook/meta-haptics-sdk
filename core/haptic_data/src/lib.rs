// Copyright (c) Meta Platforms, Inc. and affiliates.
//
// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

//! The home of the HapticsSDK data format

#![deny(missing_docs)]
#![deny(warnings)]

mod breakpoint;
mod json;
mod parametric;
pub mod test_utils;
/// Version 1 of the `HapticData` format
pub mod v1;
mod version;

#[cfg(feature = "ahap")]
pub mod ahap;

pub use breakpoint::BasicBreakpoint;
pub use breakpoint::Breakpoint;
pub use breakpoint::interpolate_breakpoints;
pub use json::FromJsonError;
pub use json::from_json;
pub use parametric::Clip;
pub use parametric::FromParametricError;
pub use parametric::Point;
pub use parametric::Transient;
pub use parametric::from_parametric;
pub use v1::HapticData;
pub use version::Version;
