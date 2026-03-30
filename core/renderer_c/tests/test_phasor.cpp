// Copyright (c) Meta Platforms, Inc. and affiliates.

// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

#include <gtest/gtest.h>

#include "haptic_renderer/phasor.h"

TEST(PhasorTest, Frequency0) {
  float allowed_error = 1.0e-9f;
  float sample_rate = 4.0f;

  HapticRendererPhasor phasor;
  haptic_renderer_phasor_init_with_frequency(&phasor, 0.0f, sample_rate);
  EXPECT_NEAR(haptic_renderer_phasor_process(&phasor), 0.0f, allowed_error);
  EXPECT_NEAR(haptic_renderer_phasor_process(&phasor), 0.0f, allowed_error);
  EXPECT_NEAR(haptic_renderer_phasor_process(&phasor), 0.0f, allowed_error);
  EXPECT_NEAR(haptic_renderer_phasor_process(&phasor), 0.0f, allowed_error);
}

TEST(PhasorTest, Frequency1) {
  float allowed_error = 1.0e-9f;
  float sample_rate = 4.0f;

  HapticRendererPhasor phasor;
  haptic_renderer_phasor_init_with_frequency(&phasor, 1.0f, sample_rate);
  EXPECT_NEAR(haptic_renderer_phasor_process(&phasor), 0.0f, allowed_error);
  EXPECT_NEAR(haptic_renderer_phasor_process(&phasor), 0.25f, allowed_error);
  EXPECT_NEAR(haptic_renderer_phasor_process(&phasor), 0.5f, allowed_error);
  EXPECT_NEAR(haptic_renderer_phasor_process(&phasor), 0.75f, allowed_error);
  EXPECT_NEAR(haptic_renderer_phasor_process(&phasor), 0.0f, allowed_error);
  EXPECT_NEAR(haptic_renderer_phasor_process(&phasor), 0.25f, allowed_error);
}

TEST(PhasorTest, SampleRate8ChangingFrequency) {
  float allowed_error = 1.0e-9f;
  float sample_rate = 8.0f;

  HapticRendererPhasor phasor;
  haptic_renderer_phasor_init_with_frequency(&phasor, 1.0f, sample_rate);
  EXPECT_NEAR(haptic_renderer_phasor_process(&phasor), 0.0f, allowed_error);
  EXPECT_NEAR(haptic_renderer_phasor_process(&phasor), 0.125f, allowed_error);
  EXPECT_NEAR(haptic_renderer_phasor_process(&phasor), 0.25f, allowed_error);
  EXPECT_NEAR(haptic_renderer_phasor_process(&phasor), 0.375f, allowed_error);

  haptic_renderer_phasor_set_frequency(&phasor, 2.0f, sample_rate);
  EXPECT_NEAR(haptic_renderer_phasor_process(&phasor), 0.5f, allowed_error);
  EXPECT_NEAR(haptic_renderer_phasor_process(&phasor), 0.75f, allowed_error);
  EXPECT_NEAR(haptic_renderer_phasor_process(&phasor), 0.0f, allowed_error);
  EXPECT_NEAR(haptic_renderer_phasor_process(&phasor), 0.25f, allowed_error);
}

TEST(PhasorTest, SampleRate50ChangingFrequency) {
  float allowed_error = 1.0e-7f;
  float sample_rate = 50.0f;

  HapticRendererPhasor phasor;
  haptic_renderer_phasor_init_with_frequency(&phasor, 1.0f, sample_rate);
  EXPECT_NEAR(haptic_renderer_phasor_process(&phasor), 0.00f, allowed_error);
  EXPECT_NEAR(haptic_renderer_phasor_process(&phasor), 0.02f, allowed_error);
  EXPECT_NEAR(haptic_renderer_phasor_process(&phasor), 0.04f, allowed_error);
  EXPECT_NEAR(haptic_renderer_phasor_process(&phasor), 0.06f, allowed_error);

  haptic_renderer_phasor_set_frequency(&phasor, 2.0f, sample_rate);
  EXPECT_NEAR(haptic_renderer_phasor_process(&phasor), 0.08f, allowed_error);
  EXPECT_NEAR(haptic_renderer_phasor_process(&phasor), 0.12f, allowed_error);
  EXPECT_NEAR(haptic_renderer_phasor_process(&phasor), 0.16f, allowed_error);
  EXPECT_NEAR(haptic_renderer_phasor_process(&phasor), 0.20f, allowed_error);
}
