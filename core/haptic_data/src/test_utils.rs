// (c) Meta Platforms, Inc. and affiliates. Confidential and proprietary.

//! Utilities for writing tests that need haptic data

use std::path::Path;

use crate::v1::*;

/// A helper for easily defining test haptic data
#[derive(Copy, Clone)]
pub struct TestClip<'a> {
    /// The test clip's amplitude breakpoints
    pub amplitude: &'a [AmplitudeBreakpoint],
    /// The test clip's frequency breakpoints
    pub frequency: &'a [FrequencyBreakpoint],
}

impl<'a> From<TestClip<'a>> for HapticData {
    fn from(test_clip: TestClip) -> Self {
        Self {
            signals: Signals {
                continuous: SignalContinuous {
                    envelopes: Envelopes {
                        amplitude: test_clip.amplitude.to_vec(),
                        frequency: if test_clip.frequency.is_empty() {
                            None
                        } else {
                            Some(test_clip.frequency.to_vec())
                        },
                    },
                },
            },
            ..Default::default()
        }
    }
}

/// A helper for making an amplitude breakpoint without emphasis
pub fn amp_bp(time: f32, amplitude: f32) -> AmplitudeBreakpoint {
    AmplitudeBreakpoint {
        time,
        amplitude,
        emphasis: None,
    }
}

/// A helper for making an amplitude breakpoint with emphasis
pub fn emphasis_bp(
    time: f32,
    amplitude: f32,
    emphasis_amp: f32,
    emphasis_freq: f32,
) -> AmplitudeBreakpoint {
    AmplitudeBreakpoint {
        time,
        amplitude,
        emphasis: Some(Emphasis {
            amplitude: emphasis_amp,
            frequency: emphasis_freq,
        }),
    }
}

/// A helper for making a frequency breakpoint
pub fn freq_bp(time: f32, frequency: f32) -> FrequencyBreakpoint {
    FrequencyBreakpoint { time, frequency }
}

/// Reads a file from the test_files/ directory into a String
pub fn load_test_file_as_string(path: &str) -> String {
    let test_files_dir = match option_env!("CARGO_MANIFEST_DIR") {
        Some(dir) => dir.to_string(),
        None => std::env::var("TEST_FILES").unwrap(),
    };
    std::fs::read_to_string(Path::new(&test_files_dir).join("test_files").join(path)).unwrap()
}

/// Reads a file from the test_files/ directory into a HapticData struct
pub fn load_test_file_as_haptic_data(path: &str) -> HapticData {
    serde_json::from_str(&load_test_file_as_string(path)).unwrap()
}
