// (c) Meta Platforms, Inc. and affiliates. Confidential and proprietary.

use haptic_dsp::Accumulator;
use haptic_dsp::db_to_amplitude;
use serde::Deserialize;
use serde::Serialize;
use typeshare::typeshare;

/// The settings used by [`preprocess_audio`]
#[derive(Debug, Default, Copy, Clone, Deserialize, Serialize)]
#[typeshare]
pub struct PreprocessingSettings {
    /// Gain that should be applied to the signal (ignored when normalize_audio is set to true)
    pub gain_db: f32,
    /// Normalizes the signal to the level specified by normalize_level_db
    pub normalize_audio: bool,
    /// The level to normalize the signal to (ignored when normalize_audio is set to false)
    pub normalize_level_db: f32,
}

/// Removes DC offset from the signal and applies gain or normalization
pub fn preprocess_audio(data: &mut [f32], settings: PreprocessingSettings, verbose: bool) {
    let (mean, peak_amp) = {
        let mut accumulator = Accumulator::default();
        let mut peak = 0.0;
        for sample in data.iter() {
            // Accumulate all sample values
            accumulator.process(*sample);
            // Find the peak sample value
            peak = sample.abs().max(peak);
        }
        let mean = accumulator.sum() / data.len() as f32;
        (mean, peak)
    };

    if verbose {
        println!("\n{settings:#?}");
        println!("Preprocessing audio - mean: {mean:.5}, peak: {peak_amp:.5}");
    }

    let normalize_factor = if settings.normalize_audio {
        let target_amp = db_to_amplitude(settings.normalize_level_db);
        let factor = target_amp / (peak_amp - mean);

        if verbose {
            println!("Normalization factor: {factor}");
        }

        factor
    } else {
        db_to_amplitude(settings.gain_db)
    };

    for sample in data.iter_mut() {
        // Remove the signal's overall DC offset
        *sample -= mean;
        // Apply normalization gain
        *sample *= normalize_factor;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check_signals_near(a: &[f32], b: &[f32], allowed_difference: f32) {
        for (i, (sample_a, sample_b)) in a.iter().zip(b.iter()).enumerate() {
            let difference = (*sample_a - *sample_b).abs();
            if difference > allowed_difference {
                panic!(
                    "check_signals_near: Mismatch at position {i} - '{}' and '{}' have a \
                       difference of '{difference}', which is greater than the allowed difference of '{allowed_difference}'",
                    *sample_a, *sample_b,
                );
            }
        }
    }

    #[test]
    fn normalize_to_0db() {
        let settings = PreprocessingSettings {
            normalize_audio: true,
            normalize_level_db: 0.0,
            ..Default::default()
        };

        let mut signal = [-0.5, -0.25, 0.0, 0.25, 0.5];
        let expected = [-1.0, -0.5, 0.0, 0.5, 1.0];

        preprocess_audio(&mut signal, settings, false);
        check_signals_near(&signal, &expected, 1.0e-6);
    }

    #[test]
    fn normalize_to_minus_6db() {
        let settings = PreprocessingSettings {
            normalize_audio: true,
            normalize_level_db: -6.0,
            ..Default::default()
        };

        let mut signal = [-1.0, -0.5, 0.0, 0.5, 1.0];
        let expected = [-0.5, -0.25, 0.0, 0.25, 0.5];

        preprocess_audio(&mut signal, settings, false);
        check_signals_near(&signal, &expected, 2.0e-3);
    }

    #[test]
    fn normalize_to_minus_6db_with_dc_offset() {
        let settings = PreprocessingSettings {
            normalize_audio: true,
            normalize_level_db: -6.0,
            ..Default::default()
        };

        let mut signal = [0.0, 0.25, 0.5, 0.75, 1.0];
        let expected = [-0.5, -0.25, 0.0, 0.25, 0.5];

        preprocess_audio(&mut signal, settings, false);
        check_signals_near(&signal, &expected, 2.0e-3);
    }

    #[test]
    fn unnormalized_with_plus_6db_gain() {
        let settings = PreprocessingSettings {
            gain_db: 6.0,
            normalize_audio: false,
            ..Default::default()
        };

        let mut signal = [-1.0, -0.5, 0.0, 0.5, 1.0];
        let expected = [-2.0, -1.0, 0.0, 1.0, 2.0];

        preprocess_audio(&mut signal, settings, false);
        check_signals_near(&signal, &expected, 5.0e-3);
    }

    #[test]
    fn unnormalized_with_minus_6db_gain_and_with_dc_offset() {
        let settings = PreprocessingSettings {
            gain_db: -6.0,
            normalize_audio: false,
            ..Default::default()
        };

        let mut signal = [0.0, 0.25, 0.5, 0.75, 1.0];
        let expected = [-0.25, -0.125, 0.0, 0.125, 0.25];

        preprocess_audio(&mut signal, settings, false);
        check_signals_near(&signal, &expected, 2.0e-3);
    }
}
