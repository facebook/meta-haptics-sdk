// Copyright (c) Meta Platforms, Inc. and affiliates.

// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

#include <gtest/gtest.h>

#include "haptic_renderer/ramped_value.h"

void check_ramp_output(HapticRendererRampedValue* ramp, const std::vector<float>& expected_output) {
  for (float value : expected_output) {
    ASSERT_EQ(haptic_renderer_ramped_value_process(ramp), value);
  }
}

TEST(RampedValueTest, NotRampingByDefault) {
  HapticRendererRampedValue ramp;
  haptic_renderer_ramped_value_init_default(&ramp);
  ASSERT_FALSE(haptic_renderer_ramped_value_is_ramping(&ramp));
  ASSERT_EQ(haptic_renderer_ramped_value_process(&ramp), 0.0);
  ASSERT_EQ(haptic_renderer_ramped_value_process(&ramp), 0.0);
}

TEST(RampedValueTest, RampUpThenBackDown) {
  float sample_rate = 1.0;
  HapticRendererRampedValue ramp;
  haptic_renderer_ramped_value_init_with_value(&ramp, 1.0f);

  haptic_renderer_ramped_value_ramp_to_value(&ramp, 2.0, 3.0, sample_rate);

  ASSERT_TRUE(haptic_renderer_ramped_value_is_ramping(&ramp));
  check_ramp_output(&ramp, {1.0, 1.5, 2.0});

  ASSERT_FALSE(haptic_renderer_ramped_value_is_ramping(&ramp));
  check_ramp_output(&ramp, {2.0, 2.0, 2.0});

  haptic_renderer_ramped_value_ramp_to_value(&ramp, 0.0, 3.0, sample_rate);

  ASSERT_TRUE(haptic_renderer_ramped_value_is_ramping(&ramp));
  check_ramp_output(&ramp, {2.0, 1.0, 0.0});

  ASSERT_FALSE(haptic_renderer_ramped_value_is_ramping(&ramp));
  check_ramp_output(&ramp, {0.0, 0.0, 0.0});
}

TEST(RampedValueTest, RampUpThenBackDownBeforeTheUpRampFinished) {
  float sample_rate = 1.0;
  HapticRendererRampedValue ramp;
  haptic_renderer_ramped_value_init_with_value(&ramp, 0.0f);

  haptic_renderer_ramped_value_ramp_to_value(&ramp, 1.0, 5.0, sample_rate);

  ASSERT_TRUE(haptic_renderer_ramped_value_is_ramping(&ramp));
  check_ramp_output(&ramp, {0.0, 0.25});

  haptic_renderer_ramped_value_ramp_to_value(&ramp, 0.0, 3.0, sample_rate);

  ASSERT_TRUE(haptic_renderer_ramped_value_is_ramping(&ramp));
  check_ramp_output(&ramp, {0.5, 0.25, 0.0});

  ASSERT_FALSE(haptic_renderer_ramped_value_is_ramping(&ramp));
  check_ramp_output(&ramp, {0.0, 0.0, 0.0});
}

TEST(RampedValueTest, RampWithEndNotOnSampleBoundary) {
  float sample_rate = 1.0;
  HapticRendererRampedValue ramp;
  haptic_renderer_ramped_value_init_with_value(&ramp, 0.0f);

  // Ramp to 1.0 over 5.5 seconds, or 5.5 samples in this test case.
  // This should result in 5 steps to the target value,
  // with the final step having a truncated increment.
  haptic_renderer_ramped_value_ramp_to_value(&ramp, 1.0, 5.5, sample_rate);

  // The first output is the ramp start value
  ASSERT_EQ(haptic_renderer_ramped_value_process(&ramp), 0.0);

  // Subsequent outputs match the trajectory of the requested ramp
  float allowed_error = 1.0e-7;
  ASSERT_NEAR(haptic_renderer_ramped_value_process(&ramp), 1.0 / 4.5, allowed_error);
  ASSERT_NEAR(haptic_renderer_ramped_value_process(&ramp), 2.0 / 4.5, allowed_error);
  ASSERT_NEAR(haptic_renderer_ramped_value_process(&ramp), 3.0 / 4.5, allowed_error);
  ASSERT_NEAR(haptic_renderer_ramped_value_process(&ramp), 4.0 / 4.5, allowed_error);

  // The final output is the ramp end value
  ASSERT_EQ(haptic_renderer_ramped_value_process(&ramp), 1.0);
}
