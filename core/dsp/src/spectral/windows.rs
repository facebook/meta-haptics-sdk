// (c) Meta Platforms, Inc. and affiliates. Confidential and proprietary.

use std::f32::consts::PI;

struct Rectangular {
    i: usize,
    size: usize,
}

impl Rectangular {
    fn new(size: usize) -> Self {
        Self { i: 0, size }
    }
}

impl Iterator for Rectangular {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.i < self.size {
            self.i += 1;
            Some(1.0)
        } else {
            None
        }
    }
}

struct Hanning {
    i: usize,
    size: usize,
    cos_factor: f32,
}

impl Hanning {
    fn new(size: usize) -> Self {
        Self {
            i: 0,
            size,
            cos_factor: (PI * 2.0) / (size as f32),
        }
    }
}

impl Iterator for Hanning {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.i < self.size {
            let result = 0.5 * (1.0 - (self.i as f32 * self.cos_factor).cos());
            self.i += 1;
            Some(result)
        } else {
            None
        }
    }
}

struct HanningZ {
    i: usize,
    size: usize,
    cos_factor: f32,
}

impl HanningZ {
    fn new(size: usize) -> Self {
        Self {
            i: 0,
            size,
            cos_factor: (PI * 2.0) / (size as f32 - 1.0),
        }
    }
}

impl Iterator for HanningZ {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.i < self.size {
            let result = 0.5 * (1.0 - (self.i as f32 * self.cos_factor).cos());
            self.i += 1;
            Some(result)
        } else {
            None
        }
    }
}

/// A collection of analysis window functions
///
/// See <https://en.wikipedia.org/wiki/Window_function> for orientation.
///
/// - See [crate::SpectralAnalyzer]
/// - See [make_window]
#[derive(Clone, Copy)]
pub enum Window {
    /// Useful in testing
    Rectangular,
    /// Useful in single frame analysis
    Hanning,
    /// Useful for continuous STFT phase-vocoder style analysis
    /// Essentially this is a Hanning window with periodic behaviour,
    /// see 'Traditional Implementations of a Phase Vocoder: the Tricks of the Trade',
    /// Götzen et al. 2000
    HanningZ,
}

/// Makes a window of the requested type and size
pub fn make_window(window_type: Window, size: usize) -> Vec<f32> {
    match window_type {
        Window::Rectangular => Rectangular::new(size).collect(),
        Window::Hanning => Hanning::new(size).collect(),
        Window::HanningZ => HanningZ::new(size).collect(),
    }
}
