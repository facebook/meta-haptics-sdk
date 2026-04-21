// Copyright (c) Meta Platforms, Inc. and affiliates.
//
// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

use crate::Complex;
use crate::linspace;

/// Returns the normalized spectral centroid of the input spectrum
///
/// A half spectrum frame including Nyquist (i.e. N/2 + 1) is expected as input.
///
/// - See [crate::SpectralAnalyzer]
pub fn spectral_centroid(spectrum: &[Complex<f32>]) -> f32 {
    let spectrum_size = spectrum.len();

    let (weighted_sums, magnitudes) = spectrum.iter().zip(linspace(0.0, 1.0, spectrum_size)).fold(
        (0.0, 0.0),
        |(weighted_sums, magnitudes), (bin, frequency)| {
            let bin_magnitude = bin.norm();
            (
                weighted_sums + bin_magnitude * frequency,
                magnitudes + bin_magnitude,
            )
        },
    );

    if magnitudes != 0.0 {
        weighted_sums / magnitudes
    } else {
        0.0
    }
}

/// Returns the Spectral Flux value by comparing the magnitudes of two spectrum frames
///
/// For technical background see <https://www.eecs.qmul.ac.uk/~simond/pub/2006/dafx.pdf>
pub fn spectral_flux(magnitudes: &[f32], previous_magnitudes: &[f32]) -> f32 {
    debug_assert_eq!(magnitudes.len(), previous_magnitudes.len());

    magnitudes
        .iter()
        .zip(previous_magnitudes.iter())
        .fold(0.0, |result, (magnitude, previous_magnitude)| {
            result + (magnitude - previous_magnitude)
        })
        .max(0.0)
}

#[cfg(test)]
mod tests {
    use assert_approx_eq::assert_approx_eq;

    use super::*;
    use crate::spectral::SpectralAnalyzer;
    use crate::spectral::SpectralAnalyzerSettings;
    use crate::spectral::Window;
    use crate::test_utils::make_sine;

    fn test_centroid(input: &[f32], expected_centroid: f32) {
        let mut analysis_result = None;

        let mut analyzer = SpectralAnalyzer::new(SpectralAnalyzerSettings {
            fft_size: input.len(),
            overlap_factor: 1,
            window: Window::Rectangular,
        });

        for input_sample in input {
            analysis_result = analyzer
                .process(*input_sample)
                .map(|fft_buffer| spectral_centroid(fft_buffer))
        }

        let allowed_error = 1.0e-6;
        assert_approx_eq!(
            expected_centroid,
            analysis_result.expect("Missing analysis result"),
            allowed_error
        );
    }

    #[test]
    fn centroid_silence() {
        test_centroid(&[0.0, 0.0, 0.0, 0.0], 0.0);
    }

    #[test]
    fn centroid_sine() {
        test_centroid(&make_sine(8.0, 1.0, 8), 0.25);
    }

    #[test]
    fn centroid_sine_double() {
        test_centroid(&make_sine(8.0, 2.0, 8), 0.5);
    }

    #[test]
    fn test_spectral_flux() {
        let previous_magnitudes = [1.0, 2.0, 3.0, 4.0];
        let current_magnitudes = [11.0, 22.0, 33.0, 44.0];

        let allowed_error = 1.0e-7;
        assert_approx_eq!(
            spectral_flux(&current_magnitudes, &previous_magnitudes),
            10.0 + 20.0 + 30.0 + 40.0,
            allowed_error
        );

        // Swapping the previous and current magnitudes should result in a spectral flux of zero,
        // due to negative outputs being clamped.
        assert_approx_eq!(
            spectral_flux(&previous_magnitudes, &current_magnitudes),
            0.0,
            allowed_error
        );
    }
}
