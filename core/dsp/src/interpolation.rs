// Copyright (c) Meta Platforms, Inc. and affiliates.
//
// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

/// Linearly interpolate between two values
#[inline(always)]
pub fn lerp(a: f32, b: f32, amount: f32) -> f32 {
    a + (b - a) * amount
}

#[cfg(test)]
mod tests {
    use assert_approx_eq::assert_approx_eq;

    use super::*;

    #[test]
    fn test_lerp() {
        let allowed_error = 5.0e-3_f32;

        assert_approx_eq!(lerp(0.0, 1.0, 0.5), 0.5, allowed_error);
        assert_approx_eq!(lerp(10.0, 20.0, 0.2), 12.0, allowed_error);
        assert_approx_eq!(lerp(100.0, -100.0, 0.75), -50.0, allowed_error);
    }
}
