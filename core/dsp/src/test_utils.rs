// (c) Meta Platforms, Inc. and affiliates. Confidential and proprietary.

//! Utilities that are useful when testing DSP logic

/// Returns true if `a` and `b` are close in value
pub fn is_near(a: f32, b: f32, allowed_difference: f32) -> bool {
    (a - b).abs() < allowed_difference
}

/// Returns a buffer containing a rendered sine wave
pub fn make_sine(sample_rate: f32, frequency: f32, size: usize) -> Vec<f32> {
    use std::f32::consts::TAU;

    (0..size)
        .map(|i| ((i as f32) / sample_rate * TAU * frequency).sin())
        .collect()
}
