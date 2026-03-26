// Copyright (c) Meta Platforms, Inc. and affiliates.

// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

#include <gtest/gtest.h>

#include "haptic_renderer/continuous_oscillator.h"

void check_oscillator_output(
    HapticRendererContinuousOscillator* oscillator,
    const std::vector<float>& expected_output) {
  float allowed_error = 1.0e-6;

  for (float i : expected_output) {
    float output = haptic_renderer_continuous_oscillator_process(oscillator);
    EXPECT_NEAR(output, i, allowed_error);
  }
}

TEST(ContinuousOscillatorSynthesisTest, AmplitudeAndFrequencyChanges) {
  float sample_rate = 4.0;
  HapticRendererContinuousOscillatorSettings settings;
  settings.gain = 1.0;
  settings.emphasis_ducking = 1.0;
  settings.frequency_min = 0.0;
  settings.frequency_max = 2.0;

  HapticRendererContinuousOscillator oscillator;
  haptic_renderer_continuous_oscillator_init(
      &oscillator, sample_rate, HAPTIC_RENDERER_MODE_SYNTHESIS, &settings);

  // The oscillator is silent by default
  check_oscillator_output(&oscillator, {0.0, 0.0, 0.0, 0.0});

  // Set the amplitude to 1.0 immediately
  haptic_renderer_continuous_oscillator_set_amplitude(&oscillator, 1.0, 0.0);

  // By default, the oscillator has a frequency in the middle of the min/max range
  // In this case, a frequency of 1.0, corresponding to a period of 4 samples
  check_oscillator_output(&oscillator, {0.0, 1.0, 0.0, -1.0, 0.0, 1.0, 0.0, -1.0});

  // Fade the amplitude to 0.5 over 2 seconds
  haptic_renderer_continuous_oscillator_set_amplitude(&oscillator, 0.5, 2.0);
  check_oscillator_output(
      &oscillator, {0.0, 0.92857146, 0.0, -0.78571427, 0.0, 0.64285713, 0.0, -0.5});
  check_oscillator_output(&oscillator, {0.0, 0.5, 0.0, -0.5, 0.0, 0.5, 0.0, -0.5});

  // Fade the frequency to 1.0 over 2 seconds
  haptic_renderer_continuous_oscillator_set_frequency(&oscillator, 1.0, 2.0);
  check_oscillator_output(
      &oscillator, {0.0, 0.5, -0.11126044, -0.3909158, 0.4874639, -0.31174466, 0.11126006, 0.0});

  // At this point, the output is all zeros due to the frequency matching nyquist and the
  // phase of the zero-crossings being aligned with the samples.
  check_oscillator_output(&oscillator, {0.0, 0.0, 0.0, 0.0});

  // Fade the frequency back down to zero over 2 seconds
  haptic_renderer_continuous_oscillator_set_frequency(&oscillator, 0.0, 2.0);
  check_oscillator_output(
      &oscillator, {0.0, 0.0, -0.21694209, 0.48746404, -0.21694146, -0.48746404, -0.2169423, 0.0});

  // Now, the output is all zeros due to the frequency being 0,
  // and the phase happens to be at 0.
  check_oscillator_output(&oscillator, {0.0, 0.0, 0.0, 0.0});
}

TEST(ContinuousOscillatorSynthesisTest, GainSettingAndEmphasisDucking) {
  float sample_rate = 4.0;
  HapticRendererContinuousOscillatorSettings settings;
  settings.gain = 0.5f;
  settings.emphasis_ducking = 0.5f;
  settings.frequency_min = 0.0f;
  settings.frequency_max = 2.0f;

  HapticRendererContinuousOscillator oscillator;
  haptic_renderer_continuous_oscillator_init(
      &oscillator, sample_rate, HAPTIC_RENDERER_MODE_SYNTHESIS, &settings);

  // The oscillator is silent by default
  check_oscillator_output(&oscillator, {0.0, 0.0, 0.0, 0.0});

  // Set the amplitude to 1.0 immediately
  haptic_renderer_continuous_oscillator_set_amplitude(&oscillator, 1.0, 0.0);

  // The output is scaled by half due to the gain setting
  check_oscillator_output(&oscillator, {0.0, 0.5, 0.0, -0.5, 0.0, 0.5, 0.0, -0.5});

  // Setting the 'emphasis is active' flag to true reduces the gain by another half
  haptic_renderer_continuous_oscillator_set_emphasis_is_active(&oscillator, 1);
  check_oscillator_output(&oscillator, {0.0, 0.25, 0.0, -0.25});

  // Clearing the flag returns the output to normal
  haptic_renderer_continuous_oscillator_set_emphasis_is_active(&oscillator, 0);
  check_oscillator_output(&oscillator, {0.0, 0.5, 0.0, -0.5});
}

TEST(ContinuousOscillatorAmpCurveTest, AmplitudeRampsAndEmphasisDucking) {
  float sample_rate = 4.0;
  HapticRendererContinuousOscillatorSettings settings;
  settings.gain = 1.0f;
  settings.emphasis_ducking = 0.5f;
  settings.frequency_min = 0.0f;
  settings.frequency_max = 2.0f;

  HapticRendererContinuousOscillator oscillator;
  haptic_renderer_continuous_oscillator_init(
      &oscillator, sample_rate, HAPTIC_RENDERER_MODE_AMP_CURVE, &settings);

  // The oscillator is silent by default
  check_oscillator_output(&oscillator, {0.0, 0.0, 0.0, 0.0});

  // Set the amplitude to 1.0 over 1 second
  haptic_renderer_continuous_oscillator_set_amplitude(&oscillator, 1.0, 1.0);

  // The output is scaled by half due to the gain setting
  check_oscillator_output(&oscillator, {0.0, 0.333333, 0.666666, 1.0});

  // Jump to 0.5 amplitude
  haptic_renderer_continuous_oscillator_set_amplitude(&oscillator, 0.5, 0.0);

  // Setting the 'emphasis is active' flag to true reduces the gain by another half
  haptic_renderer_continuous_oscillator_set_emphasis_is_active(&oscillator, 1);
  check_oscillator_output(&oscillator, {0.25, 0.25, 0.25, 0.25});

  // Clearing the flag returns the output to normal
  haptic_renderer_continuous_oscillator_set_emphasis_is_active(&oscillator, 0);
  // The output amplitude returns to the level set by the gain setting
  check_oscillator_output(&oscillator, {0.5, 0.5, 0.5, 0.5});

  // Fade the amplitude down over 1.5 seconds
  haptic_renderer_continuous_oscillator_set_amplitude(&oscillator, 0.0, 1.5);
  check_oscillator_output(&oscillator, {0.5, 0.4, 0.3, 0.2, 0.1, 0.0});
}
