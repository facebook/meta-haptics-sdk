// Copyright (c) Meta Platforms, Inc. and affiliates.
//
// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

use thiserror::Error;

use crate::StreamingEvent;

/// The Result type returned by `haptic_renderer` functions
pub type Result<T> = std::result::Result<T, Error>;

/// The error type used by `haptic_renderer`
#[derive(Error, Debug)]
#[allow(missing_docs)]
pub enum Error {
    #[error("attempting to render invalid haptic data: {0}")]
    InvalidHapticData(String),

    #[error("an output error occurred: {0}")]
    IoError(std::io::Error),

    #[error("rendering to Wav requires the 'wav' feature")]
    WavRenderingNotEnabled,

    #[cfg(feature = "wav")]
    #[error("an error occurred while rendering to Wav data: {0}")]
    WavWriteError(hound::Error),

    #[error(
        "Attempting to split a ramp outside of its bounds:
  split time: {split_time}
  ramp: {ramp:?}"
    )]
    SplitTimeOutOfBounds {
        split_time: f32,
        ramp: StreamingEvent,
    },

    #[error(
        "Attempting to split an emphasis event
  event: {0:?}"
    )]
    CantSplitAnEmphasisEvent(StreamingEvent),
}
