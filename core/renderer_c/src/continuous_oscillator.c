// Copyright (c) Meta Platforms, Inc. and affiliates.

// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

#ifndef _USE_MATH_DEFINES
#define _USE_MATH_DEFINES // for M_PI
#endif

#include "haptic_renderer/continuous_oscillator.h"

#include "haptic_renderer/phasor.h"
#include "haptic_renderer/ramped_value.h"
#include "haptic_renderer/render_mode.h"

#include <assert.h>
#include <math.h>

void haptic_renderer_continuous_oscillator_init(
    HapticRendererContinuousOscillator* self,
    float sample_rate,
    HapticRendererMode render_mode,
    HapticRendererContinuousOscillatorSettings* settings) {
  self->sample_rate = sample_rate;
  self->render_mode = render_mode;
  self->emphasis_ducking = settings->emphasis_ducking;
  haptic_renderer_phasor_init_default(&self->phasor);
  haptic_renderer_ramped_value_init_default(&self->amplitude);
  haptic_renderer_ramped_value_init_default(&self->frequency);
  self->amplitude_gain = settings->gain;
  self->ducking_gain = 1.0f;
  assert(settings->frequency_min <= settings->frequency_max);
  assert(render_mode == HAPTIC_RENDERER_MODE_AMP_CURVE || sample_rate > settings->frequency_max);
  self->frequency_min = settings->frequency_min;
  self->frequency_range = settings->frequency_max - settings->frequency_min;
  haptic_renderer_continuous_oscillator_reset(self);
}

void haptic_renderer_continuous_oscillator_reset(HapticRendererContinuousOscillator* self) {
  haptic_renderer_phasor_reset(&self->phasor);
  haptic_renderer_ramped_value_set_value(&self->amplitude, 0.0f);
  float start_frequency = self->frequency_min + 0.5f * self->frequency_range;
  haptic_renderer_ramped_value_set_value(&self->frequency, start_frequency);
  self->ducking_gain = 1.0f;
}

void haptic_renderer_continuous_oscillator_set_amplitude(
    HapticRendererContinuousOscillator* self,
    float amplitude,
    float duration) {
  haptic_renderer_ramped_value_ramp_to_value(
      &self->amplitude, amplitude * self->amplitude_gain, duration, self->sample_rate);
}

void haptic_renderer_continuous_oscillator_set_frequency(
    HapticRendererContinuousOscillator* self,
    float frequency,
    float duration) {
  float target_frequency = self->frequency_min + frequency * self->frequency_range;
  haptic_renderer_ramped_value_ramp_to_value(
      &self->frequency, target_frequency, duration, self->sample_rate);
}

void haptic_renderer_continuous_oscillator_set_emphasis_is_active(
    HapticRendererContinuousOscillator* self,
    int active) {
  self->ducking_gain = active ? self->emphasis_ducking : 1.0f;
}

float haptic_renderer_continuous_oscillator_process(HapticRendererContinuousOscillator* self) {
  float amplitude = haptic_renderer_ramped_value_process(&self->amplitude) * self->ducking_gain;

  switch (self->render_mode) {
    case HAPTIC_RENDERER_MODE_AMP_CURVE:
      return amplitude;
    case HAPTIC_RENDERER_MODE_SYNTHESIS: {
      float frequency = haptic_renderer_ramped_value_process(&self->frequency);

      haptic_renderer_phasor_set_frequency(&self->phasor, frequency, self->sample_rate);
      float phase = haptic_renderer_phasor_process(&self->phasor);
      float sine = sinf(phase * 2.0f * (float)M_PI);

      return amplitude * sine;
    }
    default:
      return 0.0f;
  }
}
