// (c) Meta Platforms, Inc. and affiliates. Confidential and proprietary.

use haptic_data::BasicBreakpoint;
use haptic_data::Breakpoint;
use haptic_data::interpolate_breakpoints;

use crate::Error;
use crate::Result;

/// The event type returned by [StreamingEventReader](crate::StreamingEventReader)
#[derive(Clone, Copy, Debug)]
pub struct StreamingEvent {
    /// The time at which the ramp should start
    pub time: f32,
    /// The event itself, which may be an amplitude or frequency ramp, or an emphasis event
    pub event: StreamingEventType,
}

impl StreamingEvent {
    /// Returns true if the event is an amplitude ramp
    pub fn is_amplitude_ramp(&self) -> bool {
        matches!(&self.event, StreamingEventType::AmplitudeRamp { .. })
    }

    /// Returns true if the event is an frequency ramp
    pub fn is_frequency_ramp(&self) -> bool {
        matches!(&self.event, StreamingEventType::FrequencyRamp { .. })
    }

    /// Splits ramps at the given time
    ///
    /// If the event is a ramp with non-zero duration, then the ramp is split at the given time,
    /// and a jump to the interpolated value at the split point is returned, along with the ramp
    /// segment following the split point. The ramp segment before the split point is discarded.
    ///
    /// If the event is a ramp with zero duration, then a jump event to the ramp's target value at
    /// the given time is returned, with None as the following ramp event.
    ///
    /// An error will be returned if the event is an emphasis event, or if the split time is
    /// outside of a non-zero duration ramp's range.
    pub fn split_ramp_at_time(
        &self,
        split_time: f32,
    ) -> Result<(StreamingEvent, Option<StreamingEvent>)> {
        use StreamingEventType::*;

        macro_rules! split_ramp {
            ($ramp:expr, $ramp_type:tt) => {{
                let ramp = $ramp;
                if ramp.duration == 0.0 {
                    Ok((
                        StreamingEvent {
                            time: split_time,
                            event: $ramp_type(ramp),
                        },
                        None,
                    ))
                } else {
                    let end_time = self.time + ramp.duration;

                    if split_time < self.time || split_time > end_time {
                        return Err(Error::SplitTimeOutOfBounds {
                            split_time,
                            ramp: *self,
                        });
                    }

                    let start_bp = BasicBreakpoint::from_time_value(self.time, ramp.start);
                    let end_bp = BasicBreakpoint::from_time_value(end_time, ramp.target);
                    let split_bp = interpolate_breakpoints(&start_bp, &end_bp, split_time);

                    let jump_event = StreamingEvent {
                        time: split_time,
                        event: $ramp_type(StreamingRamp {
                            start: split_bp.value,
                            target: split_bp.value,
                            duration: 0.0,
                        }),
                    };
                    let ramp_event = StreamingEvent {
                        time: split_time,
                        event: $ramp_type(StreamingRamp::from_breakpoints(&split_bp, &end_bp)),
                    };

                    Ok((jump_event, Some(ramp_event)))
                }
            }};
        }

        match self.event {
            AmplitudeRamp(ramp) => split_ramp!(ramp, AmplitudeRamp),
            FrequencyRamp(ramp) => split_ramp!(ramp, FrequencyRamp),
            Emphasis { .. } => Err(Error::CantSplitAnEmphasisEvent(*self)),
        }
    }

    /// Adjusts the amplitude of AmplitudeRamp and Emphasis events
    pub fn adjust_amplitude(&mut self, scaling_factor: f32) {
        use StreamingEventType::*;
        match &mut self.event {
            AmplitudeRamp(ramp) => {
                ramp.start = (ramp.start * scaling_factor).clamp(0.0, 1.0);
                ramp.target = (ramp.target * scaling_factor).clamp(0.0, 1.0);
            }
            Emphasis { amplitude, .. } => {
                *amplitude = (*amplitude * scaling_factor).clamp(0.0, 1.0)
            }
            _ => {}
        }
    }

    /// Applies a frequency shift to FrequencyRamp and Emphasis events
    pub fn apply_frequency_shift(&mut self, shift_amount: f32) {
        use StreamingEventType::*;
        match &mut self.event {
            FrequencyRamp(ramp) => {
                ramp.start = (ramp.start + shift_amount).clamp(0.0, 1.0);
                ramp.target = (ramp.target + shift_amount).clamp(0.0, 1.0);
            }
            Emphasis { frequency, .. } => *frequency = (*frequency + shift_amount).clamp(0.0, 1.0),
            _ => {}
        }
    }

    /// Returns a new StreamingEvent that is a copy of this event, with a time offset added to its
    /// time
    pub fn with_time_offset(&self, time_offset: f32) -> StreamingEvent {
        StreamingEvent {
            time: self.time + time_offset,
            ..*self
        }
    }
}

/// The inner event type contained in a [StreamingEvent]
#[derive(Clone, Copy, Debug)]
#[allow(missing_docs)]
pub enum StreamingEventType {
    /// An amplitude ramp
    AmplitudeRamp(StreamingRamp),
    /// A frequency ramp
    FrequencyRamp(StreamingRamp),
    /// An emphasis event
    Emphasis { amplitude: f32, frequency: f32 },
}

/// Common properties of amplitude and frequency ramps
#[derive(Clone, Copy, Debug)]
pub struct StreamingRamp {
    /// The value at the start of the ramp
    pub start: f32,
    /// The value at the end of the ramp
    pub target: f32,
    /// The duration of the ramp
    pub duration: f32,
}

impl StreamingRamp {
    /// Creates a ramp between two breakpoints
    pub fn from_breakpoints<T: Breakpoint>(start: &T, end: &T) -> Self {
        StreamingRamp {
            start: start.value(),
            target: end.value(),
            duration: end.time() - start.time(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::test_utils::*;

    mod split_ramp {
        use super::*;

        #[test]
        fn amplitude_jump_event() {
            let ramp = amp_ramp(10.0, 1.0, 1.0, 0.0);
            let (jump_event, following_ramp) = ramp.split_ramp_at_time(20.0).unwrap();
            compare_ramp_events(amp_ramp(20.0, 1.0, 1.0, 0.0), jump_event, 0);
            assert!(following_ramp.is_none());
        }

        #[test]
        fn amplitude_ramp() {
            // Ramp down from 1 to 0 over 20 seconds, starting at time 5
            let ramp = amp_ramp(5.0, 1.0, 0.0, 20.0);
            // Split the ramp at the half-way point
            let (jump_event, following_ramp) = ramp.split_ramp_at_time(15.0).unwrap();
            compare_ramp_events(amp_ramp(15.0, 0.5, 0.5, 0.0), jump_event, 0);
            compare_ramp_events(amp_ramp(15.0, 0.5, 0.0, 10.0), following_ramp.unwrap(), 1);
        }

        #[test]
        fn frequency_ramp() {
            // Ramp up from 0.0 to 1 over 2 seconds, starting at time 2
            let ramp = freq_ramp(2.0, 0.0, 1.0, 2.0);
            // Split the ramp a quarter of the way along
            let (jump_event, following_ramp) = ramp.split_ramp_at_time(2.5).unwrap();
            compare_ramp_events(freq_ramp(2.5, 0.25, 0.25, 0.0), jump_event, 0);
            compare_ramp_events(freq_ramp(2.5, 0.25, 1.0, 1.5), following_ramp.unwrap(), 1);
        }
    }

    mod adjust_amplitude {
        use super::*;

        #[test]
        fn amplitude_ramp() {
            let mut ramp = amp_ramp(0.0, 0.2, 0.4, 1.0);

            ramp.adjust_amplitude(0.5);
            compare_ramp_events(amp_ramp(0.0, 0.1, 0.2, 1.0), ramp, 0);

            ramp.adjust_amplitude(4.0);
            compare_ramp_events(amp_ramp(0.0, 0.4, 0.8, 1.0), ramp, 1);

            ramp.adjust_amplitude(20.0);
            compare_ramp_events(amp_ramp(0.0, 1.0, 1.0, 1.0), ramp, 2);
        }

        #[test]
        fn emphasis() {
            let mut event = emphasis_event(0.0, 0.5, 0.4);

            event.adjust_amplitude(0.5);
            compare_ramp_events(emphasis_event(0.0, 0.25, 0.4), event, 0);

            event.adjust_amplitude(3.0);
            compare_ramp_events(emphasis_event(0.0, 0.75, 0.4), event, 1);

            event.adjust_amplitude(2.0);
            compare_ramp_events(emphasis_event(0.0, 1.0, 0.4), event, 2);
        }

        #[test]
        fn frequency_ramp() {
            let mut ramp = freq_ramp(0.0, 0.2, 0.5, 0.4);

            // adjust_amplitude on a frequency ramp is a no-op
            ramp.adjust_amplitude(0.5);
            compare_ramp_events(freq_ramp(0.0, 0.2, 0.5, 0.4), ramp, 0);
        }
    }

    mod apply_frequency_shift {
        use super::*;

        #[test]
        fn frequency_ramp() {
            let mut ramp = freq_ramp(0.0, 0.0, 0.4, 0.1);

            ramp.apply_frequency_shift(0.5);
            compare_ramp_events(freq_ramp(0.0, 0.5, 0.9, 0.1), ramp, 0);

            // Shifted frequencies get clamped to 0..=1
            ramp.apply_frequency_shift(0.2);
            compare_ramp_events(freq_ramp(0.0, 0.7, 1.0, 0.1), ramp, 1);

            ramp.apply_frequency_shift(-0.8);
            compare_ramp_events(freq_ramp(0.0, 0.0, 0.2, 0.1), ramp, 2);

            ramp.apply_frequency_shift(0.3);
            compare_ramp_events(freq_ramp(0.0, 0.3, 0.5, 0.1), ramp, 3);
        }

        #[test]
        fn emphasis() {
            let mut event = emphasis_event(0.0, 0.5, 0.4);

            event.apply_frequency_shift(0.5);
            compare_ramp_events(emphasis_event(0.0, 0.5, 0.9), event, 0);

            event.apply_frequency_shift(0.2);
            compare_ramp_events(emphasis_event(0.0, 0.5, 1.0), event, 1);

            event.apply_frequency_shift(-0.6);
            compare_ramp_events(emphasis_event(0.0, 0.5, 0.4), event, 2);

            event.apply_frequency_shift(-0.6);
            compare_ramp_events(emphasis_event(0.0, 0.5, 0.0), event, 3);
        }

        #[test]
        fn amplitude_ramp() {
            let mut ramp = amp_ramp(0.0, 0.2, 0.4, 1.0);

            // apply_frequency_shift on an amplitude ramp is a no-op
            ramp.apply_frequency_shift(0.5);
            compare_ramp_events(amp_ramp(0.0, 0.2, 0.4, 1.0), ramp, 0);
        }
    }
}
