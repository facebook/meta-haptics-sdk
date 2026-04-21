// (c) Meta Platforms, Inc. and affiliates. Confidential and proprietary.

// The napi-derive proc macro generates code without doc comments when building with the
// crates.io version. Allow missing_docs to accommodate this.
#![allow(missing_docs)]
#![deny(warnings)]

//! The Haptics SDK Node-API interface used by Haptics Studio

mod audio_analysis;
mod audio_decoding;
mod haptic_data;
mod haptic_renderer;
mod helpers;

pub use crate::audio_analysis::*;
pub use crate::audio_decoding::*;
pub use crate::haptic_data::*;
pub use crate::haptic_renderer::*;
