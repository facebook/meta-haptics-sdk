// Copyright (c) Meta Platforms, Inc. and affiliates.

// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

#include <gtest/gtest.h>
#include <openxr/openxr.h>
#include <chrono>

#include "haptic_renderer/render_mode.h"
#include "parametric_haptic_renderer/parametric_haptic_renderer.h"

using namespace std::chrono_literals;

/// Helper function for tests.
void testRenderedBatchSizes(
    const std::vector<ParametricHapticPoint>& amplitudePoints,
    const std::vector<ParametricHapticPoint>& frequencyPoints,
    const std::vector<ParametricHapticTransient>& transients,
    const std::vector<size_t>& expectedBatchSizes) {
  ParametricHapticRenderer renderer;

  RenderSettings renderSettings = {
      .continuous =
          {
              1.0, // gain
              0.5, // emphasis_ducking,
              55, // frequency_min,
              200, // frequency_max,
          },
      .emphasis = {
          1.0, // gain,
          0.0, // fade_out_percent,
          // frequency_min
          {
              55, // output_frequency,
              36.4, // duration_ms,
              HAPTIC_RENDERER_EMPHASIS_SHAPE_SINE, // shape
          },
          // frequency_max
          {
              165, // output_frequency,
              12.1, // duration_ms,
              HAPTIC_RENDERER_EMPHASIS_SHAPE_SQUARE, // shape
          },
      }};

  bool init(
      RenderSettings renderSettings,
      HapticRendererMode renderMode,
      float sampleRate,
      const std::vector<ParametricHapticPoint>& amplitudePoints,
      const std::vector<ParametricHapticPoint>& frequencyPoints,
      const std::vector<ParametricHapticTransient>& transients);

  EXPECT_TRUE(renderer.init(
      renderSettings,
      HAPTIC_RENDERER_MODE_SYNTHESIS,
      2000,
      amplitudePoints,
      frequencyPoints,
      transients));

  for (size_t i = 0; i < expectedBatchSizes.size() - 1; i++) {
    auto samples = renderer.renderNextBatch(100ms);
    EXPECT_EQ(samples.size(), expectedBatchSizes[i]);
  }

  auto samples = renderer.renderNextBatch(100ms);
  EXPECT_EQ(samples.size(), expectedBatchSizes.back());
}

class Renderer : public testing::Test {
 protected:
  void SetUp() override {}
};

/// Confirms that the output of the ParametricHapticRenderer is correct for the
/// most simple effect, a single ramp with two points. No frequency points or transients.
TEST_F(Renderer, SimpleAmplitudeRamp) {
  ParametricHapticRenderer renderer;

  std::vector<ParametricHapticPoint> amplitudePoints = {
      {.timeNs = 0, .value = 0.0}, {.timeNs = 190000000, .value = 1.0}};
  std::vector<ParametricHapticPoint> frequencyPoints = {};
  std::vector<ParametricHapticTransient> transients = {};

  RenderSettings renderSettings = {
      .continuous =
          {
              0.8, // gain
              0.8, // emphasis_ducking,
              160, // frequency_min,
              186, // frequency_max,
          },
      .emphasis = {
          1.0, // gain,
          0.0, // fade_out_percent,
          // frequency_min
          {
              170, // output_frequency,
              12, // duration_ms,
              HAPTIC_RENDERER_EMPHASIS_SHAPE_SINE, // shape
          },
          // frequency_max
          {
              185, // output_frequency,
              11.0, // duration_ms,
              HAPTIC_RENDERER_EMPHASIS_SHAPE_TRIANGLE, // shape
          },
      }};

  EXPECT_TRUE(renderer.init(
      renderSettings,
      HAPTIC_RENDERER_MODE_AMP_CURVE,
      2000,
      amplitudePoints,
      frequencyPoints,
      transients));

  auto amplitudes = renderer.renderNextBatch(100ms);
  EXPECT_EQ(amplitudes.size(), 200);

  // Assert that amplitudes are ramping up.
  EXPECT_TRUE(
      std::adjacent_find(amplitudes.begin(), amplitudes.end(), std::greater_equal<>()) ==
      amplitudes.end());

  amplitudes = renderer.renderNextBatch(100ms);
  EXPECT_EQ(amplitudes.size(), 180);
  EXPECT_TRUE(
      std::adjacent_find(amplitudes.begin(), amplitudes.end(), std::greater_equal<>()) ==
      amplitudes.end());
}

// Multiple amplitude points. Some update periods have no points in them, some multiple points.
// No frequency points or transients.
TEST_F(Renderer, MultipleAmplitudePoints) {
  ParametricHapticRenderer renderer;

  std::vector<ParametricHapticPoint> amplitudePoints = {
      {.timeNs = 0, .value = 0.0},
      {.timeNs = 90000000, .value = 1.0},
      {.timeNs = 210000000, .value = 1.0},
      {.timeNs = 240000000, .value = 0.5},
      {.timeNs = 270000000, .value = 1.0}};
  std::vector<ParametricHapticPoint> frequencyPoints = {};
  std::vector<ParametricHapticTransient> transients = {};

  RenderSettings renderSettings = {
      .continuous =
          {
              0.8, // gain
              0.8, // emphasis_ducking,
              160, // frequency_min,
              186, // frequency_max,
          },
      .emphasis = {
          1.0, // gain,
          0.0, // fade_out_percent,
          // frequency_min
          {
              170, // output_frequency,
              12, // duration_ms,
              HAPTIC_RENDERER_EMPHASIS_SHAPE_SINE, // shape
          },
          // frequency_max
          {
              185, // output_frequency,
              11.0, // duration_ms,
              HAPTIC_RENDERER_EMPHASIS_SHAPE_TRIANGLE, // shape
          },
      }};

  EXPECT_TRUE(renderer.init(
      renderSettings,
      HAPTIC_RENDERER_MODE_AMP_CURVE,
      2000,
      amplitudePoints,
      frequencyPoints,
      transients));

  auto amplitudes = renderer.renderNextBatch(100ms);
  EXPECT_EQ(amplitudes.size(), 200);
  EXPECT_TRUE(
      std::adjacent_find(amplitudes.begin(), amplitudes.end(), std::greater<>()) ==
      amplitudes.end());

  amplitudes = renderer.renderNextBatch(100ms);
  EXPECT_EQ(amplitudes.size(), 200);
  EXPECT_TRUE(
      std::adjacent_find(amplitudes.begin(), amplitudes.end(), std::not_equal_to<>()) ==
      amplitudes.end());

  amplitudes = renderer.renderNextBatch(100ms);
  EXPECT_EQ(amplitudes.size(), 140);
}

// Amplitude points, frequency points, and transients. All at different times.
// Frequency envelope has shorter duration than amplitude envelope.
TEST_F(Renderer, AllTypes) {
  std::vector<ParametricHapticPoint> amplitudePoints = {
      {.timeNs = 0, .value = 0.0},
      {.timeNs = 80000000, .value = 1.0},
      {.timeNs = 190000000, .value = 0.8}};
  std::vector<ParametricHapticPoint> frequencyPoints = {
      {.timeNs = 0, .value = 1.0}, {.timeNs = 90000000, .value = 0.0}};
  std::vector<ParametricHapticTransient> transients = {
      {.timeNs = 85000000, .amplitude = 1.0, .frequency = 1.0},
      {.timeNs = 170000000, .amplitude = 1.0, .frequency = 1.0}};

  testRenderedBatchSizes(amplitudePoints, frequencyPoints, transients, {200, 180});
}

TEST_F(Renderer, OneFrequencyPoint) {
  std::vector<ParametricHapticPoint> amplitudePoints = {
      {.timeNs = 0, .value = 0.0}, {.timeNs = 90000000, .value = 1.0}};
  std::vector<ParametricHapticPoint> frequencyPoints = {{.timeNs = 0, .value = 0.9}};

  testRenderedBatchSizes(amplitudePoints, frequencyPoints, {}, {180});
}

TEST_F(Renderer, ManyFrequencyPoints) {
  std::vector<ParametricHapticPoint> amplitudePoints = {
      {.timeNs = 0, .value = 0.0}, {.timeNs = 90000000, .value = 1.0}};
  std::vector<ParametricHapticPoint> frequencyPoints = {
      {.timeNs = 0, .value = 1.0},
      {.timeNs = 25000000, .value = 0.0},
      {.timeNs = 50000000, .value = 1.0},
      {.timeNs = 75000000, .value = 0.0},
      {.timeNs = 90000000, .value = 1.0}};

  testRenderedBatchSizes(amplitudePoints, frequencyPoints, {}, {180});
}

TEST_F(Renderer, AmplitudePointAtEndOfUpdate) {
  std::vector<ParametricHapticPoint> amplitudePoints = {
      {.timeNs = 0, .value = 0.0}, {.timeNs = 100000000, .value = 1.0}};

  testRenderedBatchSizes(amplitudePoints, {}, {}, {200});
}

TEST_F(Renderer, AmplitudePointAtEndOfUpdate_Longer) {
  std::vector<ParametricHapticPoint> amplitudePoints = {
      {.timeNs = 0, .value = 0.0},
      {.timeNs = 100000000, .value = 1.0},
      {.timeNs = 200000000, .value = 0.0}};

  testRenderedBatchSizes(amplitudePoints, {}, {}, {200, 200});
}
