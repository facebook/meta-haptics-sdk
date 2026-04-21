// Copyright (c) Meta Platforms, Inc. and affiliates.
//
// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

//! # Emphasis detection
//!
//! This module provides an algorithm for assigning 'emphasis' to amplitude breakpoints.

use haptic_dsp::lerp;
use serde::Deserialize;
use serde::Serialize;
use typeshare::typeshare;

use crate::amplitude_analyzer::AmplitudeEvent;
use crate::spectrum_analysis::SpectralFeatures;

/// The properties describing emphasis
///
/// Currently an `Emphasis` struct gets attached to an `AmplitudeEvent` when the amplitude event
/// has been selected by `assign_emphasis_events`.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Emphasis {
    pub intensity: f32,
    pub sharpness: f32,
}

/// The settings used when assigning emphasis events
///
/// See [crate::OfflineAnalysisSettings].
#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
#[typeshare]
pub struct EmphasisAnalysisSettings {
    /// A sensitivity control that dials in the number of events that will have emphasis assigned.
    ///
    /// Peaks are found, and 'emphasis scores' are assigned to them.
    /// The min and max scores are then used to define the range for the sensitivity threshold.
    ///
    ///   0% -> No emphasis events
    ///   100% -> All emphasis events
    ///
    /// Between 0% and 100%, events will be selected that have an emphasis score above the
    /// sensitivity threshold.
    pub sensitivity_percent: f32,
    /// The minimum amount of time that must elapse before another peak can be selected.
    pub peak_window_ms: f32,
    /// The minimum rise in amplitude that must have occurred for an event to be selected as a peak.
    pub minimum_rise: f32,
    /// The amount that a rise can continue after an event for the event to be selected as a peak.
    /// This is expressed as a percentage of the previous rise in amplitude,
    /// e.g. with a plateau_factor of 50%:
    ///      0.1 -> 0.5 # Rise of 0.4
    ///                 # 50% of 0.4 would allow a rise up to 0.7 (0.5 + 0.2)
    ///      0.5 -> 0.6 # 0.6 is less than 0.7, so the event at 0.5 is selected as a peak
    pub plateau_factor_percent: f32,
    /// The minimum sharpness value to be used when assigning emphasis events.
    pub sharpness_min: f32,
    /// The maximum sharpness value to be used when assigning emphasis events.
    pub sharpness_max: f32,
    /// The amount that an amplitude event's amplitude will be reduced by to compensate for the
    /// addition of the emphasis at that time.
    pub ducking_percent: f32,
}

impl Default for EmphasisAnalysisSettings {
    fn default() -> Self {
        Self {
            sensitivity_percent: 50.0,
            peak_window_ms: 100.0,
            minimum_rise: 0.1,
            plateau_factor_percent: 25.0,
            sharpness_min: 0.0,
            sharpness_max: 1.0,
            ducking_percent: 50.0,
        }
    }
}

/// Assigns emphasis to amplitude events
///
/// # Algorithm overview
///
/// 1. Analyze the audio input to produce a Spectral Flux feature curve.
/// 2. For each amplitude event that is determined to be a peak (see below),
///    assign an emphasis score by interpolating the Spectral Flux curve at the event time.
///    a. Keep track of the minimum and maximum emphasis scores assigned to the peak events,
///    producing an emphasis score range.
/// 3. Based on the emphasis score range, the sensitivity setting defines a minimum score threshold.
/// 4. Remove emphasis from peak events with scores below the threshold.
/// 5. Remaining events are adjusted so that:
///    a. the event's emphasis score is mapped to the range defined by sharpness_min/sharpness_max.
///    b. the event's amplitude is reduced by the 'ducking' percentage.
///
/// ## Peak picking
///
/// An amplitude event is selected as a peak event if the following conditions are met:
/// - The 'amplitude rise' for the event (the difference between the previous event's amplitude and
///   the current event's amplitude) is above the 'minimum rise' setting.
/// - The following event's amplitude is less than the current amplitude, or less than the current
///   amplitude plus the amplitude rise (with the rise scaled by the 'plateau factor').
/// - The current event's time is after the previous peak event's time plus the 'peak window',
///   which limits how close peaks can appear together in the signal.
pub fn assign_emphasis_events(
    amplitude_events: &mut [AmplitudeEvent],
    spectral_features: &[SpectralFeatures],
    settings: &EmphasisAnalysisSettings,
) {
    let peak_window = settings.peak_window_ms / 1000.0;
    let plateau_factor = settings.plateau_factor_percent / 100.0;

    // For each peak amplitude point, interpolate SF and assign emphasis
    let mut sf_min = None;
    let mut sf_max = None;
    let mut previous_amplitude = 0.0;
    let mut last_peak_time = None;
    for i in 0..amplitude_events.len() - 1 {
        let current_event = amplitude_events[i];
        let next_event = amplitude_events[i + 1];

        let amplitude_rise = current_event.amplitude - previous_amplitude;
        let is_peak_event = amplitude_rise > settings.minimum_rise
            && (current_event.amplitude > next_event.amplitude
                || (next_event.amplitude
                    < current_event.amplitude + amplitude_rise * plateau_factor))
            && (last_peak_time.is_none()
                || (current_event.time - last_peak_time.unwrap()) >= peak_window);

        if is_peak_event {
            let sf = interpolated_spectral_flux(spectral_features, current_event.time);

            amplitude_events[i].emphasis = Some(Emphasis {
                intensity: current_event.amplitude,
                sharpness: sf,
            });

            sf_min = Some(sf_min.unwrap_or(sf).min(sf));
            sf_max = Some(sf_max.unwrap_or(sf).max(sf));
            last_peak_time = Some(current_event.time);
        }

        previous_amplitude = current_event.amplitude;
    }

    if sf_min.is_none() {
        // No peak events found, no more work to do
        return;
    }

    let sf_min = sf_min.expect("Missing sf_min");
    let sf_max = sf_max.expect("Missing sf_max");
    let sf_range = sf_max - sf_min;

    let sensitivity = settings.sensitivity_percent / 100.0;

    let minimum_spectral_flux = lerp(
        // A sensitivity of 0% sets the minimum sf to slightly above the maximum sf found in the
        // signal, which results in no emphasis events being included in the output.
        sf_max + sf_range * 1.0e-3,
        // A sensitivity of 100% will set the minimum sf to the lowest sf found in the signal,
        // which result in all emphasis events being included in the output.
        sf_min,
        sensitivity,
    );

    let amplitude_ducking_factor = 1.0 - settings.ducking_percent / 100.0;

    // Filter or scale emphasis points
    for event in amplitude_events.iter_mut() {
        if event.emphasis.is_some() {
            let sharpness = event.emphasis.unwrap().sharpness;
            if sharpness >= minimum_spectral_flux {
                // Scale the sharpness to 0->1...
                let relative_sharpness = if sf_range > 0.0 {
                    (sharpness - sf_min) / sf_range
                } else {
                    0.5
                };
                // ...and then map 0->1 to the min/max sharpness settings
                let scaled_sharpness = lerp(
                    settings.sharpness_min,
                    settings.sharpness_max,
                    relative_sharpness,
                );
                event.emphasis.as_mut().unwrap().sharpness = scaled_sharpness;
                // ...and scale the event amplitude by the ducking factor
                event.amplitude *= amplitude_ducking_factor;
            } else {
                // The emphasis sharpness is below the threshold, so remove the emphasis.
                event.emphasis = None;
            }
        }
    }
}

// Finds the interpolated spectral flux value for a given time
fn interpolated_spectral_flux(events: &[SpectralFeatures], time: f32) -> f32 {
    let partition_index = events.partition_point(|event| event.time <= time);

    if partition_index == 0 {
        events.first().unwrap().flux
    } else if partition_index == events.len() {
        events.last().unwrap().flux
    } else {
        let before = events[partition_index - 1];
        let after = events[partition_index];

        let time_factor = (time - before.time) / (after.time - before.time);
        lerp(before.flux, after.flux, time_factor)
    }
}

#[cfg(test)]
mod tests {
    use assert_approx_eq::assert_approx_eq;

    use super::*;

    #[allow(clippy::type_complexity)]
    fn run_emphasis_detection_test(
        input: &[(f32, f32)],
        spectral_flux: &[(f32, f32)],
        expected: &[(f32, f32, Option<(f32, f32)>)],
        settings: &EmphasisAnalysisSettings,
    ) {
        assert_eq!(input.len(), expected.len());

        let mut events: Vec<AmplitudeEvent> = input
            .iter()
            .cloned()
            .map(|(time, amplitude)| AmplitudeEvent {
                time,
                amplitude,
                emphasis: None,
            })
            .collect();

        let spectral_flux: Vec<SpectralFeatures> = spectral_flux
            .iter()
            .cloned()
            .map(|(time, flux)| SpectralFeatures {
                time,
                centroid: 0.0,
                flux,
            })
            .collect();

        let expected_events: Vec<AmplitudeEvent> = expected
            .iter()
            .cloned()
            .map(|(time, amplitude, emphasis)| AmplitudeEvent {
                time,
                amplitude,
                emphasis: emphasis.map(|(intensity, sharpness)| Emphasis {
                    intensity,
                    sharpness,
                }),
            })
            .collect();

        assign_emphasis_events(&mut events, &spectral_flux, settings);

        let allowed_error = 1.0e-5;
        for (expected_event, actual_event) in expected_events.iter().zip(events.iter()) {
            assert_approx_eq!(expected_event.time, actual_event.time, allowed_error);
            assert_approx_eq!(
                expected_event.amplitude,
                actual_event.amplitude,
                allowed_error
            );
            if let Some(expected_emphasis) = expected_event.emphasis {
                let actual_emphasis = actual_event.emphasis.unwrap();
                assert_approx_eq!(
                    expected_emphasis.intensity,
                    actual_emphasis.intensity,
                    allowed_error
                );
                assert_approx_eq!(
                    expected_emphasis.sharpness,
                    actual_emphasis.sharpness,
                    allowed_error
                );
            } else {
                assert!(actual_event.emphasis.is_none());
            }
        }
    }

    #[test]
    fn test_emphasis_at_first_event() {
        let settings = EmphasisAnalysisSettings::default();

        let input = vec![(0.0e-3, 0.8), (1.0e-3, 0.0)];
        let spectral_flux = vec![(0.0, 1.0)];

        // We expect the first event to be flagged as an emphasis point, with sharpness at 0.5.
        let expected = vec![(0.0e-3, 0.4, Some((0.8, 0.5))), (1.0e-3, 0.0, None)];

        run_emphasis_detection_test(&input, &spectral_flux, &expected, &settings);
    }

    #[test]
    fn test_emphasis_at_second_event() {
        let settings = EmphasisAnalysisSettings {
            minimum_rise: 0.1,
            peak_window_ms: 100.0,
            ducking_percent: 50.0,
            ..Default::default()
        };

        let min_amp = settings.minimum_rise;
        let input = vec![
            // The first event has low amplitude, not enough to be an emphasis point
            (0.0e-3, min_amp - 1.0e-3),
            // The second event rises by just above the minimum rise and is higher than the
            // following event, so this gets flagged as emphasis.
            (1.0e-3, min_amp * 2.0),
            // The third event decreases in amplitude.
            (2.0e-3, 0.0),
            // The fourth event rises more than the second, and is a peak,
            // but is within the peak window following the previous event,
            // so doesn't get flagged as emphasis.
            (3.0e-3, min_amp * 3.0),
            // The third event decreases in amplitude.
            (4.0e-3, 0.0),
        ];
        let spectral_flux = vec![(0.0, 1.0)];

        let expected = vec![
            (0.0e-3, min_amp - 1.0e-3, None),
            // emphasis detected on the second event, with 50% ducking applied
            (1.0e-3, min_amp, Some((min_amp * 2.0, 0.5))),
            (2.0e-3, 0.0, None),
            (3.0e-3, min_amp * 3.0, None),
            (4.0e-3, 0.0, None),
        ];

        run_emphasis_detection_test(&input, &spectral_flux, &expected, &settings);
    }

    #[test]
    fn test_emphasis_at_second_event_using_plateau_factor() {
        let settings = EmphasisAnalysisSettings {
            minimum_rise: 0.1,
            peak_window_ms: 0.0,
            ducking_percent: 75.0,
            plateau_factor_percent: 25.0,
            ..Default::default()
        };

        let min_amp = settings.minimum_rise;
        let input = vec![
            // The first event has low amplitude, not enough to be an emphasis point
            (0.0e-3, min_amp - 1.0e-3),
            // The second event rises by just above the minimum rise and the following event's
            // amplitude is within the plateau factor, so this gets flagged as emphasis.
            (1.0e-3, min_amp * 2.0),
            // The third event increases in ampltiude, but within the plateau factor
            (2.0e-3, min_amp * 2.1),
            // The fourth event falls to zero amplitude, allowing for  following rise
            (3.0e-3, 0.0),
            // The fifth event rises more than the second, but the following event's amplitude
            // is is higher than allowed for by the plateau factor,
            // so doesn't get flagged as emphasis.
            (4.0e-3, min_amp * 3.0),
            // The sixth event increases in amplitude, more than allowed for by the plateau factor.
            (5.0e-3, min_amp * 6.0),
        ];
        let spectral_flux = vec![(0.0, 1.0)];

        let expected = vec![
            (0.0e-3, min_amp - 1.0e-3, None),
            // emphasis detected on the second event, with 75% ducking applied
            (1.0e-3, min_amp * 0.5, Some((min_amp * 2.0, 0.5))),
            (2.0e-3, min_amp * 2.1, None),
            (3.0e-3, 0.0, None),
            (4.0e-3, min_amp * 3.0, None),
            (5.0e-3, min_amp * 6.0, None),
        ];

        run_emphasis_detection_test(&input, &spectral_flux, &expected, &settings);
    }

    #[test]
    fn test_two_emphasis_events() {
        let sharpness_min = 0.25;
        let sharpness_max = 0.75;

        let settings = EmphasisAnalysisSettings {
            sensitivity_percent: 100.0,
            minimum_rise: 0.1,
            peak_window_ms: 0.0,
            ducking_percent: 50.0,
            sharpness_min,
            sharpness_max,
            plateau_factor_percent: 25.0,
        };

        let min_amp = settings.minimum_rise;
        let input = vec![
            // The first event gets flagged as emphasis.
            (0.0e-3, min_amp * 2.0),
            (1.0e-3, 0.0),
            // The third event gets flagged as emphasis.
            (2.0e-3, min_amp * 2.0),
            (3.0e-3, 0.0),
        ];
        let spectral_flux = vec![
            // Spectral flux starting at 1, and ending at zero.
            // This will cause the first peak to have higher sharpness than the second.
            (0.0, 1.0),
            (1.0, 0.0),
        ];

        let expected = vec![
            // Emphasis detected on the first event, with 50% ducking applied.
            // The sharpness is set to sharpness_max due to higher spectral flux.
            (0.0e-3, min_amp, Some((min_amp * 2.0, sharpness_max))),
            (1.0e-3, 0.0, None),
            // Emphasis detected on the third event, with 50% ducking applied.
            // The sharpness is set to sharpness_min due to lower spectral flux.
            (2.0e-3, min_amp, Some((min_amp * 2.0, sharpness_min))),
            (3.0e-3, 0.0, None),
        ];

        run_emphasis_detection_test(&input, &spectral_flux, &expected, &settings);
    }

    #[test]
    fn test_emphasis_events_filtered_by_sensitivity() {
        let sharpness_min = 0.2;
        let sharpness_max = 0.8;

        let settings = EmphasisAnalysisSettings {
            sensitivity_percent: 90.0,
            minimum_rise: 0.1,
            peak_window_ms: 0.0,
            ducking_percent: 50.0,
            sharpness_min,
            sharpness_max,
            plateau_factor_percent: 25.0,
        };

        let min_amp = settings.minimum_rise;
        let input = vec![
            // The first event gets flagged as emphasis.
            (0.0e-3, min_amp * 2.0),
            (1.0e-3, 0.0),
            // The third event gets flagged as emphasis.
            (2.0e-3, min_amp * 2.0),
            (3.0e-3, 0.0),
            // The fifth event gets flagged as emphasis.
            (4.0e-3, min_amp * 2.0),
            (5.0e-3, 0.0),
        ];
        let spectral_flux = vec![
            // Spectral flux starting at 1, and ending at zero.
            // This will cause the emphasis sharpness values to decrease through the sequence.
            (0.0, 1.0),
            (1.0, 0.0),
        ];

        let expected = vec![
            // Emphasis detected on the first event, with 50% ducking applied.
            // The sharpness is set to sharpness_max due to higher spectral flux.
            (0.0e-3, min_amp, Some((min_amp * 2.0, sharpness_max))),
            (1.0e-3, 0.0, None),
            // Emphasis detected on the third event, with 50% ducking applied.
            // The sharpness is mid-way through the sharpness range due to lower spectral flux.
            (
                2.0e-3,
                min_amp,
                Some((min_amp * 2.0, lerp(sharpness_min, sharpness_max, 0.5))),
            ),
            (3.0e-3, 0.0, None),
            // Emphasis was detected on the third event, but it was ignored due to the sensitivity
            // setting.
            (4.0e-3, min_amp * 2.0, None),
            (5.0e-3, 0.0, None),
        ];

        run_emphasis_detection_test(&input, &spectral_flux, &expected, &settings);
    }
}
