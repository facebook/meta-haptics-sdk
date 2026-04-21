// (c) Meta Platforms, Inc. and affiliates. Confidential and proprietary.

use std::cmp::Ordering;
use std::ops::RangeInclusive;
use std::time::Duration;

use serde::Deserialize;
use serde::Serialize;
use thiserror::Error;
use typeshare::typeshare;

use crate::Breakpoint;
use crate::Version;

const VALID_BREAKPOINT_VALUE_RANGE: RangeInclusive<f32> = 0.0..=1.0;

const VERSION: Version = Version {
    major: 1,
    minor: 0,
    patch: 0,
};

/// The primary haptic data format supported by HapticsSDK
///
/// Haptic data is typically serialized as JSON data in a file with a .haptic extension.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[typeshare]
pub struct HapticData {
    /// The haptic data's version
    pub version: Version,
    /// Metadata associated with the haptic data
    #[serde(default)]
    pub metadata: MetaData,
    /// The signals contained in the haptic data
    pub signals: Signals,
}

impl Default for HapticData {
    fn default() -> Self {
        Self {
            version: VERSION,
            metadata: Default::default(),
            signals: Default::default(),
        }
    }
}

/// Defines how strict the validation of the haptic data is
pub enum ValidationMode {
    /// Validation will fail if any rule check fails
    Strict,

    /// Validation will fail for most rule check failures
    ///
    /// Some rules are not validated, to stay compatible with .haptic files that were created with
    /// earlier versions of Haptics Studio, which created invalid haptic data.
    LegacyCompatibility,
}

impl HapticData {
    /// Returns the duration of the haptic data, which is defined by the time of the last
    /// amplitude breakpoint
    pub fn duration(&self) -> Duration {
        match self.signals.continuous.envelopes.amplitude.last() {
            Some(breakpoint) => Duration::from_secs_f32(breakpoint.time),
            None => Duration::ZERO,
        }
    }

    /// Examples of invalid HapticData
    /// - Breakpoints and emphasis values are < 0.0 or > 1.0.
    /// - The breakpoint time values are not consecutive.
    /// - Emphasis amplitude is smaller than breakpoint amplitude value
    pub fn validate(self, validation_mode: ValidationMode) -> Result<Self, ValidationError> {
        use ValidationError::*;

        let mut start_time_amp = None;
        let mut last_time_amp: f32 = 0.0; // variable to keep track of the previous breakpoint time

        for amp_bp in self.signals.continuous.envelopes.amplitude.iter() {
            if !VALID_BREAKPOINT_VALUE_RANGE.contains(&amp_bp.amplitude) {
                return Err(AmplitudeOutOfRange {
                    time: amp_bp.time,
                    amplitude: amp_bp.amplitude,
                });
            }

            if last_time_amp > amp_bp.time {
                return Err(AmplitudeBreakpointsOutOfOrder {
                    time: amp_bp.time,
                    previous_time: last_time_amp,
                });
            }

            if start_time_amp.is_none() {
                start_time_amp = Some(amp_bp.time);
            }

            last_time_amp = amp_bp.time;

            if let Some(emphasis) = &amp_bp.emphasis {
                if !VALID_BREAKPOINT_VALUE_RANGE.contains(&emphasis.amplitude) {
                    return Err(EmphasisAmplitudeOutOfRange {
                        time: amp_bp.time,
                        amplitude: emphasis.amplitude,
                    });
                }

                if !VALID_BREAKPOINT_VALUE_RANGE.contains(&emphasis.frequency) {
                    return Err(EmphasisFrequencyOutOfRange {
                        time: amp_bp.time,
                        frequency: emphasis.frequency,
                    });
                }

                if emphasis.amplitude < amp_bp.amplitude {
                    return Err(EmphasisAmpLowerThanEnvelopeAmp {
                        time: amp_bp.time,
                        emphasis_amp: emphasis.amplitude,
                        envelope_amp: amp_bp.amplitude,
                    });
                }
            }
        }

        let time_range_amp = if let Some(start_time_amp) = start_time_amp {
            if self.signals.continuous.envelopes.amplitude.len() == 1 {
                return Err(InsufficientAmplitudeBreakpoints);
            }

            if !matches!(
                (last_time_amp - start_time_amp).partial_cmp(&0.0),
                Some(Ordering::Greater)
            ) {
                return Err(AmplitudeEnvelopeHasZeroDuration);
            }

            start_time_amp..=last_time_amp
        } else {
            return Err(InsufficientAmplitudeBreakpoints);
        };

        let mut last_time_freq = 0.0;

        if let Some(frequency_envelope) = &self.signals.continuous.envelopes.frequency {
            for freq_bp in frequency_envelope.iter() {
                if !VALID_BREAKPOINT_VALUE_RANGE.contains(&freq_bp.frequency) {
                    return Err(FrequencyOutOfRange {
                        time: freq_bp.time,
                        frequency: freq_bp.frequency,
                    });
                }

                let bp_time = freq_bp.time;

                if last_time_freq > bp_time {
                    return Err(FrequencyBreakpointsOutOfOrder {
                        time: bp_time,
                        previous_time: last_time_freq,
                    });
                }

                if let ValidationMode::Strict = validation_mode {
                    if !time_range_amp.contains(&bp_time) {
                        return Err(FrequencyBreakpointOutsideAmplitudeEnvelope {
                            time: bp_time,
                            amplitude_envelope_end: *time_range_amp.end(),
                        });
                    }
                }

                last_time_freq = freq_bp.time;
            }
        }

        Ok(self)
    }
}

/// The different kinds of haptic data validation errors that can occur
#[derive(Error, Debug, PartialEq)]
#[allow(missing_docs)]
pub enum ValidationError {
    #[error("amplitude out of range: (time: {time}, amplitude: {amplitude})")]
    AmplitudeOutOfRange { time: f32, amplitude: f32 },
    #[error("amplitude breakpoints out of order: (time: {time} is after {previous_time})")]
    AmplitudeBreakpointsOutOfOrder { time: f32, previous_time: f32 },
    #[error("emphasis amplitude out of range: (time: {time}, amplitude: {amplitude})")]
    EmphasisAmplitudeOutOfRange { time: f32, amplitude: f32 },
    #[error("emphasis frequency out of range: (time: {time}, frequency: {frequency})")]
    EmphasisFrequencyOutOfRange { time: f32, frequency: f32 },
    #[error(
        "emphasis amplitude lower than envelope amplitude: \
         (time: {time}, emphasis amp: {emphasis_amp}, envelope amp: {envelope_amp})"
    )]
    EmphasisAmpLowerThanEnvelopeAmp {
        time: f32,
        emphasis_amp: f32,
        envelope_amp: f32,
    },
    #[error("the amplitude envelope must contain at least two breakpoints")]
    InsufficientAmplitudeBreakpoints,
    #[error("the amplitude envelope must have a duration greater than zero")]
    AmplitudeEnvelopeHasZeroDuration,
    #[error("frequency out of range: (time: {time}, frequency: {frequency})")]
    FrequencyOutOfRange { time: f32, frequency: f32 },
    #[error("frequency breakpoints out of order: (time: {time} is after {previous_time})")]
    FrequencyBreakpointsOutOfOrder { time: f32, previous_time: f32 },
    #[error(
        "frequency breakpoint at {time} is after end of amplitude envelope at {amplitude_envelope_end}"
    )]
    FrequencyBreakpointOutsideAmplitudeEnvelope {
        time: f32,
        amplitude_envelope_end: f32,
    },
}

///(optional) Metadata structure
#[derive(Default, Clone, Serialize, Deserialize, PartialEq, Eq, Debug)]
#[typeshare]
pub struct MetaData {
    /// The name of the editor that was used to create the `HapticData`
    #[serde(default)]
    pub editor: String,
    /// The name of the `HapticData`s author
    #[serde(default)]
    pub author: String,
    /// The source of the `HapticData`'s contents,
    /// e.g. the name of the audio file that was analyzed to produce the haptic data.
    #[serde(default)]
    pub source: String,
    /// The name of the `HapticData`s project
    #[serde(default)]
    pub project: String,
    /// A series of tags that can be attached to the `HapticData`
    #[serde(default)]
    pub tags: Vec<String>,
    /// A description of the `HapticData's` contents
    #[serde(default)]
    pub description: String,
}

/// Signal structure that describes haptic data.
///
/// Currently contains only a `SignalContinuous` that represents a decomposed haptic signal over a
/// period of time.
#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
#[typeshare]
pub struct Signals {
    /// The continuous signal
    pub continuous: SignalContinuous,
}

/// Represents a decomposed haptic signal over a period of time
#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
#[typeshare]
pub struct SignalContinuous {
    /// The envelopes contained in the continuous signal
    pub envelopes: Envelopes,
}

/// Envelopes of a `SignalContinuous`.
///
/// Allows to change `amplitude` and `frequency` of a `SignalContinuous` over time.
#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
#[typeshare]
pub struct Envelopes {
    /// The amplitude envelope
    pub amplitude: Vec<AmplitudeBreakpoint>,
    /// The optional frequency envelope
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency: Option<Vec<FrequencyBreakpoint>>,
}

/// Amplitude breakpoints of a `SignalContinuous` Amplitude envelope.
///
/// Emphasis may be optionally attached to the breakpoint.
#[derive(Default, Copy, Clone, PartialEq, Debug, Serialize, Deserialize)]
#[typeshare]
pub struct AmplitudeBreakpoint {
    /// The breakpoint's time
    pub time: f32,
    /// The breakpoint's amplitude
    pub amplitude: f32,
    /// The breakpoint's optional emphasis
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emphasis: Option<Emphasis>,
}

impl Breakpoint for AmplitudeBreakpoint {
    fn time(&self) -> f32 {
        self.time
    }

    fn value(&self) -> f32 {
        self.amplitude
    }

    fn from_time_value(time: f32, value: f32) -> Self {
        Self {
            time,
            amplitude: value,
            emphasis: None,
        }
    }
}

/// Emphasis associated with an Amplitude envelope breakpoint.
///
/// Allows for a "haptic highlight" of the breakpoint.
#[derive(Clone, Copy, Default, PartialEq, Debug, Serialize, Deserialize)]
#[typeshare]
pub struct Emphasis {
    /// The amplitude of the emphasis event
    pub amplitude: f32,
    /// The frequency of the emphasis event
    pub frequency: f32,
}

/// Data associated with a Frequency envelope breakpoint.
#[derive(Clone, Copy, Default, PartialEq, Debug, Serialize, Deserialize)]
#[typeshare]
pub struct FrequencyBreakpoint {
    /// The time of the breakpoint
    pub time: f32,
    /// The frequency of the breakpoint
    pub frequency: f32,
}

impl Breakpoint for FrequencyBreakpoint {
    fn time(&self) -> f32 {
        self.time
    }

    fn value(&self) -> f32 {
        self.frequency
    }

    fn from_time_value(time: f32, value: f32) -> Self {
        Self {
            time,
            frequency: value,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod serialization {
        use super::*;
        use crate::test_utils::*;

        fn create_test_data_model() -> HapticData {
            HapticData {
                version: Version {
                    major: 1,
                    minor: 0,
                    patch: 0,
                },
                metadata: MetaData {
                    editor: "VSCode".to_owned(),
                    author: "SDK Team".to_owned(),
                    tags: vec!["Test".to_owned()],
                    description: "Testing".to_owned(),
                    ..Default::default()
                },
                ..TestClip {
                    amplitude: &[
                        amp_bp(0.0, 0.2),
                        amp_bp(0.1, 0.3),
                        amp_bp(0.2, 0.2),
                        emphasis_bp(0.3, 0.5, 0.69, 0.7),
                    ],
                    frequency: &[
                        freq_bp(0.1, 0.99),
                        freq_bp(0.2, 0.54),
                        freq_bp(0.25, 0.8),
                        freq_bp(0.3, 0.9),
                    ],
                }
                .into()
            }
        }

        fn serialize_test_data_json() -> String {
            let data = create_test_data_model();
            serde_json::to_string_pretty(&data).unwrap()
        }

        fn deserialize_test_data_json() -> HapticData {
            let serialized_json = serialize_test_data_json();
            let deserialized_json: HapticData = serde_json::from_str(&serialized_json).unwrap();

            deserialized_json
        }

        #[test]
        fn check_test_json_serialize_deserialize() {
            //verify if deserialized data matches the created data to be serialized
            let deserialized_json = deserialize_test_data_json();

            //version
            assert_eq!(deserialized_json.version.major, 1);
            assert_eq!(deserialized_json.version.minor, 0);
            assert_eq!(deserialized_json.version.patch, 0);

            //metadata
            assert_eq!(deserialized_json.metadata.author, "SDK Team");
            assert_eq!(deserialized_json.metadata.description, "Testing");
            assert_eq!(deserialized_json.metadata.editor, "VSCode");
            assert_eq!(deserialized_json.metadata.tags[0], "Test");

            //signals
            let serialized_signals = deserialized_json.signals;

            // check continuous

            assert_eq!(
                serialized_signals.continuous.envelopes.amplitude[0],
                AmplitudeBreakpoint {
                    time: 0.0,
                    amplitude: 0.2,
                    emphasis: None
                }
            );
            assert_eq!(
                serialized_signals.continuous.envelopes.amplitude[1],
                AmplitudeBreakpoint {
                    time: 0.1,
                    amplitude: 0.3,
                    emphasis: None
                }
            );
            assert_eq!(
                serialized_signals.continuous.envelopes.amplitude[2],
                AmplitudeBreakpoint {
                    time: 0.2,
                    amplitude: 0.2,
                    emphasis: None
                }
            );
            assert_eq!(
                serialized_signals.continuous.envelopes.amplitude[3],
                AmplitudeBreakpoint {
                    time: 0.3,
                    amplitude: 0.5,
                    emphasis: Some(Emphasis {
                        amplitude: 0.69,
                        frequency: 0.7,
                    }),
                }
            );

            let freq_vec = serialized_signals.continuous.envelopes.frequency.unwrap();
            assert_eq!(
                freq_vec[0],
                FrequencyBreakpoint {
                    time: 0.1,
                    frequency: 0.99
                }
            );
            assert_eq!(
                freq_vec[1],
                FrequencyBreakpoint {
                    time: 0.2,
                    frequency: 0.54
                }
            );
            assert_eq!(
                freq_vec[2],
                FrequencyBreakpoint {
                    time: 0.25,
                    frequency: 0.8
                }
            );
            assert_eq!(
                freq_vec[3],
                FrequencyBreakpoint {
                    time: 0.3,
                    frequency: 0.9
                }
            );
        }

        #[test]
        fn json_deserialized_required_fields_only() {
            let data: HapticData = load_test_file_as_haptic_data("valid_required_v1.haptic");

            let metadata = MetaData::default();
            let version = Version {
                major: 1,
                minor: 0,
                patch: 0,
            };

            //check if value of data not included in the file is the default
            assert_eq!(metadata, data.metadata);
            assert_eq!(version, data.version);
            assert_eq!(data.signals.continuous.envelopes.frequency, None);
        }

        #[test]
        fn check_serialized_required_only() {
            let reference_data: HapticData =
                load_test_file_as_haptic_data("valid_required_v1.haptic");

            let metadata = MetaData::default();
            let version = Version {
                major: 1,
                minor: 0,
                patch: 0,
            };

            let amplitude_envelope = vec![
                AmplitudeBreakpoint {
                    time: 0.0,
                    amplitude: 0.2,
                    emphasis: None,
                },
                AmplitudeBreakpoint {
                    time: 0.1,
                    amplitude: 0.3,
                    emphasis: None,
                },
                AmplitudeBreakpoint {
                    time: 0.2,
                    amplitude: 0.2,
                    emphasis: None,
                },
                AmplitudeBreakpoint {
                    time: 0.3,
                    amplitude: 0.5,
                    emphasis: None,
                },
            ];

            let signal_continuous = SignalContinuous {
                envelopes: Envelopes {
                    amplitude: amplitude_envelope,
                    frequency: None,
                },
            };

            let data = HapticData {
                version,
                metadata,
                signals: Signals {
                    continuous: signal_continuous,
                },
            };

            assert_eq!(reference_data, data);
        }

        #[test]
        fn check_test_json_deserialize() {
            let data: HapticData = load_test_file_as_haptic_data("valid_v1.haptic");

            let version = Version {
                major: 1,
                minor: 0,
                patch: 0,
            };

            //check if value of data not included in the file is the default
            assert_eq!(version, data.version);
        }

        #[test]
        fn check_test_json_deserialize_invalid_fields() {
            let data = serde_json::from_str::<HapticData>(&load_test_file_as_string(
                "invalid_fields_v1.haptic",
            ));
            let err = data.map(|_| ()).unwrap_err();
            assert!(err.to_string().contains("missing field `signals`"));
        }
    }

    mod validation_success {
        use super::*;
        use crate::test_utils::*;

        #[test]
        fn basic_validation() {
            let data = load_test_file_as_haptic_data("valid_v1.haptic");
            data.validate(ValidationMode::Strict).unwrap();
        }

        #[test]
        fn optional_fields() {
            let data = load_test_file_as_haptic_data("validation_v1_optionals.haptic");
            data.validate(ValidationMode::Strict).unwrap();
        }

        #[test]
        fn beta_impulses() {
            let haptic = load_test_file_as_haptic_data("valid_beta_impulses.haptic");
            haptic.validate(ValidationMode::Strict).unwrap();
        }

        #[test]
        fn freq_bp_outside_amp_env_legacy_validation() {
            let haptic = load_test_file_as_haptic_data("invalid_freq_bp_outside_amp_env.haptic");
            haptic
                .validate(ValidationMode::LegacyCompatibility)
                .unwrap();
        }
    }

    mod validation_failure {
        use ValidationError::*;

        use super::*;
        use crate::test_utils::*;

        fn check_that_validation_fails(path: &str, expected_error: ValidationError) {
            let haptic = load_test_file_as_haptic_data(path);
            let error = haptic
                .validate(ValidationMode::Strict)
                .map(|_| ())
                .unwrap_err();
            assert_eq!(expected_error, error);
        }

        #[test]
        fn amplitude_out_of_range() {
            check_that_validation_fails(
                "validation_v1_amplitude.haptic",
                AmplitudeOutOfRange {
                    time: 0.3,
                    amplitude: 1.5,
                },
            );
        }

        #[test]
        fn non_sequential_breakpoints() {
            check_that_validation_fails(
                "validation_v1_sequence.haptic",
                AmplitudeBreakpointsOutOfOrder {
                    time: 0.1,
                    previous_time: 0.2,
                },
            );
        }

        #[test]
        fn invalid_emphasis_amplitude() {
            check_that_validation_fails(
                "validation_v1_emphasis_amplitude.haptic",
                EmphasisAmpLowerThanEnvelopeAmp {
                    time: 0.2,
                    emphasis_amp: 0.2,
                    envelope_amp: 0.5,
                },
            );
        }

        #[test]
        fn emphasis_amplitude_out_of_range() {
            check_that_validation_fails(
                "validation_v1_emphasis_amplitude_range.haptic",
                EmphasisAmplitudeOutOfRange {
                    time: 0.2,
                    amplitude: -0.2,
                },
            );
        }

        #[test]
        fn emphasis_frequency_out_of_range() {
            check_that_validation_fails(
                "validation_v1_emphasis_frequency_range.haptic",
                EmphasisFrequencyOutOfRange {
                    time: 0.2,
                    frequency: 1.1,
                },
            );
        }

        #[test]
        fn empty_amplitude_envelope() {
            check_that_validation_fails(
                "invalid_empty_amplitude_envelope.haptic",
                InsufficientAmplitudeBreakpoints,
            );
        }

        #[test]
        fn single_amplitude_breakpoint() {
            check_that_validation_fails(
                "invalid_single_amplitude_breakpoint.haptic",
                InsufficientAmplitudeBreakpoints,
            );
        }

        #[test]
        fn zero_duration() {
            check_that_validation_fails(
                "invalid_zero_duration.haptic",
                AmplitudeEnvelopeHasZeroDuration,
            );
        }

        #[test]
        fn freq_bp_outside_amp_env_strict_validation() {
            check_that_validation_fails(
                "invalid_freq_bp_outside_amp_env.haptic",
                FrequencyBreakpointOutsideAmplitudeEnvelope {
                    time: 1.0001,
                    amplitude_envelope_end: 1.0,
                },
            );
        }
    }
}
