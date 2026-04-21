// Copyright (c) Meta Platforms, Inc. and affiliates.
//
// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

//! Support for the AHAP file format, including conversion from [HapticData]

use serde::Deserialize;
use serde::Serialize;

use crate::HapticData;
use crate::v1::AmplitudeBreakpoint;
use crate::v1::FrequencyBreakpoint;

const AMPLITUDE_DUCKING: f32 = 0.2;
const DELTA_ERR: f32 = 0.000_000_1;

// As per documentation at https://developer.apple.com/documentation/corehaptics/chhapticevent/eventtype/3081794-hapticcontinuous,
// a continuous event is limited to 30 seconds.
const MAX_CONTINUOUS_EVENT_DURATION: f32 = 30.0;

/// Core Haptics AHAP format, root structure
///
/// See <https://developer.apple.com/documentation/corehaptics/representing_haptic_patterns_in_ahap_files>
#[derive(PartialEq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
#[allow(missing_docs)]
pub struct Ahap {
    pub version: f32,
    pub pattern: Vec<Pattern>,
}

impl Default for Ahap {
    fn default() -> Self {
        Self {
            version: 1.0,
            pattern: Default::default(),
        }
    }
}

/// Core Haptics AHAP format, Pattern enum
///
/// AHAP also has a Parameter variant, but we omit that here since we don't use it.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
#[allow(missing_docs)]
pub enum Pattern {
    Event(Event),
    ParameterCurve(ParameterCurve),
}

/// Core Haptics AHAP format, Event structure
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
#[serde(tag = "EventType")]
#[allow(missing_docs)]
pub enum Event {
    #[serde(rename_all = "PascalCase")]
    HapticContinuous {
        time: f32,
        event_duration: f32,
        event_parameters: Vec<EventParameter>,
    },
    #[serde(rename_all = "PascalCase")]
    HapticTransient {
        time: f32,
        event_parameters: Vec<EventParameter>,
    },
}

/// Core Haptics AHAP format, EventParameter structure
#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
#[allow(missing_docs)]
pub struct EventParameter {
    #[serde(rename = "ParameterID")]
    pub parameter_id: ParameterId,
    pub parameter_value: f32,
}

impl PartialEq for EventParameter {
    fn eq(&self, other: &Self) -> bool {
        if self.parameter_id == other.parameter_id {
            (self.parameter_value - other.parameter_value).abs() <= DELTA_ERR
        } else {
            false
        }
    }
}

/// Core Haptics AHAP format, Parameter structure
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
#[allow(missing_docs)]
pub struct Parameter {
    #[serde(rename = "ParameterID")]
    pub parameter_id: DynamicParameterId,
    pub parameter_value: f32,
    pub time: f32,
}

/// Core Haptics AHAP format, ParameterCurve structure
#[derive(Default, PartialEq, Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
#[allow(missing_docs)]
pub struct ParameterCurve {
    #[serde(rename = "ParameterID")]
    pub parameter_id: DynamicParameterId,
    pub time: f32,
    pub parameter_curve_control_points: Vec<ParameterCurveControlPoint>,
}

/// Core Haptics AHAP format, ParameterId type for ParameterCurve
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
#[allow(missing_docs)]
pub enum DynamicParameterId {
    #[default]
    HapticIntensityControl,
    HapticSharpnessControl,
}

/// Core Haptics AHAP format, ParameterId type for EventParameter
#[derive(Debug, PartialEq, Eq, Copy, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
#[allow(missing_docs)]
pub enum ParameterId {
    HapticIntensity,
    HapticSharpness,
}

/// Core Haptics AHAP format, ParameterCurveControlPoint structure
#[derive(Debug, Copy, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
#[allow(missing_docs)]
pub struct ParameterCurveControlPoint {
    pub time: f32,
    pub parameter_value: f32,
}

/// An [Ahap] that is split into its continuous and transient components.
///
/// Such a split is useful when playing back a haptic clip with the Core Haptics API. If everything
/// is in one AHAP, Core Haptic's CHHapticPatternPlayer modulates the transients based on the
/// contents of the continuous curves. We however want the transients to be played back
/// independently of the continuous curves. This can be achieved by using two different
/// CHHapticPatternPlayer, loading [SplitAhap::continuous] into one, and [SplitAhap::transients]
/// into the other.
#[derive(PartialEq, Debug)]
pub struct SplitAhap {
    /// The continuous component of the original AHAP
    pub continuous: Ahap,

    /// The transients component of the original AHAP. Can be [None] if the original AHAP had no
    /// transients.
    pub transients: Option<Ahap>,
}

impl From<Ahap> for SplitAhap {
    fn from(ahap: Ahap) -> Self {
        let mut transients = Ahap::default();
        let mut continuous = Ahap::default();

        for pattern in ahap.pattern {
            match pattern {
                Pattern::Event(event) => match event {
                    Event::HapticContinuous {
                        time,
                        event_duration,
                        event_parameters,
                    } => continuous
                        .pattern
                        .push(Pattern::Event(Event::HapticContinuous {
                            time,
                            event_duration,
                            event_parameters,
                        })),
                    Event::HapticTransient {
                        time,
                        event_parameters,
                    } => transients
                        .pattern
                        .push(Pattern::Event(Event::HapticTransient {
                            time,
                            event_parameters,
                        })),
                },
                Pattern::ParameterCurve(parameter_curve) => {
                    continuous
                        .pattern
                        .push(Pattern::ParameterCurve(parameter_curve));
                }
            }
        }

        // In case there are no transients in AHAP, return None so they are not played
        if transients.pattern.is_empty() {
            Self {
                continuous,
                transients: None,
            }
        } else {
            Self {
                continuous,
                transients: Some(transients),
            }
        }
    }
}

impl PartialEq for ParameterCurveControlPoint {
    fn eq(&self, other: &Self) -> bool {
        if (self.time - other.time).abs() <= DELTA_ERR {
            (self.parameter_value - other.parameter_value).abs() <= DELTA_ERR
        } else {
            false
        }
    }
}

fn get_intensity_from_amplitude_breakpoint(breakpoint: &AmplitudeBreakpoint) -> f32 {
    // sqrt() is used here because in Core Haptics, the intensity is quadratic, not linear.
    if breakpoint.emphasis.is_some() {
        // When a breakpoint has an emphasis point, we reduce its amplitude. This is done to lower
        // the effect of phase cancellation: When Core Haptics plays a continuous and a transient
        // pattern at the same time, phase cancellation can happen when Core Haptics mixes both
        // signals together. This phase cancellation lowers the overall intensity of the transient.
        // By lowering the continuous amplitude here, the phase cancellation effect is reduced,
        // leading to a higher intensity of the transient.
        breakpoint.amplitude.sqrt() * (1.0 - AMPLITUDE_DUCKING)
    } else {
        breakpoint.amplitude.sqrt()
    }
}

fn ahap_transient_events_from_breakpoints(breakpoints: &[AmplitudeBreakpoint]) -> Vec<Pattern> {
    breakpoints
        .iter()
        .filter(|&x| x.emphasis.is_some())
        .map(|x| {
            Pattern::Event(Event::HapticTransient {
                time: x.time,
                event_parameters: vec![
                    EventParameter {
                        parameter_id: ParameterId::HapticIntensity,
                        parameter_value: x.emphasis.as_ref().map_or(0.0, |x| x.amplitude.sqrt()),
                    },
                    EventParameter {
                        parameter_id: ParameterId::HapticSharpness,
                        parameter_value: x.emphasis.as_ref().map_or(0.0, |x| x.frequency),
                    },
                ],
            })
        })
        .collect::<Vec<Pattern>>()
}

/// Creates events of type HapticContinuous from a list of amplitude breakpoints.
///
/// Each event will just have a constant intensity of 1 and a constant sharpness of 0. The intensity
/// and sharpness change during playback because parameter curves that modulate these constant
/// values are added to the AHAP in another place.
///
/// The only reason to use multiple events here is because CoreHaptics limits events of type
/// HapticContinuous to 30 seconds.
fn ahap_continuous_events_from_breakpoints(breakpoints: &[AmplitudeBreakpoint]) -> Vec<Pattern> {
    let mut total_remaining_duration = match breakpoints.last() {
        None => 0.0,
        Some(last) => last.time,
    };
    let event_count = (total_remaining_duration / MAX_CONTINUOUS_EVENT_DURATION).ceil() as u32;
    let mut result = Vec::new();
    for i in 0..event_count {
        let time = i as f32 * MAX_CONTINUOUS_EVENT_DURATION;
        let event_duration = if total_remaining_duration > MAX_CONTINUOUS_EVENT_DURATION {
            MAX_CONTINUOUS_EVENT_DURATION
        } else {
            total_remaining_duration
        };
        total_remaining_duration -= event_duration;

        let ahap_pattern_continuous_event = Pattern::Event(Event::HapticContinuous {
            time,
            event_duration,
            event_parameters: vec![
                EventParameter {
                    parameter_id: ParameterId::HapticIntensity,
                    parameter_value: 1.0,
                },
                EventParameter {
                    parameter_id: ParameterId::HapticSharpness,
                    parameter_value: 0.0,
                },
            ],
        });

        result.push(ahap_pattern_continuous_event);
    }
    result
}

impl From<HapticData> for Ahap {
    fn from(haptic_data: HapticData) -> Self {
        let signals = &haptic_data.signals;
        let mut ahap_data = Self::default();
        let mut transient_events_data = Vec::new();

        //
        // Convert amplitude envelope to intensity CHParameterCurve.
        // Parameter curves are added to ahap_data.pattern.
        // Transients are collected in transient_events_data.
        //

        {
            // Get the first amplitude breakpoint or set to default, if non-existent
            let default_control_point = AmplitudeBreakpoint::default();
            let first_control_point = match signals.continuous.envelopes.amplitude.first() {
                None => &default_control_point,
                Some(first) => first,
            };

            // Get all amplitude breakpoints
            let amplitude_breakpoints = &signals.continuous.envelopes.amplitude[..];

            // Add control points to ParameterCurve for Intensity
            let parameter_curve_control_points = amplitude_breakpoints
                .iter()
                .map(|point| ParameterCurveControlPoint {
                    time: point.time,
                    parameter_value: get_intensity_from_amplitude_breakpoint(point),
                })
                .collect::<Vec<ParameterCurveControlPoint>>();

            let parameter_curve_intensity = Pattern::ParameterCurve(ParameterCurve {
                parameter_id: DynamicParameterId::HapticIntensityControl,
                time: first_control_point.time,
                parameter_curve_control_points,
            });

            ahap_data.pattern.push(parameter_curve_intensity);

            transient_events_data.extend(ahap_transient_events_from_breakpoints(
                amplitude_breakpoints,
            ));
        }

        //
        // Convert frequency envelope to sharpness CHParameterCurve.
        // Parameter curves are added to ahap_data.pattern.
        //

        match &signals.continuous.envelopes.frequency {
            None => {}
            Some(frequency_breakpoint_vec) => {
                let default_control_point = FrequencyBreakpoint::default();
                let first_control_point = match frequency_breakpoint_vec.first() {
                    None => &default_control_point,
                    Some(first) => first,
                };

                // Get all frequency breakpoints
                let frequency_breakpoints = &frequency_breakpoint_vec[..];

                // Add control points to ParameterCurve for Sharpness
                let parameter_curve_control_points = frequency_breakpoints
                    .iter()
                    .map(|point| ParameterCurveControlPoint {
                        time: point.time,
                        parameter_value: point.frequency.sqrt(),
                    })
                    .collect::<Vec<ParameterCurveControlPoint>>();

                let parameter_curve_sharpness = Pattern::ParameterCurve(ParameterCurve {
                    parameter_id: DynamicParameterId::HapticSharpnessControl,
                    time: first_control_point.time,
                    parameter_curve_control_points,
                });

                ahap_data.pattern.push(parameter_curve_sharpness);
            }
        };

        //
        // Assemble the remaining parts of ahap_data
        //

        ahap_data
            .pattern
            .append(&mut ahap_continuous_events_from_breakpoints(
                &signals.continuous.envelopes.amplitude,
            ));

        // Add transients at the end, to have a bit of order in the AHAP
        ahap_data.pattern.append(&mut transient_events_data);

        ahap_data
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::*;

    fn compare_haptic_with_ahap(haptic_path: &str, ahap_path: &str) {
        let haptic_data = load_test_file_as_haptic_data(haptic_path);
        let ahap_from_haptic = Ahap::from(haptic_data);
        let ahap_reference_json = load_test_file_as_string(ahap_path);
        let ahap_reference_data = serde_json::from_str::<Ahap>(&ahap_reference_json).unwrap();
        assert_eq!(ahap_reference_data, ahap_from_haptic);
    }

    fn load_test_file_as_ahap(filename: &str) -> Result<Ahap, serde_json::Error> {
        let ahap = load_test_file_as_string(filename);
        serde_json::from_str::<Ahap>(&ahap)
    }

    /// Tests that a valid AHAP file can be deserialized into [Ahap]
    #[test]
    fn test_deserializing_valid_ahap() {
        load_test_file_as_ahap("valid.ahap").unwrap();
    }

    /// Tests that deserializing an invalid AHAP file will produce an error
    #[test]
    fn test_deserializing_invalid_ahap() {
        let err = load_test_file_as_ahap("invalid.ahap").unwrap_err();
        assert!(
            err.to_string()
                .contains("unknown variant `ParameterCurves`")
        );
    }

    /// Tests that an AHAP file exported from Haptics Studio can be deserialized
    #[test]
    fn test_deserializing_studio_export_ahap() {
        load_test_file_as_ahap("studio_export.ahap").unwrap();
    }

    #[test]
    /// Tests that an AHAP file with just a transient can be deserialized
    fn test_load_test_file_as_ahap_transient() {
        load_test_file_as_ahap("single_transient.ahap").unwrap();
    }

    /// Tests converting a .haptic to AHAP
    #[test]
    fn test_ahap_from_haptic() {
        compare_haptic_with_ahap("valid_v1.haptic", "valid_v1.ahap");
    }

    /// Tests converting a .haptic with multiple emphasis points to AHAP
    #[test]
    fn test_ahap_from_haptic_various_emphasis_count() {
        compare_haptic_with_ahap("multiple_emphasis.haptic", "multiple_emphasis.ahap");
    }

    /// Tests .haptic to AHAP conversion for various amount of points that could trigger corner
    /// cases
    #[test]
    fn test_ahap_from_haptic_data_various_point_count() {
        // Make sure that haptic data with only 2 and 3 points get correctly converted to AHAP.
        compare_haptic_with_ahap("2_points.haptic", "2_points.ahap");
        compare_haptic_with_ahap("3_points.haptic", "3_points.ahap");

        // A ParameterCurve in AHAP can only contain up to 16 points. Verify that the chunking
        // algorithm deals correctly with the boundary condition.
        compare_haptic_with_ahap("16_points.haptic", "16_points.ahap");
        compare_haptic_with_ahap("17_points.haptic", "17_points.ahap");
    }

    /// Tests .haptic to AHAP conversion for haptic data that is longer than 30 seconds
    #[test]
    fn test_30_second_limit() {
        compare_haptic_with_ahap("long_clip.haptic", "long_clip.ahap");
    }

    #[test]
    fn test_ahap_split() {
        let ahap = load_test_file_as_ahap("valid_v1.ahap").unwrap();
        let split_ahap_actual: SplitAhap = ahap.into();
        let split_ahap_reference = SplitAhap {
            continuous: load_test_file_as_ahap("valid_v1_continuous.ahap").unwrap(),
            transients: Some(load_test_file_as_ahap("valid_v1_transients.ahap").unwrap()),
        };

        assert_eq!(split_ahap_actual, split_ahap_reference);
    }

    #[test]
    fn test_ahap_split_no_transients() {
        let ahap = load_test_file_as_ahap("3_points.ahap").unwrap();
        let split_ahap_actual: SplitAhap = ahap.into();
        let split_ahap_reference = SplitAhap {
            continuous: load_test_file_as_ahap("3_points_continuous.ahap").unwrap(),
            transients: None,
        };

        assert_eq!(split_ahap_actual, split_ahap_reference);
    }
}
