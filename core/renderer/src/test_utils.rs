// (c) Meta Platforms, Inc. and affiliates. Confidential and proprietary.

//! Utilities for working with streaming events in tests

use crate::StreamingEvent;
use crate::StreamingEventType;
use crate::StreamingRamp;

/// Creates an amplitude ramp without emphasis
pub fn amp_ramp(time: f32, start: f32, target: f32, duration: f32) -> StreamingEvent {
    StreamingEvent {
        time,
        event: StreamingEventType::AmplitudeRamp(StreamingRamp {
            start,
            target,
            duration,
        }),
    }
}

/// Creates an amplitude ramp with emphasis
pub fn emphasis_event(time: f32, amplitude: f32, frequency: f32) -> StreamingEvent {
    StreamingEvent {
        time,
        event: StreamingEventType::Emphasis {
            amplitude,
            frequency,
        },
    }
}

/// Creates an frequency ramp
pub fn freq_ramp(time: f32, start: f32, target: f32, duration: f32) -> StreamingEvent {
    StreamingEvent {
        time,
        event: StreamingEventType::FrequencyRamp(StreamingRamp {
            start,
            target,
            duration,
        }),
    }
}

/// Asserts that two slices of ramp events are equivalent
#[track_caller]
pub fn compare_ramp_event_slices(
    expected_events: &[StreamingEvent],
    actual_events: &[StreamingEvent],
) {
    for (i, (expected, actual)) in expected_events.iter().zip(actual_events.iter()).enumerate() {
        compare_ramp_events(*expected, *actual, i);
    }

    assert_eq!(
        expected_events.len(),
        actual_events.len(),
        "Mismatch in number of ramp events"
    );
}

/// Asserts that two ramp events are equivalent
#[track_caller]
pub fn compare_ramp_events(expected: StreamingEvent, actual: StreamingEvent, index: usize) {
    use haptic_dsp::test_utils::is_near;

    let allowed_error = 1.0e-6;

    if !is_near(expected.time, actual.time, allowed_error) {
        panic!("Time mismatch in event {index}:\n  expected: {expected:?}\n  actual: {actual:?}",)
    }

    use StreamingEventType::*;
    match (expected.event, actual.event) {
        (
            AmplitudeRamp(StreamingRamp {
                start: expected_amplitude_start,
                target: expected_amplitude_target,
                duration: expected_duration,
            }),
            AmplitudeRamp(StreamingRamp {
                start: actual_amplitude_start,
                target: actual_amplitude_target,
                duration: actual_duration,
            }),
        ) => {
            if !is_near(expected_duration, actual_duration, allowed_error) {
                panic!(
                    "Duration mismatch in event {index}:\n  expected: {expected:?}\n  actual: {actual:?}",
                )
            }

            if !is_near(
                expected_amplitude_start,
                actual_amplitude_start,
                allowed_error,
            ) {
                panic!(
                    "Amplitude start mismatch in event {index}:\n  expected: {expected:?}\n  actual: {actual:?}",
                )
            }

            if !is_near(
                expected_amplitude_target,
                actual_amplitude_target,
                allowed_error,
            ) {
                panic!(
                    "Amplitude target mismatch in event {index}:\n  expected: {expected:?}\n  actual: {actual:?}",
                )
            }
        }
        (
            FrequencyRamp(StreamingRamp {
                start: expected_frequency_start,
                target: expected_frequency_target,
                duration: expected_duration,
            }),
            FrequencyRamp(StreamingRamp {
                start: actual_frequency_start,
                target: actual_frequency_target,
                duration: actual_duration,
            }),
        ) => {
            if !is_near(expected_duration, actual_duration, allowed_error) {
                panic!(
                    "Duration mismatch in event {index}:\n  expected: {expected:?}\n  actual: {actual:?}",
                )
            }

            if !is_near(
                expected_frequency_start,
                actual_frequency_start,
                allowed_error,
            ) {
                panic!(
                    "Frequency start mismatch in event {index}:\n  expected: {expected:?}\n  actual: {actual:?}",
                )
            }

            if !is_near(
                expected_frequency_target,
                actual_frequency_target,
                allowed_error,
            ) {
                panic!(
                    "Frequency target mismatch in event {index}:\n  expected: {expected:?}\n  actual: {actual:?}",
                )
            }
        }
        (
            Emphasis {
                amplitude: expected_amplitude,
                frequency: expected_frequency,
            },
            Emphasis {
                amplitude: actual_amplitude,
                frequency: actual_frequency,
            },
        ) => {
            if !is_near(expected_amplitude, actual_amplitude, allowed_error) {
                panic!(
                    "Amplitude mismatch in event {index}:\n  expected: {expected:?}\n  actual: {actual:?}",
                )
            }

            if !is_near(expected_frequency, actual_frequency, allowed_error) {
                panic!(
                    "Frequency mismatch in event {index}:\n  expected: {expected:?}\n  actual: {actual:?}",
                )
            }
        }
        _ => {
            panic!(
                "Event type mismatch in event {index}:\n  expected: {expected:?}\n  actual: {actual:?}",
            )
        }
    }
}

/// Compares two slices of f32 values, asserting that they are approximately equal
pub fn approx_compare_slices(a: &[f32], b: &[f32]) {
    let allowed_error = 0.001;
    assert_eq!(a.len(), b.len());
    for (&val_a, &val_b) in a.iter().zip(b.iter()) {
        if (val_a - val_b).abs() > allowed_error {
            panic!("approx_compare_slices failed:\n  a: {a:?}\n  b: {b:?}");
        }
    }
}
