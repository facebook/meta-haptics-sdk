// (c) Meta Platforms, Inc. and affiliates. Confidential and proprietary.

use std::error::Error;

use haptic_data::v1::HapticData as HapticV1;
use haptic_data::v1::*;
use serde::Deserialize;
use serde::Serialize;
use typeshare::typeshare;

use crate::amplitude_analyzer::AmplitudeAnalysisSettings;
use crate::amplitude_analyzer::AmplitudeAnalyzer;
use crate::amplitude_analyzer::AmplitudeEvent;
use crate::breakpoint_reduction::ReduceBreakpointsSettings;
use crate::breakpoint_reduction::reduce_breakpoints;
use crate::emphasis_detection;
use crate::emphasis_detection::EmphasisAnalysisSettings;
use crate::emphasis_detection::assign_emphasis_events;
use crate::frequency_analyzer::FrequencyAnalysisSettings;
use crate::frequency_analyzer::FrequencyEvent;
use crate::spectrum_analysis::SpectralFeatures;
use crate::spectrum_analysis::SpectrumAnalysisSettings;
use crate::spectrum_analysis::SpectrumAnalyzer;

/// The settings passed in when calling [audio_to_haptics]
#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
#[typeshare]
pub struct OfflineAnalysisSettings {
    /// The settings used for spectrum analysis
    ///
    /// The extracted spectral features will be used by frequency analysis and emphasis detection.
    pub spectrum: SpectrumAnalysisSettings,
    /// The settings used for amplitude analysis
    pub amplitude: AmplitudeAnalysisSettings,
    /// The target breakpoints per second used during reduction of the amplitude envelope
    pub amplitude_breakpoints_per_second: u32,
    /// The settings used for frequency analysis
    pub frequency: FrequencyAnalysisSettings,
    /// The target breakpoints per second used during reduction of the frequency envelope
    pub frequency_breakpoints_per_second: u32,
    /// Whether or not emphasis should be attached to amplitude breakpoints
    pub emphasis_enabled: bool,
    /// The settings used for emphasis detection
    pub emphasis: EmphasisAnalysisSettings,
    /// The minimum value that breakpoints are allowed to have
    ///
    /// Breakpoints with values below this score will be filtered out during breakpoint reduction.
    pub minimum_breakpoint_score: f32,
}

impl Default for OfflineAnalysisSettings {
    fn default() -> Self {
        Self {
            spectrum: SpectrumAnalysisSettings::default(),
            amplitude: AmplitudeAnalysisSettings::default(),
            frequency: FrequencyAnalysisSettings::default(),
            amplitude_breakpoints_per_second: 60,
            frequency_breakpoints_per_second: 60,
            emphasis_enabled: true,
            emphasis: EmphasisAnalysisSettings::default(),
            minimum_breakpoint_score: 1.0e-8, // A small number to filter out almost-silent sections
        }
    }
}

/// Produces Haptic data by analyzing mono audio input
///
/// This is the entry point for audio to haptics analysis.
pub fn audio_to_haptics(
    input: &[f32],
    sample_rate: f32,
    settings: OfflineAnalysisSettings,
    validate_output: bool,
    verbose: bool,
) -> Result<HapticV1, Box<dyn Error>> {
    if verbose {
        println!("\n{settings:#?}");
    }

    let (mut amplitude_signal, mut frequency_signal) =
        do_audio_to_haptics_analysis(input, sample_rate, settings);

    // - The v1 format expects the duration to be defined by the amplitude signal's last event.
    // - Numerical imprecision during analysis may lead to a frequency event that's (very) slightly
    //   after the amplitude event in time.
    // - To sanitize this imprecision, make sure that both the frequency and amplitude signals
    //   have the same duration. To ensure equal duration of both signals, set the last event's
    //   time of both signals to the maximum duration of all signals (the input audio signal,
    //   amplitude signal, and frequency signal)
    let input_duration = input.len() as f32 / sample_rate;
    let last_frequency_time = frequency_signal.last().map_or(0.0, |event| event.time);
    let last_amplitude_time = amplitude_signal.last().map_or(0.0, |event| event.time);

    let max_duration = input_duration
        .max(last_frequency_time)
        .max(last_amplitude_time);

    // Adjust the amplitude envelope if it's shorter than the maximum duration
    if let Some(last_amplitude_event) = amplitude_signal.last().cloned() {
        if last_amplitude_event.time < max_duration {
            amplitude_signal.push(AmplitudeEvent {
                time: max_duration,
                amplitude: last_amplitude_event.amplitude,
                emphasis: None,
            });
        }
    }

    // Adjust the frequency envelope if it's shorter than the maximum duration
    if let Some(last_frequency_event) = frequency_signal.last().cloned() {
        if last_frequency_event.time < max_duration {
            frequency_signal.push(FrequencyEvent {
                time: max_duration,
                frequency: last_frequency_event.frequency,
            });
        }
    }

    let mut result = HapticV1::default();

    result.signals.continuous.envelopes.amplitude = amplitude_signal
        .iter()
        .map(|event| AmplitudeBreakpoint {
            time: event.time,
            amplitude: event.amplitude,
            emphasis: event.emphasis.map(
                |emphasis_detection::Emphasis {
                     intensity,
                     sharpness,
                 }| haptic_data::v1::Emphasis {
                    amplitude: intensity,
                    frequency: sharpness,
                },
            ),
        })
        .collect();

    result.signals.continuous.envelopes.frequency = Some(
        frequency_signal
            .iter()
            .map(|event| FrequencyBreakpoint {
                time: event.time,
                frequency: event.frequency,
            })
            .collect(),
    );

    if validate_output {
        result
            .validate(ValidationMode::Strict)
            .map_err(|e| e.into())
    } else {
        Ok(result)
    }
}

// The parallelized implementation of audio to haptics analysis
//
// See below for the single-threaded version.
#[cfg(feature = "multithreaded_analysis")]
fn do_audio_to_haptics_analysis(
    input: &[f32],
    sample_rate: f32,
    settings: OfflineAnalysisSettings,
) -> (Vec<AmplitudeEvent>, Vec<FrequencyEvent>) {
    // Use a thread scope to allow the input slice to be borrowed without lifetime issues
    std::thread::scope(|scope| {
        // First, spawn a thread that will perform the amplitude analysis.
        let amplitude_thread = scope.spawn(|| analyze_amplitude(input, sample_rate, &settings));

        // Next, perform the spectrum analysis on the current thread (all subsequent operations
        // depend on the result).
        let spectral_features = analyze_spectrum(input, sample_rate, &settings);

        // Now, create a new thread scope so that we can perform parallel operations on the
        // spectral features without running into lifetime issues.
        std::thread::scope(|scope| {
            // Spawn an additional thread for performing frequency analysis
            let frequency_thread =
                scope.spawn(|| analyze_frequency(&spectral_features, sample_rate, &settings));

            // The emphasis analysis depends on the amplitude signal,
            // so join the amplitude thread now.
            let mut amplitude_signal = amplitude_thread
                .join()
                .expect("Failed to join amplitude analysis thread");

            // Perform emphasis analysis on the current thread.
            analyze_emphasis(&mut amplitude_signal, &spectral_features, &settings);

            // Join the frequency thread to get the resulting frequency signal.
            let frequency_signal = frequency_thread
                .join()
                .expect("Failed to join frequency analysis thread");

            // Finished!
            (amplitude_signal, frequency_signal)
        })
    })
}

// The single-threaded implementation of audio to haptics analysis
//
// See above for the multi-threaded version.
#[cfg(not(feature = "multithreaded_analysis"))]
fn do_audio_to_haptics_analysis(
    input: &[f32],
    sample_rate: f32,
    settings: OfflineAnalysisSettings,
) -> (Vec<AmplitudeEvent>, Vec<FrequencyEvent>) {
    let mut amplitude_signal = analyze_amplitude(input, sample_rate, &settings);

    let spectral_features = analyze_spectrum(input, sample_rate, &settings);
    let frequency_signal = analyze_frequency(&spectral_features, sample_rate, &settings);

    analyze_emphasis(&mut amplitude_signal, &spectral_features, &settings);

    (amplitude_signal, frequency_signal)
}

fn analyze_spectrum(
    input: &[f32],
    sample_rate: f32,
    settings: &OfflineAnalysisSettings,
) -> Vec<SpectralFeatures> {
    SpectrumAnalyzer::new(input, sample_rate, settings.spectrum).collect()
}

fn analyze_amplitude(
    input: &[f32],
    sample_rate: f32,
    settings: &OfflineAnalysisSettings,
) -> Vec<AmplitudeEvent> {
    let analysis_result: Vec<AmplitudeEvent> =
        AmplitudeAnalyzer::new(input, sample_rate, settings.amplitude).collect();

    let mut result = reduce_breakpoints(
        &analysis_result,
        ReduceBreakpointsSettings {
            region_duration: 1.0,
            maximum_breakpoints_per_region: settings.amplitude_breakpoints_per_second as usize,
            minimum_score: settings.minimum_breakpoint_score,
        },
    );

    // Ensure that there's an event at time 0
    match result.first().cloned() {
        Some(first_event) => {
            if first_event.time != 0.0 {
                result.insert(
                    0,
                    AmplitudeEvent {
                        time: 0.0,
                        amplitude: first_event.amplitude,
                        emphasis: None,
                    },
                );
            }
        }
        None => result.push(AmplitudeEvent::default()),
    }

    result
}

fn analyze_frequency(
    spectral_features: &[SpectralFeatures],
    sample_rate: f32,
    settings: &OfflineAnalysisSettings,
) -> Vec<FrequencyEvent> {
    let frequency_events = crate::frequency_analyzer::analyze_frequency(
        spectral_features,
        sample_rate,
        settings.frequency,
    );

    reduce_breakpoints(
        &frequency_events,
        ReduceBreakpointsSettings {
            region_duration: 1.0,
            maximum_breakpoints_per_region: settings.frequency_breakpoints_per_second as usize,
            minimum_score: settings.minimum_breakpoint_score,
        },
    )
}

fn analyze_emphasis(
    amplitude_signal: &mut [AmplitudeEvent],
    spectral_features: &[SpectralFeatures],
    settings: &OfflineAnalysisSettings,
) {
    if settings.emphasis_enabled {
        assign_emphasis_events(amplitude_signal, spectral_features, &settings.emphasis);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run_offline_analysis_test(
        input: &[f32],
        sample_rate: f32,
        settings: OfflineAnalysisSettings,
        expected_amplitude: &[(f32, f32)],
        expected_frequency: &[(f32, f32)],
    ) {
        let haptic = audio_to_haptics(input, sample_rate, settings, true, false).unwrap();

        let result_amplitude = haptic.signals.continuous.envelopes.amplitude;
        let result_frequency = haptic
            .signals
            .continuous
            .envelopes
            .frequency
            .expect("Missing frequency signal");

        let expected_events: Vec<AmplitudeBreakpoint> = expected_amplitude
            .iter()
            .map(|(time, amplitude)| AmplitudeBreakpoint {
                time: *time,
                amplitude: *amplitude,
                emphasis: None,
            })
            .collect();

        for (index, (expected, actual)) in expected_events
            .iter()
            .zip(result_amplitude.iter())
            .enumerate()
        {
            assert_eq!(
                expected, actual,
                "Mismatch in amplitude event at position {index}, expected: {expected:?}, actual: {actual:?}\
                 \n  expected events: {expected_events:#?}\
                 \n  actual events: {result_amplitude:#?}\n",
            );
        }

        assert_eq!(
            expected_amplitude.len(),
            result_amplitude.len(),
            "Mismatch in amplitude output length, expected: {}, actual: {}",
            expected_amplitude.len(),
            result_amplitude.len()
        );

        let expected_events: Vec<FrequencyBreakpoint> = expected_frequency
            .iter()
            .map(|(time, frequency)| FrequencyBreakpoint {
                time: *time,
                frequency: *frequency,
            })
            .collect();

        for (index, (expected, actual)) in expected_events
            .iter()
            .zip(result_frequency.iter())
            .enumerate()
        {
            assert_eq!(
                expected, actual,
                "Mismatch in frequency event at position {index}, expected: {expected:?}, actual: {actual:?}\
                 \n  expected events: {expected_events:#?}\
                 \n  actual events: {result_frequency:#?}\n",
            );
        }

        assert_eq!(
            expected_frequency.len(),
            result_frequency.len(),
            "Mismatch in frequency output length, expected: {}, actual: {}",
            expected_frequency.len(),
            result_frequency.len()
        );
    }

    #[test]
    fn two_seconds_of_input_with_sample_rate_4() {
        let sample_rate = 4.0;

        let mut input = vec![];

        // 1 second of 1s
        input.extend(std::iter::repeat_n(1.0, sample_rate as usize));

        // 1 second of silence
        input.extend(std::iter::repeat_n(0.0, sample_rate as usize));

        let settings = OfflineAnalysisSettings {
            spectrum: SpectrumAnalysisSettings {
                fft_size: 4,
                overlap_factor: 2,
                centroid_lowpass_hz: sample_rate / 2.0,
            },
            amplitude: AmplitudeAnalysisSettings {
                time_between_updates: 1.0 / sample_rate,
                envelope_attack_time: 0.0,
                envelope_hold_time: 0.0,
                envelope_release_time: 0.0,
                rms_windowing_time: 1.0,
            },
            frequency: FrequencyAnalysisSettings {
                frequency_min: 0.0,
                frequency_max: sample_rate / 2.0,
                ..Default::default()
            },
            amplitude_breakpoints_per_second: sample_rate as u32,
            frequency_breakpoints_per_second: sample_rate as u32,
            minimum_breakpoint_score: 0.0,
            emphasis_enabled: false,
            ..OfflineAnalysisSettings::default()
        };

        run_offline_analysis_test(
            &input,
            sample_rate,
            settings,
            &[
                // amplitude
                (0.0, 0.5),
                (0.25, 0.5),
                (0.5, 0.70710677),
                (0.75, 0.8660254),
                (1.0, 1.0),
                (1.25, 0.8660254),
                (1.5, 0.70710677),
                (1.75, 0.5),
                (2.0, 0.0),
            ],
            &[
                // frequency
                // 5 events are produced:
                //   8 input samples, fft size 4, overlap 2 -> 5 frame centers within the input
                (0.0, 0.5),
                (0.5, 0.2071068),
                (1.0, 0.5),
                (1.5, 0.0),
                (2.0, 0.0),
            ],
        );
    }

    #[test]
    fn two_seconds_of_input_with_sample_rate_8_and_breakpoint_reduction() {
        let sample_rate = 8.0;

        // 1 second of 1s
        let input = vec![1.0; sample_rate as usize];

        let settings = OfflineAnalysisSettings {
            spectrum: SpectrumAnalysisSettings {
                fft_size: 4,
                overlap_factor: 4,
                centroid_lowpass_hz: sample_rate / 2.0,
            },
            amplitude: AmplitudeAnalysisSettings {
                time_between_updates: 1.0 / sample_rate,
                envelope_attack_time: 0.0,
                envelope_hold_time: 0.0,
                envelope_release_time: 0.0,
                rms_windowing_time: 1.0,
            },
            frequency: FrequencyAnalysisSettings {
                frequency_min: 0.0,
                frequency_max: sample_rate / 2.0,
                ..Default::default()
            },
            amplitude_breakpoints_per_second: sample_rate as u32,
            frequency_breakpoints_per_second: sample_rate as u32,
            minimum_breakpoint_score: 0.0,
            emphasis_enabled: false,
            ..OfflineAnalysisSettings::default()
        };

        run_offline_analysis_test(
            &input,
            sample_rate,
            settings,
            &[
                // amplitude
                (0.0, 0.35355338),
                (0.125, 0.35355338),
                (0.25, 0.5),
                (0.375, 0.61237246),
                (0.5, 0.70710677),
                (0.625, 0.7905694),
                (0.75, 0.8660254),
                (0.875, 0.9354144),
                (1.0, 1.0),
            ],
            &[
                // frequency
                // 8 events are produced:
                //   8 input samples, fft size 4, overlap 4 -> 9 frame centers within the input
                (0.0, 0.5),
                (0.125, 0.2071068),
                (0.25, 0.2071068),
                (0.375, 0.2071068),
                (0.5, 0.2071068),
                (0.625, 0.2071068),
                (0.75, 0.2071068),
                (0.875, 0.2071068),
                (1.0, 0.5),
            ],
        );
    }
}
