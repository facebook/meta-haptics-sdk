// (c) Meta Platforms, Inc. and affiliates. Confidential and proprietary.

/// Convert a value in decibels to its corresponding linear amplitude value
#[inline(always)]
pub fn db_to_amplitude(db: f32) -> f32 {
    10.0_f32.powf(db / 20.0)
}

#[cfg(test)]
mod tests {
    use assert_approx_eq::assert_approx_eq;

    use super::*;

    #[test]
    fn test_db_to_amplitude() {
        let allowed_diff = 5.0e-3_f32;
        let sqrt_2 = 2.0_f32.sqrt();

        assert_approx_eq!(db_to_amplitude(-6.0), 0.5, allowed_diff);
        assert_approx_eq!(db_to_amplitude(-3.0), 1.0 / sqrt_2, allowed_diff);

        assert_eq!(db_to_amplitude(0.0), 1.0);

        assert_approx_eq!(db_to_amplitude(3.0), sqrt_2, allowed_diff);
        assert_approx_eq!(db_to_amplitude(6.0), 2.0, allowed_diff);
    }
}
