// Copyright (c) Meta Platforms, Inc. and affiliates.
//
// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

/// An accumulator that compensates for floating-point loss of precision
///
/// The `Accumulator` takes an input sample and adds it to its running sum,
/// and then returns the sum.
///
/// The Kahan summation algorithm is used to compensate for loss of precision over time,
/// see [here](https://en.wikipedia.org/wiki/Kahan_summation_algorithm) for more information.
///
/// # Example
/// ```
/// let mut accumulator = haptic_dsp::Accumulator::default();
/// assert_eq!(accumulator.process(1.0), 1.0);
/// assert_eq!(accumulator.process(1.0), 2.0);
/// assert_eq!(accumulator.process(-1.0), 1.0);
/// //
/// ```
#[derive(Default, Clone, Copy)]
pub struct Accumulator {
    sum: f32,
    compensation: f32,
}

impl Accumulator {
    /// Takes a sample as input, adds it to its internal sum, and returns the sum
    pub fn process(&mut self, input: f32) -> f32 {
        let add_to_sum = input - self.compensation;
        let new_sum = self.sum + add_to_sum;
        self.compensation = (new_sum - self.sum) - add_to_sum;
        self.sum = new_sum;
        new_sum
    }

    /// Returns the current accumulated sum
    pub fn sum(&self) -> f32 {
        self.sum
    }

    /// Resets the accumulator to 0.0
    pub fn reset(&mut self) {
        self.sum = 0.0;
        self.compensation = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_accumulating() {
        let mut a = Accumulator::default();

        assert_eq!(a.process(0.0), 0.0);
        assert_eq!(a.process(1.0), 1.0);
        assert_eq!(a.sum(), 1.0);

        assert_eq!(a.process(0.5), 1.5);
        assert_eq!(a.process(-2.5), -1.0);
        assert_eq!(a.process(1.0), 0.0);
        assert_eq!(a.sum(), 0.0);
    }

    #[test]
    fn reset() {
        let mut a = Accumulator::default();

        assert_eq!(a.process(0.5), 0.5);
        assert_eq!(a.process(-1.0), -0.5);

        a.reset();
        assert_eq!(a.sum(), 0.0);
    }

    #[test]
    fn accumulating_without_loss_of_precision() {
        let mut accumulator = Accumulator::default();
        let mut imprecise_sum = 0.0;

        let target = 1.0;
        let steps = 10;
        let increment = target / steps as f32;
        for _ in 0..steps {
            accumulator.process(increment);
            imprecise_sum += increment;
        }

        assert_eq!(accumulator.process(0.0), target);
        assert_ne!(imprecise_sum, target);
    }
}
