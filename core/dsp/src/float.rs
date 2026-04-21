// Copyright (c) Meta Platforms, Inc. and affiliates.
//
// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

/// Flush tiny f32s to zero to avoid denormalized numbers
///
/// We want to avoid denormals to ensure stable floating point calculation performance.
/// Rather than trying to disable denormals programatically (which will require platform specific
/// operations, and isn't a good idea anyway in an embeddable SDK), we take advantage of
/// precision loss to flush small numbers to zero.
///
/// This should be used whenever a DSP operation will reduce a value towards zero,
/// e.g. exponential decay in an envelope follower.
///
/// The approach taken here in flushing denormals is described as 'Elimination by quantification'
/// in [this article by Laurent de Soras](http://ldesoras.free.fr/doc/articles/denormal-en.pdf).
/// The basic idea is to absorb tiny inputs into the limited resolution of a normal float: a
/// denormal input will be absorbed on the first addition, and then the subraction sets the value
/// to zero.
///
/// The main advantage of this technique it that it avoids branching, unlike a more explicit
/// approach which would rely on testing the value's bits to detect denormals.
///
/// The increment value of 1e-5 results in flushing to zero at ~-140dB, which is a reasonable
/// point to drop to silence for general audio processing.  For other purposes a much smaller
/// increment can be considered (e.g. the article linked above suggests 1e-18).
#[inline(always)]
pub fn flush_f32_to_zero(mut x: f32) -> f32 {
    let increment = 1.0e-5f32;
    x += increment;
    x - increment
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flush_f32_to_zero() {
        assert_eq!(flush_f32_to_zero(0.0), 0.0);
        assert_eq!(flush_f32_to_zero(0.3), 0.3);
        assert_eq!(flush_f32_to_zero(1.0), 1.0);
        assert_eq!(flush_f32_to_zero(-0.3), -0.3);
        assert_eq!(flush_f32_to_zero(-1.0), -1.0);

        assert_ne!(flush_f32_to_zero(1.0e-12f32), 0.0);
        assert_ne!(flush_f32_to_zero(-1.0e-12f32), 0.0);

        assert_eq!(flush_f32_to_zero(1.0e-13f32), 0.0);
        assert_eq!(flush_f32_to_zero(-1.0e-13f32), 0.0);
    }
}
