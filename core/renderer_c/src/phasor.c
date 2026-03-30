// Copyright (c) Meta Platforms, Inc. and affiliates.

// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

#include "haptic_renderer/phasor.h"

#include <assert.h>

void haptic_renderer_phasor_init_default(HapticRendererPhasor* self) {
  *self = (HapticRendererPhasor){0.0f, 0.0f};
}

void haptic_renderer_phasor_init_with_frequency(
    HapticRendererPhasor* self,
    float frequency,
    float sample_rate) {
  haptic_renderer_phasor_init_default(self);
  haptic_renderer_phasor_set_frequency(self, frequency, sample_rate);
}

void haptic_renderer_phasor_reset(HapticRendererPhasor* self) {
  haptic_renderer_phasor_init_default(self);
}

void haptic_renderer_phasor_set_phase(HapticRendererPhasor* self, float phase) {
  self->phase = phase;
}

void haptic_renderer_phasor_set_frequency(
    HapticRendererPhasor* self,
    float frequency,
    float sample_rate) {
  self->increment = frequency / sample_rate;
  assert(self->increment >= 0.0f && self->increment <= 1.0f);
}

float haptic_renderer_phasor_process(HapticRendererPhasor* self) {
  float result = self->phase;
  self->phase += self->increment;
  if (self->phase >= 1.0f) {
    self->phase -= 1.0f;
  }
  return result;
}
