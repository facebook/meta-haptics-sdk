// Copyright (c) Meta Platforms, Inc. and affiliates.

// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

#ifndef _USE_MATH_DEFINES
#define _USE_MATH_DEFINES // for M_PI
#endif

#include <gtest/gtest.h>
#include <cmath>

#include "haptic_renderer/emphasis_oscillator.h"
#include "haptic_renderer/render_mode.h"

void check_oscillator_output(
    HapticRendererEmphasisOscillator* oscillator,
    const std::vector<float>& expected_output) {
  float allowed_error = 1.0e-9;

  for (float i : expected_output) {
    float output = haptic_renderer_emphasis_oscillator_process(oscillator);
    EXPECT_NEAR(output, i, allowed_error);
  }
}

TEST(EmphasisOscillatorSynthesisTest, SimpleEvents) {
  float sample_rate = 4.0;
  HapticRendererEmphasisOscillatorSettings settings;
  settings.gain = 1.0f;
  settings.fade_out_percent = 0.0f;
  settings.frequency_min.output_frequency = 1.0f;
  settings.frequency_min.duration_ms = 1000.0f;
  settings.frequency_min.shape = HAPTIC_RENDERER_EMPHASIS_SHAPE_SQUARE;
  settings.frequency_max.output_frequency = 1.0f;
  settings.frequency_max.duration_ms = 2000.0f;
  settings.frequency_max.shape = HAPTIC_RENDERER_EMPHASIS_SHAPE_SAW;

  HapticRendererEmphasisOscillator oscillator;
  haptic_renderer_emphasis_oscillator_init(
      &oscillator, sample_rate, HAPTIC_RENDERER_MODE_SYNTHESIS, &settings);

  // The oscillator is silent by default
  check_oscillator_output(&oscillator, {0.0, 0.0, 0.0, 0.0});

  // Trigger an emphasis event with frequency 0.0
  // The event should last for a second, with a square wave output
  haptic_renderer_emphasis_oscillator_start_emphasis(&oscillator, 1.0, 0.0);
  check_oscillator_output(&oscillator, {1.0, 1.0, -1.0, -1.0});
  check_oscillator_output(&oscillator, {0.0, 0.0});

  // Trigger another emphasis event with frequency 0.0, but this time with half amplitude
  // The event should last for a second, with a square wave output
  haptic_renderer_emphasis_oscillator_start_emphasis(&oscillator, 0.5, 0.0);
  check_oscillator_output(&oscillator, {0.5, 0.5, -0.5, -0.5});
  check_oscillator_output(&oscillator, {0.0, 0.0});

  // Trigger an emphasis event with frequency 1.0
  // The event should last for 2 seconds, with a half-frequency saw wave output
  haptic_renderer_emphasis_oscillator_start_emphasis(&oscillator, 1.0, 1.0);
  check_oscillator_output(&oscillator, {1.0, 0.5, 0.0, -0.5, 1.0, 0.5, 0.0, -0.5});
  check_oscillator_output(&oscillator, {0.0, 0.0});
}

// Setting all frequency_min and frequency_max  values to 1.0 (essentially a
// frequency range of 0.0; see test above) doesn't test the case where we
// actually have a non-zero frequency range to test with.
//
// To make sure that the frequency min-max ranges work properly, this test
// was added with frequency_min and frequency_max having more reasonable values.
TEST(EmphasisOscillatorSynthesisTest, FrequencyRangeTest) {
  float sample_rate = 4.0;
  HapticRendererEmphasisOscillatorSettings settings;
  settings.gain = 1.0f;
  settings.fade_out_percent = 0.0f;
  settings.frequency_min.output_frequency = 0.5f;
  settings.frequency_min.duration_ms = 2000.0f;
  settings.frequency_min.shape = HAPTIC_RENDERER_EMPHASIS_SHAPE_SQUARE;
  settings.frequency_max.output_frequency = 1.0f;
  settings.frequency_max.duration_ms = 2000.0f;
  settings.frequency_max.shape = HAPTIC_RENDERER_EMPHASIS_SHAPE_SAW;

  HapticRendererEmphasisOscillator oscillator;
  haptic_renderer_emphasis_oscillator_init(
      &oscillator, sample_rate, HAPTIC_RENDERER_MODE_SYNTHESIS, &settings);

  // The oscillator is silent by default
  check_oscillator_output(&oscillator, {0.0, 0.0, 0.0, 0.0});

  // Trigger an emphasis event with frequency 0.0
  // The event should last for two seconds, with a square wave output
  haptic_renderer_emphasis_oscillator_start_emphasis(&oscillator, 1.0, 0.0);
  check_oscillator_output(&oscillator, {1.0, 1.0, 1.0, 1.0, -1.0, -1.0, -1.0, -1.0});
  check_oscillator_output(&oscillator, {0.0, 0.0});

  // Trigger another emphasis event with frequency 0, but this time with half amplitude
  // The event should last for two seconds, with a square wave output
  haptic_renderer_emphasis_oscillator_start_emphasis(&oscillator, 0.5, 0.0);
  check_oscillator_output(&oscillator, {0.5, 0.5, 0.5, 0.5, -0.5, -0.5, -0.5, -0.5});
  check_oscillator_output(&oscillator, {0.0, 0.0});

  // Trigger an emphasis event with frequency 1.0
  // The event should last for 2 seconds, with a half-frequency saw wave output
  haptic_renderer_emphasis_oscillator_start_emphasis(&oscillator, 1.0, 1.0);
  check_oscillator_output(&oscillator, {1.0, 0.5, 0.0, -0.5, 1.0, 0.5, 0.0, -0.5});
  check_oscillator_output(&oscillator, {0.0, 0.0});
}

TEST(EmphasisOscillatorSynthesisTest, MixedShapes) {
  float sample_rate = 8.0;

  HapticRendererEmphasisOscillatorSettings settings;
  settings.gain = 1.0f;
  settings.fade_out_percent = 0.0f;
  settings.frequency_min.output_frequency = 1.0f;
  settings.frequency_min.duration_ms = 1000.0f;
  settings.frequency_min.shape = HAPTIC_RENDERER_EMPHASIS_SHAPE_SQUARE;
  settings.frequency_max.output_frequency = 1.0f;
  settings.frequency_max.duration_ms = 1000.0f;
  settings.frequency_max.shape = HAPTIC_RENDERER_EMPHASIS_SHAPE_SAW;

  HapticRendererEmphasisOscillator oscillator;
  haptic_renderer_emphasis_oscillator_init(
      &oscillator, sample_rate, HAPTIC_RENDERER_MODE_SYNTHESIS, &settings);

  // A frequency of 0.5 should produce an equal mix of square and saw
  haptic_renderer_emphasis_oscillator_start_emphasis(&oscillator, 1.0, 0.5);

  check_oscillator_output(&oscillator, {1.0, 0.875, 0.75, 0.625, -0.5, -0.625, -0.75, -0.875});
  check_oscillator_output(&oscillator, {0.0, 0.0});
}

TEST(EmphasisOscillatorSynthesisTest, FadeToSilenceDuringEvent) {
  float sample_rate = 4.0;

  HapticRendererEmphasisOscillatorSettings settings;
  settings.gain = 1.0f;
  // Events should be faded by 100% over their duration, i.e. to silence
  settings.fade_out_percent = 100.0f;
  settings.frequency_min.output_frequency = 1.0f;
  settings.frequency_min.duration_ms = 1000.0f;
  settings.frequency_min.shape = HAPTIC_RENDERER_EMPHASIS_SHAPE_SQUARE;
  settings.frequency_max.output_frequency = 1.0f;
  settings.frequency_max.duration_ms = 2000.0f;
  settings.frequency_max.shape = HAPTIC_RENDERER_EMPHASIS_SHAPE_SAW;

  HapticRendererEmphasisOscillator oscillator;
  haptic_renderer_emphasis_oscillator_init(
      &oscillator, sample_rate, HAPTIC_RENDERER_MODE_SYNTHESIS, &settings);

  // Faded square over 1 second / 4 samples
  haptic_renderer_emphasis_oscillator_start_emphasis(&oscillator, 1.0, 0.0);
  check_oscillator_output(&oscillator, {1.0, 2.0 / 3.0, -1.0 / 3.0, 0.0});
  check_oscillator_output(&oscillator, {0.0, 0.0});

  // Faded square over 1 second / 4 samples with half event amplitude
  haptic_renderer_emphasis_oscillator_start_emphasis(&oscillator, 0.5, 0.0);
  check_oscillator_output(&oscillator, {0.5, 1.0 / 3.0, -0.5 / 3.0, 0.0});
  check_oscillator_output(&oscillator, {0.0, 0.0});

  // Faded half-speed saw over 2 seconds / 8 samples
  haptic_renderer_emphasis_oscillator_start_emphasis(&oscillator, 1.0, 1.0);
  check_oscillator_output(
      &oscillator, {1.0, 0.4285714626, 0.0, -0.2857142984, 0.4285714626, 0.142857149, 0.0});
  check_oscillator_output(&oscillator, {0.0, 0.0});
}

TEST(EmphasisOscillatorAmpCurveTest, FadeToSilenceDuringEvent) {
  float sample_rate = 4.0;

  HapticRendererEmphasisOscillatorSettings settings;
  settings.gain = 1.0f;
  // Events should be faded by 100% over their duration, i.e. to silence
  settings.fade_out_percent = 100.0f;
  settings.frequency_min.output_frequency = 1.0f;
  settings.frequency_min.duration_ms = 1000.0f;
  settings.frequency_min.shape = HAPTIC_RENDERER_EMPHASIS_SHAPE_SQUARE;
  settings.frequency_max.output_frequency = 1.0f;
  settings.frequency_max.duration_ms = 1500.0f;
  settings.frequency_max.shape = HAPTIC_RENDERER_EMPHASIS_SHAPE_SAW;

  HapticRendererEmphasisOscillator oscillator;
  haptic_renderer_emphasis_oscillator_init(
      &oscillator, sample_rate, HAPTIC_RENDERER_MODE_AMP_CURVE, &settings);

  // Faded over 1 second / 4 samples
  haptic_renderer_emphasis_oscillator_start_emphasis(&oscillator, 1.0, 0.0);
  check_oscillator_output(&oscillator, {1.0, 2.0 / 3.0, 1.0 / 3.0, 0.0});
  check_oscillator_output(&oscillator, {0.0, 0.0});

  // Faded square over 1 second / 4 samples with half event amplitude
  haptic_renderer_emphasis_oscillator_start_emphasis(&oscillator, 0.5, 0.0);
  check_oscillator_output(&oscillator, {0.5, 1.0 / 3.0, 0.5 / 3.0, 0.0});
  check_oscillator_output(&oscillator, {0.0, 0.0});

  // Faded over 1.5 seconds / 6 samples
  haptic_renderer_emphasis_oscillator_start_emphasis(&oscillator, 1.0, 1.0);
  check_oscillator_output(&oscillator, {1.0, 0.8, 0.6, 0.4, 0.2, 0.0});
  check_oscillator_output(&oscillator, {0.0, 0.0});
}

// Confirms that no assertions are triggered due to Nyquist violations
// when playing emphasis as an amplitude curve (because Nyquist violations
// only matter when playing as PCM).
TEST(EmphasisOscillatorAmpCurveTest, FrequencyRangeAssertionTest) {
  float sample_rate = 50.0;

  HapticRendererEmphasisOscillatorSettings settings;
  settings.gain = 1.0f;
  settings.fade_out_percent = 0.0f;
  // Sample rate is lower than both min and max output_frequency which
  // would be a Nyquist violation if we were playing as PCM, but as
  // we are playing as an amp curve, this is fine.
  settings.frequency_min.output_frequency = 55.0f; // > sample_rate
  settings.frequency_min.duration_ms = 36.4f;
  settings.frequency_min.shape = HAPTIC_RENDERER_EMPHASIS_SHAPE_SINE;
  settings.frequency_max.output_frequency = 165.5f; // > sample_rate
  settings.frequency_max.duration_ms = 12.1;
  settings.frequency_max.shape = HAPTIC_RENDERER_EMPHASIS_SHAPE_SQUARE;

  HapticRendererEmphasisOscillator oscillator;
  haptic_renderer_emphasis_oscillator_init(
      &oscillator, sample_rate, HAPTIC_RENDERER_MODE_AMP_CURVE, &settings);

  haptic_renderer_emphasis_oscillator_start_emphasis(&oscillator, 1.0, 0.0);
}

TEST(EmphasisShapeTest, Saw) {
  float allowed_error = 1.0e-9;
  HapticRendererEmphasisShape shape = HAPTIC_RENDERER_EMPHASIS_SHAPE_SAW;

  EXPECT_NEAR(haptic_renderer_emphasis_shape_process(shape, 0.0), 1.0, allowed_error);
  EXPECT_NEAR(haptic_renderer_emphasis_shape_process(shape, 0.25), 0.5, allowed_error);
  EXPECT_NEAR(haptic_renderer_emphasis_shape_process(shape, 0.5), 0.0, allowed_error);
  EXPECT_NEAR(haptic_renderer_emphasis_shape_process(shape, 0.75), -0.5, allowed_error);
  EXPECT_NEAR(haptic_renderer_emphasis_shape_process(shape, 1.0), -1.0, allowed_error);
}

TEST(EmphasisShapeTest, Sine) {
  float allowed_error = 1.0e-6;
  HapticRendererEmphasisShape shape = HAPTIC_RENDERER_EMPHASIS_SHAPE_SINE;

  EXPECT_NEAR(haptic_renderer_emphasis_shape_process(shape, 0.0), 0.0, allowed_error);
  EXPECT_NEAR(haptic_renderer_emphasis_shape_process(shape, 0.125), sinf(M_PI / 4), allowed_error);
  EXPECT_NEAR(haptic_renderer_emphasis_shape_process(shape, 0.25), sinf(M_PI / 2), allowed_error);
  EXPECT_NEAR(
      haptic_renderer_emphasis_shape_process(shape, 0.375), sinf(3 * M_PI / 4), allowed_error);
  EXPECT_NEAR(haptic_renderer_emphasis_shape_process(shape, 0.5), sinf(M_PI), allowed_error);
  EXPECT_NEAR(
      haptic_renderer_emphasis_shape_process(shape, 0.625), sinf(5 * M_PI / 4), allowed_error);
  EXPECT_NEAR(
      haptic_renderer_emphasis_shape_process(shape, 0.75), sinf(3 * M_PI / 2), allowed_error);
  EXPECT_NEAR(
      haptic_renderer_emphasis_shape_process(shape, 0.875), sinf(7 * M_PI / 4), allowed_error);
  EXPECT_NEAR(haptic_renderer_emphasis_shape_process(shape, 1.0), sinf(2 * M_PI), allowed_error);
}

TEST(EmphasisShapeTest, Square) {
  float allowed_error = 1.0e-9;
  HapticRendererEmphasisShape shape = HAPTIC_RENDERER_EMPHASIS_SHAPE_SQUARE;

  EXPECT_NEAR(haptic_renderer_emphasis_shape_process(shape, 0.0), 1.0, allowed_error);
  EXPECT_NEAR(haptic_renderer_emphasis_shape_process(shape, 0.25), 1.0, allowed_error);
  EXPECT_NEAR(haptic_renderer_emphasis_shape_process(shape, 0.5), -1.0, allowed_error);
  EXPECT_NEAR(haptic_renderer_emphasis_shape_process(shape, 0.75), -1.0, allowed_error);
  EXPECT_NEAR(haptic_renderer_emphasis_shape_process(shape, 1.0), -1.0, allowed_error);
}

TEST(EmphasisShapeTest, Triangle) {
  float allowed_error = 1.0e-9;
  HapticRendererEmphasisShape shape = HAPTIC_RENDERER_EMPHASIS_SHAPE_TRIANGLE;

  EXPECT_NEAR(haptic_renderer_emphasis_shape_process(shape, 0.0), 0.0, allowed_error);
  EXPECT_NEAR(haptic_renderer_emphasis_shape_process(shape, 0.125), 0.5, allowed_error);
  EXPECT_NEAR(haptic_renderer_emphasis_shape_process(shape, 0.25), 1.0, allowed_error);
  EXPECT_NEAR(haptic_renderer_emphasis_shape_process(shape, 0.375), 0.5, allowed_error);
  EXPECT_NEAR(haptic_renderer_emphasis_shape_process(shape, 0.5), 0.0, allowed_error);
  EXPECT_NEAR(haptic_renderer_emphasis_shape_process(shape, 0.625), -0.5, allowed_error);
  EXPECT_NEAR(haptic_renderer_emphasis_shape_process(shape, 0.75), -1.0, allowed_error);
  EXPECT_NEAR(haptic_renderer_emphasis_shape_process(shape, 0.875), -0.5, allowed_error);
  EXPECT_NEAR(haptic_renderer_emphasis_shape_process(shape, 1.0), 0.0, allowed_error);
}
