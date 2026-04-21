// Copyright (c) Meta Platforms, Inc. and affiliates.

// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

#ifndef _USE_MATH_DEFINES
#define _USE_MATH_DEFINES // for M_PI
#endif

#include "haptic_renderer/emphasis_oscillator.h"
#include "haptic_renderer/phasor.h"
#include "haptic_renderer/ramped_value.h"

#include <assert.h>
#include <math.h>

// Linearly interpolate between two values
float haptic_renderer_lerp(float a, float b, float amount) {
  return a + (b - a) * amount;
}

float haptic_renderer_emphasis_shape_process(HapticRendererEmphasisShape shape, float phase) {
  switch (shape) {
    case HAPTIC_RENDERER_EMPHASIS_SHAPE_SAW:
      return haptic_renderer_lerp(1.0f, -1.0f, phase);
    case HAPTIC_RENDERER_EMPHASIS_SHAPE_SINE:
      return sinf(2.0f * (float)M_PI * phase);
    case HAPTIC_RENDERER_EMPHASIS_SHAPE_SQUARE:
      return phase < 0.5f ? 1.0f : -1.0f;
    case HAPTIC_RENDERER_EMPHASIS_SHAPE_TRIANGLE:
      // If you're curious about how this works, you can play around with it here:
      // https://www.desmos.com/calculator/qhsinfa7bm
      return 1.0f - fabsf(2.0f - fabsf(3.0f - 4.0f * phase));
    default:
      return 0.0f;
  }
}

void haptic_renderer_emphasis_oscillator_init(
    HapticRendererEmphasisOscillator* self,
    float sample_rate,
    HapticRendererMode render_mode,
    HapticRendererEmphasisOscillatorSettings* settings) {
  self->sample_rate = sample_rate;
  self->render_mode = render_mode;
  haptic_renderer_phasor_init_default(&self->phasor);
  self->samples_remaining = 0;
  self->gain_start = settings->gain;
  self->gain_end = settings->gain * (1.0f - (settings->fade_out_percent / 100.0f));
  haptic_renderer_ramped_value_init_with_value(&self->gain, 0.0f);
  assert(settings->frequency_min.output_frequency <= settings->frequency_max.output_frequency);
  self->frequency_min = settings->frequency_min.output_frequency;
  self->frequency_range =
      settings->frequency_max.output_frequency - settings->frequency_min.output_frequency;
  self->duration_min = settings->frequency_min.duration_ms / 1000.0f;
  self->duration_range =
      (settings->frequency_max.duration_ms - settings->frequency_min.duration_ms) / 1000.0f;
  self->shape_min = settings->frequency_min.shape;
  self->shape_max = settings->frequency_max.shape;
  self->shape_gain_min = 1.0f;
  self->shape_gain_max = 0.0f;
}

void haptic_renderer_emphasis_oscillator_reset(HapticRendererEmphasisOscillator* self) {
  haptic_renderer_phasor_reset(&self->phasor);
  haptic_renderer_ramped_value_set_value(&self->gain, 0.0f);
  self->samples_remaining = 0;
}

void haptic_renderer_emphasis_oscillator_start_emphasis(
    HapticRendererEmphasisOscillator* self,
    float amplitude,
    float frequency) {
  // Figure out the event's duration based on its frequency value
  float event_duration = self->duration_min + frequency * self->duration_range;
  int duration_in_samples = (int)(event_duration * self->sample_rate);
  self->samples_remaining = duration_in_samples;

  // Reset the phasor for the beginning of the event
  haptic_renderer_phasor_set_phase(&self->phasor, 0.0f);

  // We only want to set the frequency of the phasor, if we are rendering PCM
  if (self->render_mode == HAPTIC_RENDERER_MODE_SYNTHESIS) {
    haptic_renderer_phasor_set_frequency(
        &self->phasor, self->frequency_min + frequency * self->frequency_range, self->sample_rate);
  }

  // Set up the gain ramp
  float gain_start = self->gain_start * amplitude;
  float gain_end = self->gain_end * amplitude;
  haptic_renderer_ramped_value_set_value(&self->gain, gain_start);
  haptic_renderer_ramped_value_ramp_to_value(
      &self->gain, gain_end, event_duration, self->sample_rate);

  // Set up a linear cross-fade between the min and max shapes
  self->shape_gain_min = 1.0f - frequency;
  self->shape_gain_max = frequency;
}

float haptic_renderer_emphasis_oscillator_process(HapticRendererEmphasisOscillator* self) {
  if (!haptic_renderer_emphasis_oscillator_is_active(self)) {
    return 0.0f;
  }

  self->samples_remaining -= 1;

  float gain = haptic_renderer_ramped_value_process(&self->gain);

  switch (self->render_mode) {
    case HAPTIC_RENDERER_MODE_AMP_CURVE:
      return gain;
    case HAPTIC_RENDERER_MODE_SYNTHESIS: {
      float phase = haptic_renderer_phasor_process(&self->phasor);

      float shaped_min =
          haptic_renderer_emphasis_shape_process(self->shape_min, phase) * self->shape_gain_min;
      float shaped_max =
          haptic_renderer_emphasis_shape_process(self->shape_max, phase) * self->shape_gain_max;

      return (shaped_min + shaped_max) * gain;
    }
    default:
      return 0.0f;
  }
}

int haptic_renderer_emphasis_oscillator_is_active(HapticRendererEmphasisOscillator* self) {
  return self->samples_remaining > 0;
}
