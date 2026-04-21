// (c) Meta Platforms, Inc. and affiliates. Confidential and proprietary.

#![allow(missing_docs)]

/// A monophonic fixed-length delay line.
///
/// Samples that are passed in to `process()` are stored and then returned later after a delay of
/// a fixed number of samples.
///
/// The number of samples is derived from a requested delay time in seconds,
/// given a specific sample rate.
///
/// # Example
///
/// ```
/// use haptic_dsp::FixedDelayLine;
///
/// // A sample rate of 2 is unrealistic, but useful for demonstrating the delay line's behaviour.
/// let sample_rate = 2.0;
///
/// // A delay time of 1 second with a sample rate of 2 produces a delay line of 2 samples.
/// let delay_time = 1.0;
///
/// let mut d = FixedDelayLine::new(delay_time, sample_rate);
///
/// assert_eq!(d.process(1.0), 0.0); // The delay line is initialized with 0.0
/// assert_eq!(d.process(2.0), 0.0);
/// assert_eq!(d.process(3.0), 1.0); // Previous input starts appearing after 2 samples
/// assert_eq!(d.process(4.0), 2.0);
/// assert_eq!(d.process(5.0), 3.0);
/// assert_eq!(d.process(6.0), 4.0);
/// // ...
/// ```
pub struct FixedDelayLine {
    buffer: Vec<f32>,
    position: usize,
}

impl FixedDelayLine {
    /// Initializes the delay line with a fixed number of samples based on the specified parameters
    pub fn new(delay_time: f32, sample_rate: f32) -> Self {
        let delay_samples = (delay_time * sample_rate).round().max(0.0) as usize;

        Self {
            buffer: vec![0.0; delay_samples + 1],
            position: 0,
        }
    }

    pub fn with_fixed_length(length_in_samples: usize) -> Self {
        Self {
            buffer: vec![0.0; length_in_samples + 1],
            position: 0,
        }
    }

    /// Processes a single sample of input and returns a delayed sample of output
    pub fn process(&mut self, input: f32) -> f32 {
        // Write the sample to the buffer at the current position
        self.buffer[self.position] = input;

        // Advance the buffer position, wrapping back to the start if necessary
        self.position += 1;
        if self.position == self.buffer.len() {
            self.position = 0;
        }

        // Read out the delayed sample from the new buffer position
        self.buffer[self.position]
    }

    /// Provides the delay line's length in samples
    pub fn length_in_samples(&self) -> usize {
        self.buffer.len() - 1
    }

    /// Returns an iterator that provides the delay line's contents starting from the oldest sample
    pub fn iter(&self) -> FixedDelayLineIter<'_> {
        // Start from the oldest sample,
        // i.e. the sample that would be returned next from process().
        let start_position = if (self.position + 1) < self.buffer.len() {
            self.position + 1
        } else {
            0
        };

        // End at the current position,
        // i.e. the sample that was written to most recently.
        let end_position = self.position;

        FixedDelayLineIter {
            delay_line: self,
            position: start_position,
            end_position,
        }
    }
}

/// An iterator for the FixedDelayLine's buffer
///
/// This iterator provides access to the delay line's buffer contents, starting with the oldest
/// stored sample, and then ending with the most recently stored sample.
///
/// # Example
///
/// ```
/// use haptic_dsp::FixedDelayLine;
///
/// // A sample rate of 2 is unrealistic, but useful for demonstrating the delay line's behaviour.
/// let sample_rate = 2.0;
///
/// // A delay time of 1 second with a sample rate of 2 produces a delay line of 2 samples.
/// let delay_time = 1.0;
///
/// let mut d = FixedDelayLine::new(delay_time, sample_rate);
///
/// assert_eq!(d.iter().collect::<Vec<_>>(), vec![0.0, 0.0]);
/// d.process(1.0);
/// assert_eq!(d.iter().collect::<Vec<_>>(), vec![0.0, 1.0]);
/// d.process(2.0);
/// assert_eq!(d.iter().collect::<Vec<_>>(), vec![1.0, 2.0]);
/// ```
pub struct FixedDelayLineIter<'a> {
    delay_line: &'a FixedDelayLine,
    position: usize,
    end_position: usize,
}

impl<'a> Iterator for FixedDelayLineIter<'a> {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        if self.position != self.end_position {
            let result = self.delay_line.buffer[self.position];

            self.position += 1;
            if self.position == self.delay_line.buffer.len() {
                self.position = 0;
            }

            Some(result)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check_buffer_contents(delay_line: &FixedDelayLine, expected: &[f32]) {
        let mut checked_samples = 0;

        for (expected, actual) in expected.iter().zip(delay_line.iter()) {
            if *expected != actual {
                panic!(
                    "check_buffer_contents: mismatch at sample {checked_samples}, expected '{expected}', actual '{actual}'",
                );
            }
            checked_samples += 1;
        }

        assert_eq!(
            checked_samples,
            expected.len(),
            "Unexpected delay line iteration length"
        );
    }

    #[test]
    fn delay_time_of_zero() {
        let delay_time = 0.0;
        let sample_rate = 2.0;
        let mut d = FixedDelayLine::new(delay_time, sample_rate);

        assert_eq!(d.process(42.0), 42.0);
        assert_eq!(d.process(99.0), 99.0);

        check_buffer_contents(&d, &[]);
    }

    #[test]
    fn delay_time_of_one_second() {
        let delay_time = 1.0;
        let sample_rate = 2.0;
        let mut d = FixedDelayLine::new(delay_time, sample_rate);

        check_buffer_contents(&d, &[0.0, 0.0]);

        assert_eq!(d.process(1.0), 0.0);
        check_buffer_contents(&d, &[0.0, 1.0]);

        assert_eq!(d.process(2.0), 0.0);
        check_buffer_contents(&d, &[1.0, 2.0]);

        assert_eq!(d.process(3.0), 1.0);
        check_buffer_contents(&d, &[2.0, 3.0]);

        assert_eq!(d.process(4.0), 2.0);
        check_buffer_contents(&d, &[3.0, 4.0]);

        assert_eq!(d.process(5.0), 3.0);
        check_buffer_contents(&d, &[4.0, 5.0]);

        assert_eq!(d.process(6.0), 4.0);
        check_buffer_contents(&d, &[5.0, 6.0]);
    }
}
