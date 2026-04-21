// Copyright (c) Meta Platforms, Inc. and affiliates.

// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

#include "haptic_renderer/renderer.h"

#include "haptic_renderer/continuous_oscillator.h"
#include "haptic_renderer/emphasis_oscillator.h"

void haptic_renderer_init(
    HapticRenderer* self,
    float sample_rate,
    HapticRendererMode render_mode,
    HapticRendererContinuousOscillatorSettings* continuous_settings,
    HapticRendererEmphasisOscillatorSettings* emphasis_settings) {
  haptic_renderer_continuous_oscillator_init(
      &self->continuous_oscillator, sample_rate, render_mode, continuous_settings);
  haptic_renderer_emphasis_oscillator_init(
      &self->emphasis_oscillator, sample_rate, render_mode, emphasis_settings);
}

void haptic_renderer_reset(HapticRenderer* self) {
  haptic_renderer_continuous_oscillator_reset(&self->continuous_oscillator);
  haptic_renderer_emphasis_oscillator_reset(&self->emphasis_oscillator);
}

void haptic_renderer_start_amplitude_ramp(HapticRenderer* self, float target, float duration) {
  haptic_renderer_continuous_oscillator_set_amplitude(
      &self->continuous_oscillator, target, duration);
}

void haptic_renderer_start_frequency_ramp(HapticRenderer* self, float target, float duration) {
  haptic_renderer_continuous_oscillator_set_frequency(
      &self->continuous_oscillator, target, duration);
}

void haptic_renderer_start_emphasis(HapticRenderer* self, float amplitude, float frequency) {
  haptic_renderer_emphasis_oscillator_start_emphasis(
      &self->emphasis_oscillator, amplitude, frequency);
}

float haptic_renderer_process(HapticRenderer* self) {
  haptic_renderer_continuous_oscillator_set_emphasis_is_active(
      &self->continuous_oscillator,
      haptic_renderer_emphasis_oscillator_is_active(&self->emphasis_oscillator));

  float continuous = haptic_renderer_continuous_oscillator_process(&self->continuous_oscillator);
  float emphasis = haptic_renderer_emphasis_oscillator_process(&self->emphasis_oscillator);

  return continuous + emphasis;
}

HapticRendererMode haptic_renderer_render_mode(HapticRenderer* self) {
  return self->continuous_oscillator.render_mode;
}

float haptic_renderer_sample_rate(HapticRenderer* self) {
  return self->continuous_oscillator.sample_rate;
}
