// Copyright (c) Meta Platforms, Inc. and affiliates.
//
// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

use std::sync::Arc;

use itertools::izip;
use realfft::RealFftPlanner;
use realfft::RealToComplex;

use super::Window;
use super::make_window;
use crate::Complex;
use crate::FixedDelayLine;

/// The settings used by [SpectralAnalyzer]
#[derive(Clone, Copy)]
pub struct SpectralAnalyzerSettings {
    /// The FFT size to be used during analysis
    pub fft_size: usize,
    /// The overlap factor to be used
    pub overlap_factor: usize,
    /// The analysis window to be applied for each frame
    pub window: Window,
}

/// A spectral analyzer for mono audio signals
///
/// The analyzer can be used in realtime or non-realtime contexts, allocations are performed on
/// initialization and then buffers are re-used during processing.
///
/// Input samples are passed into .process(), and then when enough samples have been passed in to
/// produce a new frame (taking into account the overlap factor), analysis is performed and the
/// analyzed frame is returned.
///
/// See [SpectralAnalyzerSettings]
pub struct SpectralAnalyzer {
    fft: Arc<dyn RealToComplex<f32>>,
    input_buffer: FixedDelayLine,
    window: Vec<f32>,
    fft_buffer_real: Vec<f32>,
    fft_buffer_complex: Vec<Complex<f32>>,
    hop_size: usize,
    samples_since_last_analysis: usize,
}

impl SpectralAnalyzer {
    /// Initialize an analyzer with the provided settings
    pub fn new(settings: SpectralAnalyzerSettings) -> Self {
        let fft_size = settings.fft_size;
        let mut fft_planner = RealFftPlanner::new();
        let fft = fft_planner.plan_fft_forward(fft_size);
        let fft_buffer_real = fft.make_input_vec();
        let fft_buffer_complex = fft.make_output_vec();

        Self {
            fft,
            window: make_window(settings.window, fft_size),
            input_buffer: FixedDelayLine::with_fixed_length(fft_size),
            fft_buffer_real,
            fft_buffer_complex,
            hop_size: fft_size / settings.overlap_factor,
            samples_since_last_analysis: 0,
        }
    }

    /// Process a single sample of input, optionally returning a spectrum frame
    ///
    /// The return frame is only present when enough input samples have been received to analyze a
    /// new frame. This will happen on each 'hop', i.e. the FFT size divided by the overlap factor.
    ///
    /// The returned spectrum frame has size (N/2 + 1); i.e. bins above Nyquist aren't included.
    pub fn process(&mut self, input: f32) -> Option<&mut [Complex<f32>]> {
        self.input_buffer.process(input);
        self.samples_since_last_analysis += 1;

        if self.samples_since_last_analysis == self.hop_size {
            for (analysis_input_sample, input_sample, window_sample) in izip!(
                self.fft_buffer_real.iter_mut(),
                self.input_buffer.iter(),
                self.window.iter()
            ) {
                *analysis_input_sample = input_sample * window_sample;
            }

            self.fft
                .process(&mut self.fft_buffer_real, &mut self.fft_buffer_complex)
                .ok();

            self.samples_since_last_analysis = 0;
            Some(&mut self.fft_buffer_complex)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use assert_approx_eq::assert_approx_eq;

    use super::*;
    use crate::test_utils::make_sine;

    fn test_fft(settings: SpectralAnalyzerSettings, input: &[f32], expected_output: &[(f32, f32)]) {
        assert_eq!(
            input.len(),
            settings.fft_size,
            "Input size and FFT size should match"
        );

        let mut analyzer = SpectralAnalyzer::new(settings);

        for input_sample in input.iter().take(settings.fft_size - 1) {
            let result = analyzer.process(*input_sample);
            assert_eq!(
                result, None,
                "The analyzer should only return a result after being called {} steps",
                settings.fft_size
            );
        }

        let analysis_result = analyzer
            .process(*input.last().unwrap())
            .expect("Missing analyzer output");

        assert_eq!(
            analysis_result.len(),
            expected_output.len(),
            "Output doesn't match FFT size"
        );

        for (output, (expected_real, expected_imaginary)) in
            analysis_result.iter().zip(expected_output.iter())
        {
            let allowed_error = 1.0e-6;
            assert_approx_eq!(output.re, *expected_real, allowed_error);
            assert_approx_eq!(output.im, *expected_imaginary, allowed_error);
        }
    }

    #[test]
    fn ones_as_input_rectangular() {
        test_fft(
            SpectralAnalyzerSettings {
                fft_size: 4,
                overlap_factor: 1,
                window: Window::Rectangular,
            },
            &[1.0; 4],
            &[(4.0, 0.0), (0.0, 0.0), (0.0, 0.0)],
        )
    }

    #[test]
    fn ones_as_input_hanning() {
        test_fft(
            SpectralAnalyzerSettings {
                fft_size: 8,
                overlap_factor: 1,
                window: Window::Hanning,
            },
            &[1.0; 8],
            &[(4.0, 0.0), (-2.0, 0.0), (0.0, 0.0), (0.0, 0.0), (0.0, 0.0)],
        )
    }

    #[test]
    fn ones_as_input_hanningz() {
        test_fft(
            SpectralAnalyzerSettings {
                fft_size: 8,
                overlap_factor: 1,
                window: Window::HanningZ,
            },
            &[1.0; 8],
            &[
                (3.5, 0.0),
                (-1.9216883, -0.7959895),
                (0.15096891, 0.15096885),
                (0.020719588, 0.0500215),
                (0.0, 0.0),
            ],
        )
    }

    #[test]
    fn sine_as_input_rectangular() {
        test_fft(
            SpectralAnalyzerSettings {
                fft_size: 8,
                overlap_factor: 1,
                window: Window::Rectangular,
            },
            &make_sine(8.0, 1.0, 8),
            &[(0.0, 0.0), (0.0, -4.0), (0.0, 0.0), (0.0, 0.0), (0.0, 0.0)],
        )
    }

    #[test]
    fn double_sine_as_input_rectangular() {
        test_fft(
            SpectralAnalyzerSettings {
                fft_size: 8,
                overlap_factor: 1,
                window: Window::Rectangular,
            },
            &make_sine(8.0, 2.0, 8),
            &[(0.0, 0.0), (0.0, 0.0), (0.0, -4.0), (0.0, 0.0), (0.0, 0.0)],
        )
    }
}
