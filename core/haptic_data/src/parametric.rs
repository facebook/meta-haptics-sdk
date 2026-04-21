// (c) Meta Platforms, Inc. and affiliates. Confidential and proprietary.

use std::time::Duration;

use thiserror::Error;

use crate::HapticData;
use crate::interpolate_breakpoints;
use crate::v1;
use crate::v1::ValidationMode;

/// A paramtric point (amplitude point or frequency point)
pub struct Point {
    /// Time of the point
    pub time: Duration,
    /// Value of the point, either amplitude or frequency
    pub value: f32,
}

/// A parametric transient
pub struct Transient {
    /// Time of the transient
    pub time: Duration,
    /// Amplitude of the transient
    pub amplitude: f32,
    /// Frequency of the transient
    pub frequency: f32,
}

/// A parametric clip
pub struct Clip {
    /// Amplitude points of the clip
    pub amplitude_points: Vec<Point>,
    /// Frequency points of the clip
    pub frequency_points: Vec<Point>,
    /// Transients of the clip
    pub transients: Vec<Transient>,
}

/// Create HapticData from a parametric clip, then run validation
pub fn from_parametric(clip: &Clip) -> Result<HapticData, FromParametricError> {
    let amplitude_breakpoints = build_amplitude_breakpoints(clip)?;
    let frequency_breakpoints = build_frequency_breakpoints(clip);

    let haptic_data = v1::HapticData {
        signals: v1::Signals {
            continuous: v1::SignalContinuous {
                envelopes: v1::Envelopes {
                    amplitude: amplitude_breakpoints,
                    frequency: frequency_breakpoints,
                },
            },
        },
        ..Default::default()
    };

    haptic_data
        .validate(ValidationMode::Strict)
        .map_err(FromParametricError::ValidationError)
}

/// Threshold for merging transients with amplitude points (1ms in seconds)
const TIME_MERGE_THRESHOLD: f32 = 0.001;

fn build_amplitude_breakpoints(
    clip: &Clip,
) -> Result<Vec<v1::AmplitudeBreakpoint>, FromParametricError> {
    // Convert all amplitude points to amplitude breakpoints
    let mut breakpoints: Vec<v1::AmplitudeBreakpoint> = clip
        .amplitude_points
        .iter()
        .map(|p| v1::AmplitudeBreakpoint {
            time: p.time.as_secs_f32(),
            amplitude: p.value,
            emphasis: None,
        })
        .collect();

    // Convert all transients. Either add them to existing breakpoints, or create new breakpoints
    // if no breakpoint exists at the transient time.
    for transient in &clip.transients {
        let transient_time = transient.time.as_secs_f32();
        let emphasis = v1::Emphasis {
            amplitude: transient.amplitude,
            frequency: transient.frequency,
        };

        // Find index of the first breakpoint at or after the transient time
        let insert_pos = breakpoints
            .iter()
            .position(|bp| bp.time >= transient_time - TIME_MERGE_THRESHOLD)
            .ok_or(FromParametricError::TransientAfterLastAmplitudePoint { transient_time })?;

        // Merge with existing breakpoint
        if (breakpoints[insert_pos].time - transient_time).abs() <= TIME_MERGE_THRESHOLD {
            breakpoints[insert_pos].emphasis = Some(emphasis);
        }
        // Add new breakpoint, interpolating the amplitude value from the breakpoints around it
        else {
            let interpolated_amplitude = if insert_pos == 0 {
                breakpoints[0].amplitude
            } else {
                let prev = &breakpoints[insert_pos - 1];
                let next = &breakpoints[insert_pos];
                interpolate_breakpoints(prev, next, transient_time).amplitude
            };

            let new_bp = v1::AmplitudeBreakpoint {
                time: transient_time,
                amplitude: interpolated_amplitude,
                emphasis: Some(emphasis),
            };
            breakpoints.insert(insert_pos, new_bp);
        }
    }

    Ok(breakpoints)
}

fn build_frequency_breakpoints(clip: &Clip) -> Option<Vec<v1::FrequencyBreakpoint>> {
    if clip.frequency_points.is_empty() {
        return None;
    }

    Some(
        clip.frequency_points
            .iter()
            .map(|p| v1::FrequencyBreakpoint {
                time: p.time.as_secs_f32(),
                frequency: p.value,
            })
            .collect(),
    )
}

/// Errors that can occur when converting [Clip] to [HapticData]
#[derive(Error, Debug, PartialEq)]
#[allow(missing_docs)]
pub enum FromParametricError {
    #[error("transient at time {transient_time} is after the last amplitude point")]
    TransientAfterLastAmplitudePoint { transient_time: f32 },
    #[error(transparent)]
    ValidationError(v1::ValidationError),
}

#[cfg(test)]
mod tests {
    use assert_approx_eq::assert_approx_eq;

    use super::*;
    use crate::test_utils::*;

    fn point(time_secs: f32, value: f32) -> Point {
        Point {
            time: Duration::from_secs_f32(time_secs),
            value,
        }
    }

    fn transient(time_secs: f32, amplitude: f32, frequency: f32) -> Transient {
        Transient {
            time: Duration::from_secs_f32(time_secs),
            amplitude,
            frequency,
        }
    }

    fn assert_amp_bp_approx_eq(
        actual: &v1::AmplitudeBreakpoint,
        expected: &v1::AmplitudeBreakpoint,
    ) {
        assert_approx_eq!(actual.time, expected.time, 1.0e-6);
        assert_approx_eq!(actual.amplitude, expected.amplitude, 1.0e-6);
        match (&actual.emphasis, &expected.emphasis) {
            (Some(actual_emp), Some(expected_emp)) => {
                assert_approx_eq!(actual_emp.amplitude, expected_emp.amplitude, 1.0e-6);
                assert_approx_eq!(actual_emp.frequency, expected_emp.frequency, 1.0e-6);
            }
            _ => assert_eq!(actual.emphasis.is_some(), expected.emphasis.is_some()),
        }
    }

    fn assert_freq_bp_approx_eq(
        actual: &v1::FrequencyBreakpoint,
        expected: &v1::FrequencyBreakpoint,
    ) {
        assert_approx_eq!(actual.time, expected.time, 1.0e-6);
        assert_approx_eq!(actual.frequency, expected.frequency, 1.0e-6);
    }

    fn assert_haptic_data_eq(actual: &HapticData, expected: &TestClip) {
        let actual_amp = &actual.signals.continuous.envelopes.amplitude;
        assert_eq!(
            actual_amp.len(),
            expected.amplitude.len(),
            "Amplitude breakpoint count mismatch"
        );
        for (actual_bp, expected_bp) in actual_amp.iter().zip(expected.amplitude.iter()) {
            assert_amp_bp_approx_eq(actual_bp, expected_bp);
        }

        let actual_freq = actual
            .signals
            .continuous
            .envelopes
            .frequency
            .as_deref()
            .unwrap_or(&[]);
        assert_eq!(
            actual_freq.len(),
            expected.frequency.len(),
            "Frequency breakpoint count mismatch"
        );
        for (actual_bp, expected_bp) in actual_freq.iter().zip(expected.frequency.iter()) {
            assert_freq_bp_approx_eq(actual_bp, expected_bp);
        }
    }

    fn check_from_parametric(clip: Clip, expected: TestClip) {
        let result = from_parametric(&clip).unwrap();
        assert_haptic_data_eq(&result, &expected);
    }

    #[test]
    fn basic_conversion_without_transients() {
        check_from_parametric(
            Clip {
                amplitude_points: vec![point(0.0, 0.2), point(0.5, 0.8), point(1.0, 0.3)],
                frequency_points: vec![point(0.0, 0.5), point(1.0, 0.7)],
                transients: vec![],
            },
            TestClip {
                amplitude: &[amp_bp(0.0, 0.2), amp_bp(0.5, 0.8), amp_bp(1.0, 0.3)],
                frequency: &[freq_bp(0.0, 0.5), freq_bp(1.0, 0.7)],
            },
        );
    }

    #[test]
    fn transient_at_existing_amplitude_point() {
        check_from_parametric(
            Clip {
                amplitude_points: vec![point(0.0, 0.2), point(0.5, 0.5), point(1.0, 0.3)],
                frequency_points: vec![],
                transients: vec![transient(0.5, 0.8, 0.6)],
            },
            TestClip {
                amplitude: &[
                    amp_bp(0.0, 0.2),
                    emphasis_bp(0.5, 0.5, 0.8, 0.6),
                    amp_bp(1.0, 0.3),
                ],
                frequency: &[],
            },
        );
    }

    #[test]
    fn transient_between_amplitude_points() {
        check_from_parametric(
            Clip {
                amplitude_points: vec![point(0.0, 0.2), point(1.0, 0.6)],
                frequency_points: vec![],
                transients: vec![transient(0.5, 0.8, 0.7)],
            },
            TestClip {
                amplitude: &[
                    amp_bp(0.0, 0.2),
                    emphasis_bp(0.5, 0.4, 0.8, 0.7),
                    amp_bp(1.0, 0.6),
                ],
                frequency: &[],
            },
        );
    }

    #[test]
    fn multiple_transients() {
        check_from_parametric(
            Clip {
                amplitude_points: vec![point(0.0, 0.0), point(1.0, 1.0)],
                frequency_points: vec![],
                transients: vec![transient(0.25, 0.5, 0.5), transient(0.75, 0.9, 0.8)],
            },
            TestClip {
                amplitude: &[
                    amp_bp(0.0, 0.0),
                    emphasis_bp(0.25, 0.25, 0.5, 0.5),
                    emphasis_bp(0.75, 0.75, 0.9, 0.8),
                    amp_bp(1.0, 1.0),
                ],
                frequency: &[],
            },
        );
    }

    #[test]
    fn no_frequency_points() {
        check_from_parametric(
            Clip {
                amplitude_points: vec![point(0.0, 0.2), point(1.0, 0.3)],
                frequency_points: vec![],
                transients: vec![],
            },
            TestClip {
                amplitude: &[amp_bp(0.0, 0.2), amp_bp(1.0, 0.3)],
                frequency: &[],
            },
        );
    }

    #[test]
    fn transient_within_1ms_of_amplitude_point_merges() {
        // Transient is 0.5ms away from amplitude point at 0.5
        // Should still have 3 breakpoints (transient merged with existing point)
        // The amplitude point at 0.5 should have the emphasis attached
        check_from_parametric(
            Clip {
                amplitude_points: vec![point(0.0, 0.2), point(0.5, 0.5), point(1.0, 0.3)],
                frequency_points: vec![],
                transients: vec![transient(0.5005, 0.8, 0.6)],
            },
            TestClip {
                amplitude: &[
                    amp_bp(0.0, 0.2),
                    emphasis_bp(0.5, 0.5, 0.8, 0.6),
                    amp_bp(1.0, 0.3),
                ],
                frequency: &[],
            },
        );
    }

    #[test]
    fn transient_beyond_1ms_creates_new_point() {
        // Transient is 2ms away from amplitude point at 0.5
        // Should have 4 breakpoints (new point created for transient)
        check_from_parametric(
            Clip {
                amplitude_points: vec![point(0.0, 0.2), point(0.5, 0.5), point(1.0, 0.3)],
                frequency_points: vec![],
                transients: vec![transient(0.502, 0.8, 0.6)],
            },
            TestClip {
                amplitude: &[
                    amp_bp(0.0, 0.2),
                    amp_bp(0.5, 0.5),
                    emphasis_bp(0.502, 0.4992, 0.8, 0.6),
                    amp_bp(1.0, 0.3),
                ],
                frequency: &[],
            },
        );
    }

    #[test]
    fn multiple_transients_near_same_point_last_wins() {
        // Two transients both within 1ms of amplitude point at 0.5
        // Should still have 3 breakpoints (both transients merged with same point)
        // Last transient wins
        check_from_parametric(
            Clip {
                amplitude_points: vec![point(0.0, 0.2), point(0.5, 0.5), point(1.0, 0.3)],
                frequency_points: vec![],
                transients: vec![transient(0.4995, 0.7, 0.5), transient(0.5005, 0.9, 0.8)],
            },
            TestClip {
                amplitude: &[
                    amp_bp(0.0, 0.2),
                    emphasis_bp(0.5, 0.5, 0.9, 0.8),
                    amp_bp(1.0, 0.3),
                ],
                frequency: &[],
            },
        );
    }

    #[test]
    fn transient_after_last_amplitude_point_returns_error() {
        let clip = Clip {
            amplitude_points: vec![point(0.0, 0.2), point(1.0, 0.3)],
            frequency_points: vec![],
            transients: vec![transient(1.5, 0.8, 0.6)],
        };

        let result = from_parametric(&clip);

        assert!(matches!(
            result,
            Err(FromParametricError::TransientAfterLastAmplitudePoint { .. })
        ));
    }

    #[test]
    fn validation_failure_returns_error() {
        let clip = Clip {
            amplitude_points: vec![point(0.0, 0.5), point(1.0, 1.1)],
            frequency_points: vec![],
            transients: vec![],
        };

        let result = from_parametric(&clip);

        assert!(matches!(
            result,
            Err(FromParametricError::ValidationError(
                v1::ValidationError::AmplitudeOutOfRange { .. }
            ))
        ));
    }
}
