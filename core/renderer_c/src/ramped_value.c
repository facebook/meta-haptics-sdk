// Copyright (c) Meta Platforms, Inc. and affiliates.

// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

#include "haptic_renderer/ramped_value.h"

#include <math.h>

void haptic_renderer_ramped_value_init_default(HapticRendererRampedValue* self) {
  *self = (HapticRendererRampedValue){0.0, 0.0, 0.0, 0.0};
}

void haptic_renderer_ramped_value_init_with_value(HapticRendererRampedValue* self, float value) {
  *self = (HapticRendererRampedValue){value, value, 0.0, 0.0};
}

void haptic_renderer_ramped_value_set_value(HapticRendererRampedValue* self, float value) {
  self->current = value;
  self->target = value;
  self->samples_remaining_to_target = 0.0;
}

void haptic_renderer_ramped_value_ramp_to_value(
    HapticRendererRampedValue* self,
    float target,
    float duration,
    float sample_rate) {
  self->target = target;

  float samples_remaining_to_target = duration * sample_rate;
  if (samples_remaining_to_target <= 1.0 || fabsf(self->current - target) < 1.0e-6) {
    // If the ramp duration is 0 or 1 samples, or if the current value is effectively the same as
    // the target, then set the ramp as already finished.
    self->current = target;
    self->samples_remaining_to_target = 0.0;
  } else {
    // The current value is returned as the first sample in the ramp, so deduct one from the samples
    // remaining count
    self->samples_remaining_to_target = samples_remaining_to_target - 1.0f;
    self->ramp_increment = (target - self->current) / self->samples_remaining_to_target;
  }
}

float haptic_renderer_ramped_value_process(HapticRendererRampedValue* self) {
  float result = self->current;

  if (haptic_renderer_ramped_value_is_ramping(self)) {
    self->samples_remaining_to_target -= 1.0;
    self->current = self->target - self->samples_remaining_to_target * self->ramp_increment;
  } else {
    self->current = self->target;
  }

  return result;
}

int haptic_renderer_ramped_value_is_ramping(HapticRendererRampedValue* self) {
  // The ramp has finished as soon as 'samples remaining' is below 1.
  // Ramps that end between samples get truncated.
  return self->samples_remaining_to_target >= 1.0;
}

float haptic_renderer_ramped_value_target_value(HapticRendererRampedValue* self) {
  return self->target;
}
