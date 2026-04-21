// Copyright (c) Meta Platforms, Inc. and affiliates.
//
// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

/// An iterator providing evenly spaced values over a range
///
/// See [linspace] for usage.
pub struct Linspace {
    start: f32,
    step_size: f32,
    count: f32,
    current: f32,
}

impl Iterator for Linspace {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        if self.current < self.count {
            let result = self.start + self.step_size * self.current;
            self.current += 1.0;
            Some(result)
        } else {
            None
        }
    }
}

/// Provides evenly spaced values spread over the range of start to end inclusive
///
/// # Example
/// ```
/// let x: Vec<f32> = haptic_dsp::linspace(0.0, 1.0, 5).collect();
/// assert_eq!(x, vec![0.0, 0.25, 0.5, 0.75, 1.0]);
/// ```
pub fn linspace(start: f32, end: f32, count: usize) -> Linspace {
    Linspace {
        start,
        step_size: (end - start) / (count - 1) as f32,
        count: count as f32,
        current: 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linspace_3_steps() {
        let mut l = linspace(0.0, 1.0, 3);
        assert_eq!(l.next(), Some(0.0));
        assert_eq!(l.next(), Some(0.5));
        assert_eq!(l.next(), Some(1.0));
        assert_eq!(l.next(), None);
    }

    #[test]
    fn linspace_5_steps() {
        let mut l = linspace(1.0, 2.0, 5);
        assert_eq!(l.next(), Some(1.0));
        assert_eq!(l.next(), Some(1.25));
        assert_eq!(l.next(), Some(1.5));
        assert_eq!(l.next(), Some(1.75));
        assert_eq!(l.next(), Some(2.0));
        assert_eq!(l.next(), None);
    }
}
