// (c) Meta Platforms, Inc. and affiliates. Confidential and proprietary.

use std::ops::Deref;

use haptic_data::interpolate_breakpoints;
use haptic_data::v1::AmplitudeBreakpoint;
use haptic_data::v1::Envelopes;
use haptic_data::v1::FrequencyBreakpoint;
use haptic_data::v1::HapticData;
use itertools::PeekingNext;

use crate::StreamingEvent;
use crate::StreamingEventType;
use crate::StreamingRamp;

/// Reads haptic breakpoints from a haptic clip as a combined series of ramp events
///
/// The transformation from breakpoints to ramp events is useful if you want to work with the
/// changing amplitude and frequency curves of a haptic clip over time.
///
/// Breakpoints are read from the clip in the following form:
///   - Amplitude and frequency breakpoints are read out mixed together in sequential order.
///   - Each breakpoint is translated into a ramp event.
///     - This means that for breakpoints at times [0, 1, 2] the first breakpoint at time 0 will be
///       read out immediately as a ramp event with a duration of 0,
///       (i.e. 'jump immediately to this value), followed by another ramp event at time 0 with the
///       value of the breakpoint at time 1, and a duration that represents the gap between the
///       breakpoints at time 0 and 1.
///     - ...or to put it another way, for breakpoints
///       {time: 0, value: 1}, {time: 1, value: 2}, {time: 3, value: 4}, {time: 3, value: 0}
///       the following ramp events will be produced:
///       {time: 0, value: 1, duration: 0}
///       {time: 0, value: 2, duration: 1}
///       {time: 1, value: 4, duration: 2}
///       {time: 3, value: 0, duration: 0}
///   - The last breakpoint of the amplitude envelope is read out as a zero-duration ramp with an
///     amplitude of zero.
///   - The last breakpoint of the frequency envelope is read out as a zero-duration ramp with the
///     final frequency value.
///   - When amplitude and frequency events share the same time, the amplitude events will be read
///     first.
///   - Emphasis is returned as a separate event type (it's attached to amplitude breakpoints in
///     the haptic data model).
///
/// The reader keeps track of its position in the clip, and then provides the amount of time from
/// the current position to the next event in the haptic clip, which may be an amplitude or
/// frequency event.
///
/// Seeking to a new playback position is available.
///   - If no events are at the new playback time, then interpolated events will be produced to
///     represent the ramps starting from the new time.
///   - When seeking to a new time that has multiple events, the events will not be skipped and will
///     be read out in order.
///   - When seeking to a negative time, a jump event will be read out with the negative seek time
///     and amplitude zero.
///   - Seeking past the end of the clip will cause playback to stop immediately,
///     unless looping is enabled (see below).
///
/// Looped playback is available.
///   - The start of the loop is at the first amplitude breakpoint
///   - The end of the loop is at the last amplitude breakpoint
///   - A running offset is applied to the breakpoint times. Each loop iteration increases the
///     offset by the loop's duration.
///   - Seeking to before the first amplitude breakpoint will act like a negative seek, with a zero
///     amplitude flatline before the envelope activates.
///   - Seeking past the end of the clip will cause an immediate jump to the start of the loop.
///   - If the amplitude envelope has finished, then enabling looping will cause the envelope to
///     restart from the beginning of the loop.
///
/// The reader implements `Iterator`, with next events being accessed via `.next()`.
///
/// The reader also implements itertools' `PeekingNext` trait, which enables the use of
/// `peeking_take_while`, which is useful for consuming events that meet a condition.
/// Q. Why not just use `Peekable`?
/// A. If you want to peek events, and also call functions on the event reader like `seek` or
///    `set_looping_enabled`, then you're stuck because `Peekable` doesn't provide access to the
///    underlying iterator.
#[derive(Debug, Copy, Clone)]
pub struct StreamingEventReader<H> {
    // The currently played haptic clip
    haptic_data: H,
    // Whether or not an amplitude ramp is currently active
    amplitude_state: EnvelopeState<AmplitudeBreakpoint>,
    // Whether or not a frequency ramp is currently active
    frequency_state: EnvelopeState<FrequencyBreakpoint>,
    // An optional emphasis event that should be returned as the next event
    pending_emphasis_event: Option<StreamingEvent>,
    // Used by the `next_if` / `peek` functions
    peeked_event: Option<StreamingEvent>,
    // An offset to apply to event time values, used during looped playback
    time_offset: f32,
    // Some when looped playback is enabled, None when it's disabled
    loop_info: Option<LoopInfo>,
}

impl<H> StreamingEventReader<H>
where
    H: Deref<Target = HapticData>,
{
    /// Makes a new StreamingEventReader for the provided haptic clip
    pub fn new(haptic_data: H) -> Self {
        let mut result = Self {
            haptic_data,
            amplitude_state: EnvelopeState::Finished,
            frequency_state: EnvelopeState::Finished,
            pending_emphasis_event: None,
            peeked_event: None,
            time_offset: 0.0,
            loop_info: None,
        };
        result.seek(0.0);
        result
    }

    /// Moves the internal playback position to the provided time
    ///
    /// If no events are present at the playback time then interpolated events will be produced to
    /// represent the ramps starting from the new time.
    pub fn seek(&mut self, seek_time: f32) {
        // Find the index of the first amplitude event at or after the seek time
        let next_index = self
            .envelopes()
            .amplitude
            .partition_point(|bp| bp.time < seek_time);

        self.amplitude_state = match self.amplitude_breakpoint(next_index) {
            Some(next_bp) if seek_time == next_bp.time => {
                // If there's a breakpoint at the seek time then we don't need a seek
                // breakpoint, we can just start playback from the matching breakpoint.
                EnvelopeState::Start { index: next_index }
            }
            Some(next_bp) if seek_time < next_bp.time => {
                if next_index == 0 {
                    // The first bp is after the seek time, so output an event with zero amplitude
                    EnvelopeState::SeekStart {
                        seek_bp: AmplitudeBreakpoint {
                            time: seek_time,
                            amplitude: 0.0,
                            emphasis: None,
                        },
                        next_index: 0,
                    }
                } else {
                    // The seek time is between two breakpoints, so create an interpolated breakpoint
                    match self.amplitude_breakpoint(next_index - 1) {
                        Some(previous_bp) => EnvelopeState::SeekStart {
                            seek_bp: interpolate_breakpoints(&previous_bp, &next_bp, seek_time),
                            next_index,
                        },
                        None => {
                            // This branch is guaranteed to be unreachable:
                            // Next_index is a valid index greater than 0,
                            // so (next_index - 1) is also a valid index.
                            debug_assert!(false);
                            // In the unlikely event that something very wrong is happening,
                            // stop playback immediately.
                            EnvelopeState::Finished
                        }
                    }
                }
            }
            _ => {
                // The seek time is after the last breakpoint, so either restart the loop or finish
                if self.looping_enabled() {
                    EnvelopeState::Start { index: 0 }
                } else {
                    EnvelopeState::Finished
                }
            }
        };

        if let Some(frequency_envelope) = &self.haptic_data.signals.continuous.envelopes.frequency {
            // Find the index of the first frequency event at or after the seek time
            let next_index = frequency_envelope.partition_point(|bp| bp.time < seek_time);

            self.frequency_state = match self.frequency_breakpoint(next_index) {
                Some(next_bp) if seek_time == next_bp.time => {
                    // If there's a breakpoint at the seek time then we don't need a seek
                    // breakpoint, we can just start playback from the matching breakpoint.
                    EnvelopeState::Start { index: next_index }
                }
                Some(next_bp) if seek_time < next_bp.time => {
                    if next_index == 0 {
                        // The first bp is after the seek time, so start playback at index 0
                        EnvelopeState::Start { index: 0 }
                    } else {
                        // The seek time is between two breakpoints, so create an interpolated
                        // breakpoint
                        match self.frequency_breakpoint(next_index - 1) {
                            Some(previous_bp) => EnvelopeState::SeekStart {
                                seek_bp: interpolate_breakpoints(&previous_bp, &next_bp, seek_time),
                                next_index,
                            },
                            None => {
                                // This branch is guaranteed to be unreachable:
                                // next_index is a valid index greater than 0,
                                // so (next_index - 1) is also a valid index.
                                debug_assert!(false);
                                // In the unlikely event that something very wrong is happening,
                                // stop the frequency envelope immediately
                                EnvelopeState::Finished
                            }
                        }
                    }
                }
                _ => {
                    // The seek time is after the last breakpoint,
                    // so either restart the envelope or finish
                    if self.looping_enabled() {
                        EnvelopeState::Start { index: 0 }
                    } else {
                        EnvelopeState::Finished
                    }
                }
            }
        }
    }

    /// Enables or disables looped playback
    pub fn set_looping_enabled(&mut self, enabled: bool) {
        self.loop_info = if enabled {
            let amp_envelope = &self.envelopes().amplitude;
            match (amp_envelope.first(), amp_envelope.last()) {
                (Some(first_bp), Some(last_bp)) => {
                    let duration = last_bp.time - first_bp.time;
                    if duration > 0.0 {
                        Some(LoopInfo {
                            start_time: first_bp.time,
                            duration,
                        })
                    } else {
                        // We can only get here with invalid haptic data
                        debug_assert!(false);
                        None
                    }
                }
                _ => {
                    // We can only get here with invalid haptic data
                    debug_assert!(false);
                    None
                }
            }
        } else {
            None
        }
    }

    /// Returns true when looping is enabled
    pub fn looping_enabled(&self) -> bool {
        self.loop_info.is_some()
    }

    fn handle_looping_at_end(&mut self) -> Option<StreamingEvent> {
        if let Some(loop_info) = &self.loop_info {
            // Adjust the time offset to match the start of the next loop iteration
            self.time_offset += loop_info.duration;
            // Seek to the start of the loop. This will set the amplitude envelope
            // to Start { index: 0 }, and initialize the frequency envelope
            // correctly).
            self.seek(loop_info.start_time);
            // Return the amplitude event resulting from the seek.
            self.next_amplitude_ramp()
        } else {
            None
        }
    }

    /// Returns the next event without consuming it
    pub fn peek(&mut self) -> Option<StreamingEvent> {
        if let Some(peeked_event) = self.peeked_event {
            Some(peeked_event)
        } else {
            self.peeked_event = self.next_event();
            self.peeked_event
        }
    }

    /// Consumes and returns the next event if the provided condition is true
    pub fn next_if<F>(&mut self, accept: F) -> Option<StreamingEvent>
    where
        F: FnOnce(&StreamingEvent) -> bool,
    {
        if let Some(event) = self.peek() {
            if accept(&event) {
                self.peeked_event = None;
                Some(event)
            } else {
                None
            }
        } else {
            None
        }
    }

    fn envelopes(&self) -> &Envelopes {
        &self.haptic_data.signals.continuous.envelopes
    }

    fn amplitude_breakpoint(&self, index: usize) -> Option<AmplitudeBreakpoint> {
        if self.loop_info.is_none()
            && index == self.envelopes().amplitude.len()
            && let Some(amp_prev_bp) = self.envelopes().amplitude.get(index - 1).cloned()
            && amp_prev_bp.emphasis.is_some()
        {
            // If the final amplitude breakpoint has emphasis and we are not looping
            // we need to tag on an additional amplitude breakpoint to give the emphasis
            // time to play out *with* the amplitude of the final breakpoint held for that
            // short time duration and then the tagged on breakpoint will finalise the clip.
            return Some(AmplitudeBreakpoint {
                time: amp_prev_bp.time + 0.1,
                amplitude: amp_prev_bp.amplitude,
                emphasis: None,
            });
        }

        self.envelopes().amplitude.get(index).cloned()
    }

    fn frequency_breakpoint(&self, index: usize) -> Option<FrequencyBreakpoint> {
        self.envelopes()
            .frequency
            // The frequency envelope is optional
            .as_ref()
            .and_then(|envelope| envelope.get(index).cloned())
    }

    fn next_amplitude_ramp(&mut self) -> Option<StreamingEvent> {
        use EnvelopeState::*;

        self.pending_emphasis_event = None;

        let (start_bp, end_bp, new_state) = match self.amplitude_state {
            Start { index } => {
                // The envelope hasn't started yet, so jump to the initial value
                match self.amplitude_breakpoint(index) {
                    Some(first_bp) => (first_bp, first_bp, Active { index }),
                    None => return None,
                }
            }
            Active { index } => {
                // The envelope is active, so advance to the next event to get the end bp
                let end_index = index + 1;
                match (
                    self.amplitude_breakpoint(index),
                    self.amplitude_breakpoint(end_index),
                ) {
                    (Some(start_bp), Some(end_bp)) => {
                        if let Some(emphasis) = start_bp.emphasis {
                            self.pending_emphasis_event = Some(StreamingEvent {
                                time: self.time_offset + start_bp.time,
                                event: StreamingEventType::Emphasis {
                                    amplitude: emphasis.amplitude,
                                    frequency: emphasis.frequency,
                                },
                            });
                        }

                        (start_bp, end_bp, Active { index: end_index })
                    }
                    (Some(start_bp), None) => {
                        match self.handle_looping_at_end() {
                            Some(next_event) => return Some(next_event),
                            None => {
                                // The last breakpoint has been reached, so jump to zero amplitude
                                let end_bp = AmplitudeBreakpoint {
                                    time: start_bp.time,
                                    amplitude: 0.0,
                                    emphasis: None,
                                };
                                (start_bp, end_bp, Finished)
                            }
                        }
                    }
                    _ => return None,
                }
            }
            SeekStart {
                seek_bp,
                next_index,
            } => {
                let new_state = if next_index > 0 {
                    SeekRamp {
                        seek_bp,
                        next_index,
                    }
                } else {
                    Start { index: 0 }
                };
                (seek_bp, seek_bp, new_state)
            }
            SeekRamp {
                seek_bp,
                next_index,
            } => {
                match self.amplitude_breakpoint(next_index) {
                    Some(end_bp) => (seek_bp, end_bp, Active { index: next_index }),
                    None => {
                        // This should be unreachable, when there's a seek event then there should
                        // be a subsequent end ramp breakpoint
                        debug_assert!(false);
                        return None;
                    }
                }
            }
            Finished => return None,
        };

        self.amplitude_state = new_state;

        Some(StreamingEvent {
            time: self.time_offset + start_bp.time,
            event: StreamingEventType::AmplitudeRamp(StreamingRamp::from_breakpoints(
                &start_bp, &end_bp,
            )),
        })
    }

    fn next_frequency_ramp(&mut self) -> Option<StreamingEvent> {
        use EnvelopeState::*;

        let (start_bp, end_bp, new_state) = match self.frequency_state {
            Start { index } => {
                // The envelope hasn't started yet, so jump to the initial value
                match self.frequency_breakpoint(index) {
                    Some(first_bp) => (first_bp, first_bp, Active { index }),
                    None => return None,
                }
            }
            Active { index } => {
                // The envelope is active, so advance to the next event to get the end bp
                let end_index = index + 1;
                match (
                    self.frequency_breakpoint(index),
                    self.frequency_breakpoint(end_index),
                ) {
                    (Some(start_bp), Some(end_bp)) => {
                        (start_bp, end_bp, Active { index: end_index })
                    }
                    (Some(start_bp), None) => {
                        // The last breakpoint has been reached, so return a zero duration event at
                        // the last value, telling the caller to maintain the final frequency.
                        // Note that looping is handled when reading out the amplitude envelope.
                        (start_bp, start_bp, Finished)
                    }
                    _ => return None,
                }
            }
            SeekStart {
                seek_bp,
                next_index,
            } => {
                let new_state = if seek_bp.time >= 0.0 {
                    SeekRamp {
                        seek_bp,
                        next_index,
                    }
                } else {
                    Start { index: 0 }
                };
                (seek_bp, seek_bp, new_state)
            }
            SeekRamp {
                seek_bp,
                next_index,
            } => {
                match self.frequency_breakpoint(next_index) {
                    Some(end_bp) => (seek_bp, end_bp, Active { index: next_index }),
                    None => {
                        // This should be unreachable, when there's a seek event then there should
                        // be a subsequent end ramp breakpoint
                        debug_assert!(false);
                        return None;
                    }
                }
            }
            Finished => return None,
        };

        self.frequency_state = new_state;

        Some(StreamingEvent {
            time: self.time_offset + start_bp.time,
            event: StreamingEventType::FrequencyRamp(StreamingRamp::from_breakpoints(
                &start_bp, &end_bp,
            )),
        })
    }

    fn next_event(&mut self) -> Option<StreamingEvent> {
        use EnvelopeState::*;

        if let Some(peeked_event) = self.peeked_event.take() {
            Some(peeked_event)
        } else if let Some(pending_emphasis_event) = self.pending_emphasis_event.take() {
            Some(pending_emphasis_event)
        } else {
            match (self.amplitude_state, self.frequency_state) {
                // The amplitude envelope is at the end, but looping might be enabled
                (Finished, _) => {
                    // We check the looping state here due to an issue with the StreamingEventReader
                    // and the "peeking" functionality: When setting looping to true *after* having
                    // peeked the last event already, the state in StreamingEventReader has already
                    // advanced and is otherwise incorrect.
                    self.handle_looping_at_end()
                }
                // Seek has been performed on the amplitude envelope
                (SeekStart { .. } | SeekRamp { .. }, _) => self.next_amplitude_ramp(),
                // Seek has been performed on the frequency envelope
                (_, SeekStart { .. } | SeekRamp { .. }) => self.next_frequency_ramp(),
                // The amplitude envelope is starting/active, but frequency is finished
                (Start { .. } | Active { .. }, Finished) => self.next_amplitude_ramp(),
                // The amplitude and frequency envelopes are both starting/active
                (
                    Start { index: amp_index } | Active { index: amp_index },
                    Start { index: freq_index } | Active { index: freq_index },
                ) => {
                    match (
                        self.amplitude_breakpoint(amp_index),
                        self.frequency_breakpoint(freq_index),
                    ) {
                        (Some(amp_bp), Some(freq_bp)) => {
                            if amp_bp.time <= freq_bp.time {
                                self.next_amplitude_ramp()
                            } else {
                                self.next_frequency_ramp()
                            }
                        }
                        _ => {
                            // We should never reach here: the only way we can have a Start or
                            // Active state is when we have a valid envelope index.
                            debug_assert!(false);
                            // Rather than panic, simply terminate the iteration
                            None
                        }
                    }
                }
            }
        }
    }
}

impl<H> Iterator for StreamingEventReader<H>
where
    H: Deref<Target = HapticData>,
{
    type Item = StreamingEvent;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_event()
    }
}

impl<H> PeekingNext for StreamingEventReader<H>
where
    H: Deref<Target = HapticData>,
{
    fn peeking_next<F>(&mut self, accept: F) -> Option<Self::Item>
    where
        F: FnOnce(&Self::Item) -> bool,
    {
        self.next_if(accept)
    }
}

// The state of the amplitude or frequency envelope
#[derive(Debug, Copy, Clone)]
enum EnvelopeState<Breakpoint> {
    // The envelope hasn't been started yet.
    // - The first event will be a jump (zero-duration ramp) to the start of a ramp.
    // - The next event will be the start of the ramp to the following breakpoint.
    // - The start index can be at any position in the envelope due to seek operations.
    Start {
        index: usize,
    },
    // The envelope is active. The next breakpoint will be read out as the start of the next ramp,
    // with the following breakpoint representing the end point. If no following breakpoint is
    // present then the end of the envelope has been reached and no more ramps will be read out.
    Active {
        index: usize,
    },
    // A seek operation has been performed, and the next event will be a jump to the start of the
    // seek ramp.
    SeekStart {
        seek_bp: Breakpoint,
        next_index: usize,
    },
    // The previous event was the seek start, and the next event will be a ramp towards the first
    // breakpoint following the seek time.
    SeekRamp {
        seek_bp: Breakpoint,
        next_index: usize,
    },
    // The reader is past the end of the envelope, there are no more events to read.
    Finished,
}

// Values that get cached when looping is enabled
#[derive(Debug, Copy, Clone)]
struct LoopInfo {
    // The loop's start time
    //
    // Currently this will always match the start of the amp envelope.
    start_time: f32,
    // The loop's duration
    //
    // Currently this will always match the duration between the start and end of the amp envelope.
    duration: f32,
}

#[cfg(test)]
mod tests {
    use haptic_data::test_utils::*;

    use self::test_utils::*;
    use super::*;
    use crate::test_utils::*;

    mod simple_ramps {
        use super::*;

        #[test]
        fn amplitude_envelope() {
            check_reader_output(
                TestClip {
                    amplitude: &[amp_bp(0.0, 5.0), amp_bp(1.0, 6.0), amp_bp(4.0, 7.0)],
                    frequency: &[],
                },
                &[
                    amp_ramp(0.0, 5.0, 5.0, 0.0),
                    amp_ramp(0.0, 5.0, 6.0, 1.0),
                    amp_ramp(1.0, 6.0, 7.0, 3.0),
                    amp_ramp(4.0, 7.0, 0.0, 0.0),
                ],
            );
        }

        #[test]
        fn amplitude_and_frequency_envelope() {
            check_reader_output(
                TestClip {
                    amplitude: &[amp_bp(0.0, 3.0), amp_bp(1.0, 9.0), amp_bp(3.0, 0.0)],
                    frequency: &[freq_bp(0.0, 5.0), freq_bp(2.0, 6.0), freq_bp(3.0, 7.0)],
                },
                &[
                    amp_ramp(0.0, 3.0, 3.0, 0.0),
                    amp_ramp(0.0, 3.0, 9.0, 1.0),
                    freq_ramp(0.0, 5.0, 5.0, 0.0),
                    freq_ramp(0.0, 5.0, 6.0, 2.0),
                    amp_ramp(1.0, 9.0, 0.0, 2.0),
                    freq_ramp(2.0, 6.0, 7.0, 1.0),
                    amp_ramp(3.0, 0.0, 0.0, 0.0),
                ],
            );
        }

        #[test]
        fn amplitude_envelope_with_emphasis_on_first_event() {
            check_reader_output(
                TestClip {
                    amplitude: &[
                        emphasis_bp(0.0, 2.0, 3.0, 4.0),
                        amp_bp(2.0, 8.0),
                        amp_bp(3.0, 4.0),
                    ],
                    frequency: &[],
                },
                &[
                    amp_ramp(0.0, 2.0, 2.0, 0.0),
                    amp_ramp(0.0, 2.0, 8.0, 2.0),
                    emphasis_event(0.0, 3.0, 4.0),
                    amp_ramp(2.0, 8.0, 4.0, 1.0),
                    amp_ramp(3.0, 4.0, 0.0, 0.0),
                ],
            );
        }

        #[test]
        fn amplitude_and_frequency_envelope_with_emphasis_on_second_event() {
            check_reader_output(
                TestClip {
                    amplitude: &[
                        amp_bp(0.0, 2.0),
                        emphasis_bp(1.0, 5.0, 2.0, 0.5),
                        amp_bp(3.0, 4.0),
                    ],
                    frequency: &[freq_bp(0.0, 5.0), freq_bp(3.0, 7.0)],
                },
                &[
                    amp_ramp(0.0, 2.0, 2.0, 0.0),
                    amp_ramp(0.0, 2.0, 5.0, 1.0),
                    freq_ramp(0.0, 5.0, 5.0, 0.0),
                    freq_ramp(0.0, 5.0, 7.0, 3.0),
                    amp_ramp(1.0, 5.0, 4.0, 2.0),
                    emphasis_event(1.0, 2.0, 0.5),
                    amp_ramp(3.0, 4.0, 0.0, 0.0),
                ],
            );
        }

        #[test]
        fn amplitude_envelope_with_emphasis_on_last_event() {
            check_reader_output(
                TestClip {
                    amplitude: &[
                        amp_bp(0.0, 2.0),
                        amp_bp(2.0, 8.0),
                        emphasis_bp(3.0, 4.0, 2.0, 0.5),
                    ],
                    frequency: &[],
                },
                &[
                    amp_ramp(0.0, 2.0, 2.0, 0.0),
                    amp_ramp(0.0, 2.0, 8.0, 2.0),
                    amp_ramp(2.0, 8.0, 4.0, 1.0),
                    amp_ramp(3.0, 4.0, 4.0, 0.1),
                    emphasis_event(3.0, 2.0, 0.5),
                    amp_ramp(3.1, 4.0, 0.0, 0.0),
                ],
            );
        }

        #[test]
        fn amplitude_envelope_with_single_event() {
            // A single-event amplitude envelope is likely to be an input error,
            // but this documents the resulting behaviour.
            check_reader_output(
                TestClip {
                    amplitude: &[amp_bp(1.0, 6.0)],
                    frequency: &[],
                },
                &[
                    amp_ramp(0.0, 0.0, 0.0, 0.0),
                    amp_ramp(1.0, 6.0, 6.0, 0.0),
                    amp_ramp(1.0, 6.0, 0.0, 0.0),
                ],
            );
        }

        #[test]
        fn frequency_envelope_with_single_event() {
            check_reader_output(
                TestClip {
                    amplitude: &[amp_bp(0.0, 3.0), amp_bp(1.0, 9.0), amp_bp(3.0, 0.0)],
                    frequency: &[freq_bp(2.0, 6.0)],
                },
                &[
                    amp_ramp(0.0, 3.0, 3.0, 0.0),
                    amp_ramp(0.0, 3.0, 9.0, 1.0),
                    amp_ramp(1.0, 9.0, 0.0, 2.0),
                    freq_ramp(2.0, 6.0, 6.0, 0.0),
                    freq_ramp(2.0, 6.0, 6.0, 0.0),
                    amp_ramp(3.0, 0.0, 0.0, 0.0),
                ],
            );
        }
    }

    mod seek {
        use super::*;

        #[test]
        fn amplitude_envelope_with_seek() {
            check_reader_output_after_seek(
                TestClip {
                    amplitude: &[amp_bp(0.0, 5.0), amp_bp(1.0, 6.0), amp_bp(4.0, 7.0)],
                    frequency: &[],
                },
                0.5,
                &[
                    amp_ramp(0.5, 5.5, 5.5, 0.0),
                    amp_ramp(0.5, 5.5, 6.0, 0.5),
                    amp_ramp(1.0, 6.0, 7.0, 3.0),
                    amp_ramp(4.0, 7.0, 0.0, 0.0),
                ],
            );
        }

        #[test]
        fn amplitude_and_frequency_envelopes_with_seek() {
            check_reader_output_after_seek(
                TestClip {
                    amplitude: &[amp_bp(0.0, 5.0), amp_bp(1.0, 1.0), amp_bp(4.0, 7.0)],
                    frequency: &[freq_bp(0.0, 3.0), freq_bp(1.5, 7.0), freq_bp(3.0, 1.0)],
                },
                0.75,
                &[
                    amp_ramp(0.75, 2.0, 2.0, 0.0),
                    amp_ramp(0.75, 2.0, 1.0, 0.25),
                    freq_ramp(0.75, 5.0, 5.0, 0.0),
                    freq_ramp(0.75, 5.0, 7.0, 0.75),
                    amp_ramp(1.0, 1.0, 7.0, 3.0),
                    freq_ramp(1.5, 7.0, 1.0, 1.5),
                    freq_ramp(3.0, 1.0, 1.0, 0.0),
                    amp_ramp(4.0, 7.0, 0.0, 0.0),
                ],
            );
        }

        #[test]
        fn amplitude_and_frequency_envelopes_with_two_seeks() {
            let clip = TestClip {
                amplitude: &[amp_bp(0.0, 5.0), amp_bp(1.0, 1.0), amp_bp(5.0, 2.0)],
                frequency: &[freq_bp(0.0, 3.0), freq_bp(2.0, 7.0), freq_bp(3.0, 1.0)],
            };
            let data = clip.into();
            let mut reader = StreamingEventReader::new(&data);

            reader.seek(0.5);
            compare_ramp_events(amp_ramp(0.5, 3.0, 3.0, 0.0), reader.next().unwrap(), 0);
            compare_ramp_events(amp_ramp(0.5, 3.0, 1.0, 0.5), reader.next().unwrap(), 1);
            compare_ramp_events(freq_ramp(0.5, 4.0, 4.0, 0.0), reader.next().unwrap(), 2);
            compare_ramp_events(freq_ramp(0.5, 4.0, 7.0, 1.5), reader.next().unwrap(), 3);

            reader.seek(3.0);
            compare_ramp_events(amp_ramp(3.0, 1.5, 1.5, 0.0), reader.next().unwrap(), 4);
            compare_ramp_events(amp_ramp(3.0, 1.5, 2.0, 2.0), reader.next().unwrap(), 5);
            compare_ramp_events(freq_ramp(3.0, 1.0, 1.0, 0.0), reader.next().unwrap(), 6);
            compare_ramp_events(freq_ramp(3.0, 1.0, 1.0, 0.0), reader.next().unwrap(), 7);
            compare_ramp_events(amp_ramp(5.0, 2.0, 0.0, 0.0), reader.next().unwrap(), 8);
            assert!(reader.next().is_none());
        }

        #[test]
        fn amplitude_and_frequency_envelopes_with_seek_to_time_with_breakpoints() {
            check_reader_output_after_seek(
                TestClip {
                    amplitude: &[amp_bp(0.0, 5.0), amp_bp(1.0, 1.0), amp_bp(4.0, 9.0)],
                    frequency: &[freq_bp(0.0, 3.0), freq_bp(1.0, 7.0), freq_bp(3.0, 8.0)],
                },
                1.0,
                &[
                    amp_ramp(1.0, 1.0, 1.0, 0.0),
                    amp_ramp(1.0, 1.0, 9.0, 3.0),
                    freq_ramp(1.0, 7.0, 7.0, 0.0),
                    freq_ramp(1.0, 7.0, 8.0, 2.0),
                    freq_ramp(3.0, 8.0, 8.0, 0.0),
                    amp_ramp(4.0, 9.0, 0.0, 0.0),
                ],
            );
        }

        #[test]
        fn negative_seek() {
            check_reader_output_after_seek(
                TestClip {
                    amplitude: &[amp_bp(0.0, 5.0), amp_bp(2.0, 6.0)],
                    frequency: &[freq_bp(0.0, 3.0), freq_bp(1.0, 7.0)],
                },
                -1.0,
                &[
                    amp_ramp(-1.0, 0.0, 0.0, 0.0),
                    amp_ramp(0.0, 5.0, 5.0, 0.0),
                    amp_ramp(0.0, 5.0, 6.0, 2.0),
                    freq_ramp(0.0, 3.0, 3.0, 0.0),
                    freq_ramp(0.0, 3.0, 7.0, 1.0),
                    freq_ramp(1.0, 7.0, 7.0, 0.0),
                    amp_ramp(2.0, 6.0, 0.0, 0.0),
                ],
            );
        }

        #[test]
        fn seek_to_before_first_bp() {
            check_reader_output_after_seek(
                TestClip {
                    amplitude: &[amp_bp(2.0, 5.0), amp_bp(5.0, 6.0)],
                    frequency: &[freq_bp(3.0, -1.0), freq_bp(4.0, -2.0)],
                },
                1.0,
                &[
                    amp_ramp(1.0, 0.0, 0.0, 0.0),
                    amp_ramp(2.0, 5.0, 5.0, 0.0),
                    amp_ramp(2.0, 5.0, 6.0, 3.0),
                    freq_ramp(3.0, -1.0, -1.0, 0.0),
                    freq_ramp(3.0, -1.0, -2.0, 1.0),
                    freq_ramp(4.0, -2.0, -2.0, 0.0),
                    amp_ramp(5.0, 6.0, 0.0, 0.0),
                ],
            );
        }

        #[test]
        fn seek_to_last_bp() {
            check_reader_output_after_seek(
                TestClip {
                    amplitude: &[amp_bp(2.0, 5.0), amp_bp(5.0, 6.0)],
                    frequency: &[freq_bp(3.0, -1.0), freq_bp(4.0, -2.0)],
                },
                5.0,
                &[amp_ramp(5.0, 6.0, 6.0, 0.0), amp_ramp(5.0, 6.0, 0.0, 0.0)],
            );
        }

        #[test]
        fn seek_to_after_last_bp() {
            check_reader_output_after_seek(
                TestClip {
                    amplitude: &[amp_bp(2.0, 5.0), amp_bp(5.0, 6.0)],
                    frequency: &[freq_bp(3.0, -1.0), freq_bp(4.0, -2.0)],
                },
                6.0,
                &[],
            );
        }
    }

    mod looping {
        use super::*;

        #[test]
        fn looped_amplitude_envelope() {
            check_looped_reader_output(
                TestClip {
                    amplitude: &[amp_bp(0.0, 5.0), amp_bp(1.0, 6.0), amp_bp(4.0, 7.0)],
                    frequency: &[],
                },
                &[
                    amp_ramp(0.0, 5.0, 5.0, 0.0),
                    amp_ramp(0.0, 5.0, 6.0, 1.0),
                    amp_ramp(1.0, 6.0, 7.0, 3.0),
                    // Iteration 2
                    amp_ramp(4.0, 5.0, 5.0, 0.0),
                    amp_ramp(4.0, 5.0, 6.0, 1.0),
                    amp_ramp(5.0, 6.0, 7.0, 3.0),
                    // Iteration 3
                    amp_ramp(8.0, 5.0, 5.0, 0.0),
                    amp_ramp(8.0, 5.0, 6.0, 1.0),
                    amp_ramp(9.0, 6.0, 7.0, 3.0),
                    // Iteration 4...
                    amp_ramp(12.0, 5.0, 5.0, 0.0),
                ],
            );
        }

        #[test]
        fn looped_short_amplitude_envelope() {
            // Define short clip with only two breakpoints
            let data = TestClip {
                amplitude: &[amp_bp(0.0, 0.0), amp_bp(5.0, 1.0)],
                frequency: &[],
            }
            .into();

            // Set up StreamingEventReader
            let mut reader = StreamingEventReader::new(&data);

            // Iteration 1
            // Consume first 2 amplitude ramp events (start of playback)
            reader.next_event().unwrap();
            reader.next_event().unwrap();

            // reader.peeked_event() is empty now. We want to populate it and implicitly call reader.next_event()
            reader.peek().unwrap();

            // Enable looping (during playback)
            reader.set_looping_enabled(true);

            // Check if we get are successfully getting the last clip event
            compare_ramp_events(
                amp_ramp(5.0, 1.0, 0.0, 0.0),
                reader.next_event().unwrap(),
                0,
            );

            // Iteration 2
            compare_ramp_events(
                amp_ramp(5.0, 0.0, 0.0, 0.0),
                reader.next_event().unwrap(),
                0,
            );
            compare_ramp_events(
                amp_ramp(5.0, 0.0, 1.0, 5.0),
                reader.next_event().unwrap(),
                0,
            );

            // Disable looping (during playback)
            reader.set_looping_enabled(false);

            // Check last event: looping is disabled; we fade to zero and stop.
            compare_ramp_events(
                amp_ramp(10.0, 1.0, 0.0, 0.0),
                reader.next_event().unwrap(),
                0,
            );

            // Playback is stopped: no further events are expected
            assert!(reader.peek().is_none());
            assert!(reader.next_event().is_none());
        }

        #[test]
        fn looped_short_amplitude_and_frequency_envelope() {
            // Define short clip, including frequency modulation
            let data = TestClip {
                amplitude: &[amp_bp(0.0, 0.0), amp_bp(5.0, 1.0)],
                frequency: &[freq_bp(2.0, -2.0), freq_bp(4.0, 2.0)],
            }
            .into();

            // Set up StreamingEventReader
            let mut reader = StreamingEventReader::new(&data);

            // Iteration 1
            // Consume first 2 amplitude ramp events (start of playback)
            reader.next_event().unwrap();
            reader.next_event().unwrap();

            // Check frequency modulation events
            compare_ramp_events(
                freq_ramp(2.0, -2.0, -2.0, 0.0),
                reader.next_event().unwrap(),
                0,
            );
            compare_ramp_events(
                freq_ramp(2.0, -2.0, 2.0, 2.0),
                reader.next_event().unwrap(),
                0,
            );
            compare_ramp_events(
                freq_ramp(4.0, 2.0, 2.0, 0.0),
                reader.next_event().unwrap(),
                0,
            );

            // reader.peeked_event() is empty now. We want to populate it and implicitly call reader.next_event()
            reader.peek().unwrap();

            // Enable looping (during playback)
            reader.set_looping_enabled(true);

            // Iteration 2
            compare_ramp_events(
                amp_ramp(5.0, 1.0, 0.0, 0.0),
                reader.next_event().unwrap(),
                0,
            );
            compare_ramp_events(
                amp_ramp(5.0, 0.0, 0.0, 0.0),
                reader.next_event().unwrap(),
                0,
            );
            compare_ramp_events(
                amp_ramp(5.0, 0.0, 1.0, 5.0),
                reader.next_event().unwrap(),
                0,
            );

            // Disable looping (during playback)
            reader.set_looping_enabled(false);

            // Check frequency modulation events
            compare_ramp_events(
                freq_ramp(7.0, -2.0, -2.0, 0.0),
                reader.next_event().unwrap(),
                0,
            );
            compare_ramp_events(
                freq_ramp(7.0, -2.0, 2.0, 2.0),
                reader.next_event().unwrap(),
                0,
            );
            compare_ramp_events(
                freq_ramp(9.0, 2.0, 2.0, 0.0),
                reader.next_event().unwrap(),
                0,
            );

            // Check last event: looping is disabled; we fade to zero and stop.
            compare_ramp_events(
                amp_ramp(10.0, 1.0, 0.0, 0.0),
                reader.next_event().unwrap(),
                0,
            );

            // Playback should stop: no further events to process
            assert!(reader.peek().is_none());
            assert!(reader.next_event().is_none());
        }

        #[test]
        fn looped_amplitude_and_frequency_envelopes() {
            check_looped_reader_output(
                TestClip {
                    amplitude: &[amp_bp(0.0, 5.0), amp_bp(1.0, 6.0), amp_bp(4.0, 7.0)],
                    frequency: &[freq_bp(0.5, 1.0), freq_bp(2.5, 4.0)],
                },
                &[
                    amp_ramp(0.0, 5.0, 5.0, 0.0),
                    amp_ramp(0.0, 5.0, 6.0, 1.0),
                    freq_ramp(0.5, 1.0, 1.0, 0.0),
                    freq_ramp(0.5, 1.0, 4.0, 2.0),
                    amp_ramp(1.0, 6.0, 7.0, 3.0),
                    freq_ramp(2.5, 4.0, 4.0, 0.0),
                    // Iteration 2
                    amp_ramp(4.0, 5.0, 5.0, 0.0),
                    amp_ramp(4.0, 5.0, 6.0, 1.0),
                    freq_ramp(4.5, 1.0, 1.0, 0.0),
                    freq_ramp(4.5, 1.0, 4.0, 2.0),
                    amp_ramp(5.0, 6.0, 7.0, 3.0),
                    freq_ramp(6.5, 4.0, 4.0, 0.0),
                    // Iteration 3
                    amp_ramp(8.0, 5.0, 5.0, 0.0),
                    amp_ramp(8.0, 5.0, 6.0, 1.0),
                    freq_ramp(8.5, 1.0, 1.0, 0.0),
                    freq_ramp(8.5, 1.0, 4.0, 2.0),
                    amp_ramp(9.0, 6.0, 7.0, 3.0),
                    freq_ramp(10.5, 4.0, 4.0, 0.0),
                    // Iteration 4...
                    amp_ramp(12.0, 5.0, 5.0, 0.0),
                ],
            );
        }

        #[test]
        fn looped_amplitude_envelope_with_emphasis_on_first_event() {
            check_looped_reader_output(
                TestClip {
                    amplitude: &[
                        emphasis_bp(0.0, 5.0, 2.0, 3.0),
                        amp_bp(1.0, 6.0),
                        amp_bp(4.0, 7.0),
                    ],
                    frequency: &[],
                },
                &[
                    amp_ramp(0.0, 5.0, 5.0, 0.0),
                    amp_ramp(0.0, 5.0, 6.0, 1.0),
                    emphasis_event(0.0, 2.0, 3.0),
                    amp_ramp(1.0, 6.0, 7.0, 3.0),
                    // Iteration 2
                    amp_ramp(4.0, 5.0, 5.0, 0.0),
                    amp_ramp(4.0, 5.0, 6.0, 1.0),
                    emphasis_event(4.0, 2.0, 3.0),
                    amp_ramp(5.0, 6.0, 7.0, 3.0),
                    // Iteration 3
                    amp_ramp(8.0, 5.0, 5.0, 0.0),
                    amp_ramp(8.0, 5.0, 6.0, 1.0),
                    emphasis_event(8.0, 2.0, 3.0),
                    amp_ramp(9.0, 6.0, 7.0, 3.0),
                    // Iteration 4...
                    amp_ramp(12.0, 5.0, 5.0, 0.0),
                ],
            );
        }

        #[test]
        fn looped_amplitude_envelope_with_emphasis_on_last_event() {
            // This test demonstrates that emphasis on the last event is discarded,
            // even when looping is activated (you can think of the loop region as being
            // non-inclusive of the end breakpoint).
            check_looped_reader_output(
                TestClip {
                    amplitude: &[
                        amp_bp(0.0, 5.0),
                        amp_bp(1.0, 6.0),
                        emphasis_bp(4.0, 7.0, 2.0, 3.0),
                    ],
                    frequency: &[],
                },
                &[
                    amp_ramp(0.0, 5.0, 5.0, 0.0),
                    amp_ramp(0.0, 5.0, 6.0, 1.0),
                    amp_ramp(1.0, 6.0, 7.0, 3.0),
                    // Iteration 2
                    amp_ramp(4.0, 5.0, 5.0, 0.0),
                    amp_ramp(4.0, 5.0, 6.0, 1.0),
                    amp_ramp(5.0, 6.0, 7.0, 3.0),
                    // Iteration 3
                    amp_ramp(8.0, 5.0, 5.0, 0.0),
                    amp_ramp(8.0, 5.0, 6.0, 1.0),
                    amp_ramp(9.0, 6.0, 7.0, 3.0),
                    // Iteration 4...
                    amp_ramp(12.0, 5.0, 5.0, 0.0),
                ],
            );
        }

        #[test]
        fn looped_amplitude_envelope_after_seek() {
            check_looped_reader_output_after_seek(
                TestClip {
                    amplitude: &[amp_bp(0.0, 5.0), amp_bp(1.0, 6.0), amp_bp(3.0, 7.0)],
                    frequency: &[],
                },
                2.0,
                &[
                    amp_ramp(2.0, 6.5, 6.5, 0.0),
                    amp_ramp(2.0, 6.5, 7.0, 1.0),
                    // Iteration 2
                    amp_ramp(3.0, 5.0, 5.0, 0.0),
                    amp_ramp(3.0, 5.0, 6.0, 1.0),
                    amp_ramp(4.0, 6.0, 7.0, 2.0),
                    // Iteration 3
                    amp_ramp(6.0, 5.0, 5.0, 0.0),
                    amp_ramp(6.0, 5.0, 6.0, 1.0),
                    amp_ramp(7.0, 6.0, 7.0, 2.0),
                    // Iteration 4...
                    amp_ramp(9.0, 5.0, 5.0, 0.0),
                ],
            );
        }

        #[test]
        fn looped_amplitude_and_frequency_envelopes_after_seek_that_matches_breakpoint() {
            check_looped_reader_output_after_seek(
                TestClip {
                    amplitude: &[amp_bp(0.0, 5.0), amp_bp(1.0, 6.0), amp_bp(4.0, 7.0)],
                    frequency: &[freq_bp(0.0, 0.0), freq_bp(2.0, 4.0)],
                },
                1.0,
                &[
                    // The generated seek events appear first
                    freq_ramp(1.0, 2.0, 2.0, 0.0),
                    freq_ramp(1.0, 2.0, 4.0, 1.0),
                    amp_ramp(1.0, 6.0, 6.0, 0.0),
                    amp_ramp(1.0, 6.0, 7.0, 3.0),
                    freq_ramp(2.0, 4.0, 4.0, 0.0),
                    // Iteration 2
                    amp_ramp(4.0, 5.0, 5.0, 0.0),
                    amp_ramp(4.0, 5.0, 6.0, 1.0),
                    freq_ramp(4.0, 0.0, 0.0, 0.0),
                    freq_ramp(4.0, 0.0, 4.0, 2.0),
                    amp_ramp(5.0, 6.0, 7.0, 3.0),
                    freq_ramp(6.0, 4.0, 4.0, 0.0),
                    // Iteration 3
                    amp_ramp(8.0, 5.0, 5.0, 0.0),
                    amp_ramp(8.0, 5.0, 6.0, 1.0),
                    freq_ramp(8.0, 0.0, 0.0, 0.0),
                    freq_ramp(8.0, 0.0, 4.0, 2.0),
                    amp_ramp(9.0, 6.0, 7.0, 3.0),
                    freq_ramp(10.0, 4.0, 4.0, 0.0),
                    // Iteration 4...
                    amp_ramp(12.0, 5.0, 5.0, 0.0),
                ],
            );
        }

        #[test]
        fn looped_amplitude_envelope_after_negative_seek() {
            check_looped_reader_output_after_seek(
                TestClip {
                    amplitude: &[amp_bp(0.0, 5.0), amp_bp(1.0, 6.0), amp_bp(4.0, 7.0)],
                    frequency: &[],
                },
                -2.0,
                &[
                    amp_ramp(-2.0, 0.0, 0.0, 0.0),
                    amp_ramp(0.0, 5.0, 5.0, 0.0),
                    amp_ramp(0.0, 5.0, 6.0, 1.0),
                    amp_ramp(1.0, 6.0, 7.0, 3.0),
                    // Iteration 2...
                    amp_ramp(4.0, 5.0, 5.0, 0.0),
                    amp_ramp(4.0, 5.0, 6.0, 1.0),
                ],
            );
        }

        #[test]
        fn looped_envelope_after_seek_past_end() {
            check_looped_reader_output_after_seek(
                TestClip {
                    amplitude: &[amp_bp(0.0, 5.0), amp_bp(1.0, 6.0), amp_bp(4.0, 7.0)],
                    frequency: &[freq_bp(2.0, 3.0), freq_bp(3.0, 4.0)],
                },
                8.0,
                &[
                    // Looping is enabled so playback starts from the loop start
                    amp_ramp(0.0, 5.0, 5.0, 0.0),
                    amp_ramp(0.0, 5.0, 6.0, 1.0),
                    amp_ramp(1.0, 6.0, 7.0, 3.0),
                    freq_ramp(2.0, 3.0, 3.0, 0.0),
                    freq_ramp(2.0, 3.0, 4.0, 1.0),
                    freq_ramp(3.0, 4.0, 4.0, 0.0),
                    // Iteration 2...
                    amp_ramp(4.0, 5.0, 5.0, 0.0),
                    amp_ramp(4.0, 5.0, 6.0, 1.0),
                    amp_ramp(5.0, 6.0, 7.0, 3.0),
                    freq_ramp(6.0, 3.0, 3.0, 0.0),
                    freq_ramp(6.0, 3.0, 4.0, 1.0),
                    freq_ramp(7.0, 4.0, 4.0, 0.0),
                ],
            );
        }

        #[test]
        fn looped_amplitude_envelope_with_shorter_frequency_envelope() {
            check_looped_reader_output(
                TestClip {
                    amplitude: &[amp_bp(0.0, 1.0), amp_bp(1.0, 2.0), amp_bp(3.0, 3.0)],
                    frequency: &[freq_bp(0.5, -1.0), freq_bp(2.5, -2.0)],
                },
                &[
                    amp_ramp(0.0, 1.0, 1.0, 0.0),
                    amp_ramp(0.0, 1.0, 2.0, 1.0),
                    freq_ramp(0.5, -1.0, -1.0, 0.0),
                    freq_ramp(0.5, -1.0, -2.0, 2.0),
                    amp_ramp(1.0, 2.0, 3.0, 2.0),
                    freq_ramp(2.5, -2.0, -2.0, 0.0),
                    // Iteration 2
                    amp_ramp(3.0, 1.0, 1.0, 0.0),
                    amp_ramp(3.0, 1.0, 2.0, 1.0),
                    freq_ramp(3.5, -1.0, -1.0, 0.0),
                    freq_ramp(3.5, -1.0, -2.0, 2.0),
                    amp_ramp(4.0, 2.0, 3.0, 2.0),
                    freq_ramp(5.5, -2.0, -2.0, 0.0),
                    // Iteration 3...
                    amp_ramp(6.0, 1.0, 1.0, 0.0),
                    amp_ramp(6.0, 1.0, 2.0, 1.0),
                    freq_ramp(6.5, -1.0, -1.0, 0.0),
                ],
            );
        }
    }

    mod frequency_envelope_with_event_past_the_end {
        //! Frequency envelopes with events past the end of the amplitude envelope don't make sense
        //! (the end of the clip is defined as the end of the amp envelope), but a bug in Studio led
        //! to some buggy clips making it out into the wild, and we now consider these clips to be
        //! valid input, albeit with events that will be ignored.

        use super::*;

        #[test]
        fn event_past_the_end() {
            check_reader_output(
                TestClip {
                    amplitude: &[amp_bp(0.0, 1.0), amp_bp(3.0, 0.0)],
                    frequency: &[freq_bp(2.0, 6.0), freq_bp(4.0, 1.0)],
                },
                &[
                    amp_ramp(0.0, 1.0, 1.0, 0.0),
                    amp_ramp(0.0, 1.0, 0.0, 3.0),
                    freq_ramp(2.0, 6.0, 6.0, 0.0),
                    freq_ramp(2.0, 6.0, 1.0, 2.0),
                    amp_ramp(3.0, 0.0, 0.0, 0.0),
                    // No terminating frequency event is expected, playback has finished
                ],
            );
        }

        #[test]
        fn event_past_the_end_with_looping() {
            check_looped_reader_output(
                TestClip {
                    amplitude: &[amp_bp(0.0, 1.0), amp_bp(3.0, 0.0)],
                    frequency: &[freq_bp(2.0, 6.0), freq_bp(4.0, 1.0)],
                },
                &[
                    amp_ramp(0.0, 1.0, 1.0, 0.0),
                    amp_ramp(0.0, 1.0, 0.0, 3.0),
                    freq_ramp(2.0, 6.0, 6.0, 0.0),
                    freq_ramp(2.0, 6.0, 1.0, 2.0),
                    // Iteration 2
                    amp_ramp(3.0, 1.0, 1.0, 0.0),
                    amp_ramp(3.0, 1.0, 0.0, 3.0),
                    freq_ramp(5.0, 6.0, 6.0, 0.0),
                    freq_ramp(5.0, 6.0, 1.0, 2.0),
                    // Iteration 3...
                    amp_ramp(6.0, 1.0, 1.0, 0.0),
                    amp_ramp(6.0, 1.0, 0.0, 3.0),
                    freq_ramp(8.0, 6.0, 6.0, 0.0),
                ],
            );
        }
    }

    mod invalid_input {
        //! We plan to validate haptic data before it's passed to the event reader,
        //! but in the unlikely event that the reader is provided with invalid data,
        //! we want to ensure that it behaves predictably (even if the behaviour is odd).

        use super::*;

        #[test]
        fn amp_envelope_events_out_of_order() {
            check_reader_output(
                TestClip {
                    amplitude: &[amp_bp(0.0, 3.0), amp_bp(1.0, 9.0), amp_bp(0.0, 0.0)],
                    frequency: &[],
                },
                &[
                    amp_ramp(0.0, 3.0, 3.0, 0.0),
                    amp_ramp(0.0, 3.0, 9.0, 1.0),
                    // A negative duration!
                    amp_ramp(1.0, 9.0, 0.0, -1.0),
                    amp_ramp(0.0, 0.0, 0.0, 0.0),
                ],
            );
        }
    }

    mod owned_clip_data {
        use std::rc::Rc;
        use std::sync::Arc;

        use super::*;

        #[test]
        fn rc() {
            check_reader_output_with_owned_data(
                Rc::new(
                    TestClip {
                        amplitude: &[amp_bp(0.0, 3.0), amp_bp(1.0, 9.0), amp_bp(3.0, 0.0)],
                        frequency: &[],
                    }
                    .into(),
                ),
                &[
                    amp_ramp(0.0, 3.0, 3.0, 0.0),
                    amp_ramp(0.0, 3.0, 9.0, 1.0),
                    amp_ramp(1.0, 9.0, 0.0, 2.0),
                    amp_ramp(3.0, 0.0, 0.0, 0.0),
                ],
            );
        }

        #[test]
        fn arc() {
            check_reader_output_with_owned_data(
                Arc::new(
                    TestClip {
                        amplitude: &[amp_bp(0.0, 3.0), amp_bp(1.0, 9.0), amp_bp(3.0, 0.0)],
                        frequency: &[],
                    }
                    .into(),
                ),
                &[
                    amp_ramp(0.0, 3.0, 3.0, 0.0),
                    amp_ramp(0.0, 3.0, 9.0, 1.0),
                    amp_ramp(1.0, 9.0, 0.0, 2.0),
                    amp_ramp(3.0, 0.0, 0.0, 0.0),
                ],
            );
        }
    }

    mod peeking_next {
        use super::*;

        #[test]
        fn peek() {
            let clip = TestClip {
                amplitude: &[amp_bp(0.0, 5.0), amp_bp(1.0, 1.0)],
                frequency: &[],
            };
            let data = clip.into();
            let mut reader = StreamingEventReader::new(&data);
            compare_ramp_events(amp_ramp(0.0, 5.0, 5.0, 0.0), reader.peek().unwrap(), 0);
        }

        #[test]
        fn next_if() {
            let clip = TestClip {
                amplitude: &[amp_bp(0.0, 5.0), amp_bp(1.0, 1.0)],
                frequency: &[],
            };
            let data = clip.into();
            let mut reader = StreamingEventReader::new(&data);

            assert!(reader.next_if(|event| event.is_frequency_ramp()).is_none());
            compare_ramp_events(
                amp_ramp(0.0, 5.0, 5.0, 0.0),
                reader.next_if(|event| event.is_amplitude_ramp()).unwrap(),
                0,
            );

            assert!(reader.next_if(|event| event.is_frequency_ramp()).is_none());
            compare_ramp_events(
                amp_ramp(0.0, 5.0, 1.0, 1.0),
                reader.next_if(|event| event.is_amplitude_ramp()).unwrap(),
                1,
            );
        }

        #[test]
        fn peeking_take_while() {
            use itertools::Itertools;

            let clip = TestClip {
                amplitude: &[amp_bp(0.0, 5.0), amp_bp(1.0, 1.0)],
                frequency: &[],
            };
            let data = clip.into();
            let mut reader = StreamingEventReader::new(&data);

            let events: Vec<StreamingEvent> = reader
                .peeking_take_while(|event| event.time < 1.0)
                .collect();
            compare_ramp_event_slices(
                &[amp_ramp(0.0, 5.0, 5.0, 0.0), amp_ramp(0.0, 5.0, 1.0, 1.0)],
                &events,
            );

            compare_ramp_events(amp_ramp(1.0, 1.0, 0.0, 0.0), reader.next().unwrap(), 0);
        }
    }

    mod test_utils {
        use super::*;

        pub fn check_reader_output(clip: TestClip, expected_output: &[StreamingEvent]) {
            let data = clip.into();
            let actual_output: Vec<StreamingEvent> = StreamingEventReader::new(&data).collect();
            compare_ramp_event_slices(expected_output, &actual_output);
        }

        pub fn check_reader_output_with_owned_data<H>(
            haptic_data: H,
            expected_output: &[StreamingEvent],
        ) where
            H: Deref<Target = HapticData>,
        {
            let actual_output: Vec<StreamingEvent> =
                StreamingEventReader::new(haptic_data).collect();
            compare_ramp_event_slices(expected_output, &actual_output);
        }

        pub fn check_reader_output_after_seek(
            clip: TestClip,
            seek_time: f32,
            expected_output: &[StreamingEvent],
        ) {
            let data = clip.into();
            let mut reader = StreamingEventReader::new(&data);
            reader.seek(seek_time);
            let actual_output: Vec<StreamingEvent> = reader.collect();
            compare_ramp_event_slices(expected_output, &actual_output);
        }

        pub fn check_looped_reader_output(clip: TestClip, expected_output: &[StreamingEvent]) {
            let data = clip.into();
            let mut reader = StreamingEventReader::new(&data);
            reader.set_looping_enabled(true);
            let actual_output: Vec<StreamingEvent> = reader.take(expected_output.len()).collect();
            compare_ramp_event_slices(expected_output, &actual_output);
        }

        pub fn check_looped_reader_output_after_seek(
            clip: TestClip,
            seek_time: f32,
            expected_output: &[StreamingEvent],
        ) {
            let data = clip.into();
            let mut reader = StreamingEventReader::new(&data);
            reader.set_looping_enabled(true);
            reader.seek(seek_time);
            let actual_output: Vec<StreamingEvent> = reader.take(expected_output.len()).collect();
            compare_ramp_event_slices(expected_output, &actual_output);
        }
    }
}
