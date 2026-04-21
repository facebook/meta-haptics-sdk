// Copyright (c) Meta Platforms, Inc. and affiliates.
//
// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

/// A trait for breakpoints, allowing time/value structs to maintain their own representations
pub trait Breakpoint: Clone {
    /// The time of the breakpoint
    fn time(&self) -> f32;
    /// The value (i.e. the position on the Y axis) of the breakpoint
    fn value(&self) -> f32;

    /// Creates a breakpoint from a time and value
    fn from_time_value(time: f32, value: f32) -> Self;
}

/// Interpolates between two breakpoints given the provided time
pub fn interpolate_breakpoints<T: Breakpoint>(a: &T, b: &T, time: f32) -> T {
    let time_a = a.time();
    let time_b = b.time();
    debug_assert!(
        time_b >= time_a,
        "The breakpoints should be ordered in time"
    );
    debug_assert!(
        time >= time_a && time <= time_b,
        "The time value needs to be within the time range of the provided breakpoints"
    );
    let time_diff = time_b - time_a;
    if time_diff == 0.0 {
        return b.clone();
    }
    let value_a = a.value();
    let value_range = b.value() - value_a;
    let factor = (time - time_a) / time_diff;
    T::from_time_value(time, value_a + value_range * factor)
}

/// A basic implementation of [Breakpoint], useful in testing and generic code
#[derive(Clone, PartialEq)]
#[allow(missing_docs)]
pub struct BasicBreakpoint {
    pub time: f32,
    pub value: f32,
}

impl Breakpoint for BasicBreakpoint {
    fn time(&self) -> f32 {
        self.time
    }

    fn value(&self) -> f32 {
        self.value
    }

    fn from_time_value(time: f32, value: f32) -> Self {
        Self { time, value }
    }
}

impl From<(f32, f32)> for BasicBreakpoint {
    fn from((time, value): (f32, f32)) -> Self {
        Self { time, value }
    }
}

#[cfg(test)]
mod tests {
    use assert_approx_eq::assert_approx_eq;

    use super::*;

    fn check_interpolate_breakpoints_result(
        a: &BasicBreakpoint,
        b: &BasicBreakpoint,
        time: f32,
        expected_value: f32,
    ) {
        let result = interpolate_breakpoints(a, b, time);
        assert_eq!(result.time, time);
        assert_approx_eq!(result.value, expected_value, 1.0e-6);
    }

    #[test]
    fn test_interpolate_breakpoints() {
        let a = BasicBreakpoint {
            time: 0.5,
            value: 2.0,
        };
        let b = BasicBreakpoint {
            time: 1.0,
            value: 5.0,
        };
        check_interpolate_breakpoints_result(&a, &b, 0.5, 2.0);
        check_interpolate_breakpoints_result(&a, &b, 0.75, 3.5);
        check_interpolate_breakpoints_result(&a, &b, 1.0, 5.0);
    }
}
