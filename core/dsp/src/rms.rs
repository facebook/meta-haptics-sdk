// Copyright (c) Meta Platforms, Inc. and affiliates.
//
// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

use crate::Accumulator;
use crate::FixedDelayLine;

/// Provides a 'windowed moving RMS' of the input signal
///
/// This keeps track of the last `N` samples passed in as input to `process()`,
/// and returns the RMS of those samples.
///
/// See [here](https://www.mathworks.com/help/dsp/ref/movingrms.html#bvaobjz-1_sep_bu_1nj9-18_head)
/// for more information.
pub struct WindowedMovingRms {
    // Keeps track of the last N samples
    delay_line: FixedDelayLine,
    // Maintains a running sum while avoiding loss of precision over time
    accumulator: Accumulator,
    // Caches the scaling factor required when calculating the mean in `process()`
    averaging_scaling_factor: f32,
}

impl WindowedMovingRms {
    /// Initializes the window with a fixed size based on the specified parameters
    pub fn new(averaging_time: f32, sample_rate: f32) -> Self {
        let delay_line = FixedDelayLine::new(averaging_time, sample_rate);
        let averaging_scaling_factor = 1.0 / delay_line.length_in_samples() as f32;

        Self {
            delay_line,
            accumulator: Accumulator::default(),
            averaging_scaling_factor,
        }
    }

    /// Processes a single sample of input and returns the RMS of the last N samples received
    pub fn process(&mut self, input: f32) -> f32 {
        let input_squared = input * input;
        let out_of_window = self.delay_line.process(input_squared);
        let windowed_sum = self
            .accumulator
            .process(input_squared - out_of_window)
            .max(0.0);
        let mean_squared = windowed_sum * self.averaging_scaling_factor;
        mean_squared.sqrt()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rms_with_2_sample_window() {
        let averaging_time = 2.0;
        let sample_rate = 1.0;
        let mut rms = WindowedMovingRms::new(averaging_time, sample_rate);

        // silence in, silence out
        assert_eq!(rms.process(0.0), 0.0);
        assert_eq!(rms.process(0.0), 0.0);

        // pass in a window's worth of 0.5
        assert_eq!(rms.process(0.5), ((0.5 * 0.5) / 2.0f32).sqrt());
        assert_eq!(rms.process(0.5), 0.5);

        // The RMS maintains this level while the input is constant
        assert_eq!(rms.process(0.5), 0.5);

        // pass in a window's worth of silence
        assert_eq!(rms.process(0.0), ((0.5 * 0.5) / 2.0f32).sqrt());
        assert_eq!(rms.process(0.0), 0.0);

        // silence in, silence out
        assert_eq!(rms.process(0.0), 0.0);
        assert_eq!(rms.process(0.0), 0.0);

        // pass in a window's worth of 1
        assert_eq!(rms.process(1.0), (1.0 / 2.0f32).sqrt());
        assert_eq!(rms.process(1.0), 1.0);
    }

    #[test]
    fn rms_with_4_sample_window() {
        let averaging_time = 0.5;
        let sample_rate = 8.0;
        let mut rms = WindowedMovingRms::new(averaging_time, sample_rate);

        // silence in, silence out
        assert_eq!(rms.process(0.0), 0.0);
        assert_eq!(rms.process(0.0), 0.0);
        assert_eq!(rms.process(0.0), 0.0);
        assert_eq!(rms.process(0.0), 0.0);

        // pass in a window's worth of 1
        assert_eq!(rms.process(1.0), (1.0 / 4.0f32).sqrt());
        assert_eq!(rms.process(1.0), (2.0 / 4.0f32).sqrt());
        assert_eq!(rms.process(1.0), (3.0 / 4.0f32).sqrt());
        assert_eq!(rms.process(1.0), 1.0);

        // The RMS maintains this level while the input is constant
        assert_eq!(rms.process(1.0), 1.0);

        // pass in a window's worth of silence
        assert_eq!(rms.process(0.0), (3.0 / 4.0f32).sqrt());
        assert_eq!(rms.process(0.0), (2.0 / 4.0f32).sqrt());
        assert_eq!(rms.process(0.0), (1.0 / 4.0f32).sqrt());
        assert_eq!(rms.process(0.0), 0.0);

        // silence in, silence out
        assert_eq!(rms.process(0.0), 0.0);
        assert_eq!(rms.process(0.0), 0.0);
        assert_eq!(rms.process(0.0), 0.0);
        assert_eq!(rms.process(0.0), 0.0);

        // pass in a window's worth of 0.5
        assert_eq!(rms.process(0.5), (1.0 * (0.5 * 0.5) / 4.0f32).sqrt());
        assert_eq!(rms.process(0.5), (2.0 * (0.5 * 0.5) / 4.0f32).sqrt());
        assert_eq!(rms.process(0.5), (3.0 * (0.5 * 0.5) / 4.0f32).sqrt());
        assert_eq!(rms.process(0.5), 0.5);
    }

    #[test]
    fn long_term_precision() {
        // Set up a 4 sample window
        let averaging_time = 1.0;
        let sample_rate = 4.0;
        let mut rms = WindowedMovingRms::new(averaging_time, sample_rate);

        // Sum lots of small floats, which would accumulate summing precision errors if they
        // weren't correctly compensated. You can confirm this behaviour by commenting out the
        // update of the compensation value in the Accumulator.
        let mut x = 0.1;
        for _ in 0..500 {
            rms.process(x);
            x += f32::EPSILON;
        }

        // To test precision, now pass in a window's worth of 1s and check the resulting RMS
        rms.process(1.0);
        rms.process(1.0);
        rms.process(1.0);
        assert_eq!(rms.process(1.0), 1.0);
    }
}
