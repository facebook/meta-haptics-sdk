// Copyright (c) Meta Platforms, Inc. and affiliates.

// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

#pragma once

#ifdef __cplusplus
extern "C" {
#endif

/// An oscillator that provides continuously ramping output between 0 and 1
typedef struct {
  float phase;
  float increment;
} HapticRendererPhasor;

void haptic_renderer_phasor_init_default(HapticRendererPhasor* self);

/// Initializes a Phasor with the given frequency
void haptic_renderer_phasor_init_with_frequency(
    HapticRendererPhasor* self,
    float frequency,
    float sample_rate);

/// Resets the phasor to its initial state
void haptic_renderer_phasor_reset(HapticRendererPhasor* self);

/// Immediately sets the phase to the provided value
void haptic_renderer_phasor_set_phase(HapticRendererPhasor* self, float phase);

/// Sets the current frequency
void haptic_renderer_phasor_set_frequency(
    HapticRendererPhasor* self,
    float frequency,
    float sample_rate);

/// Gets the next sample of output
float haptic_renderer_phasor_process(HapticRendererPhasor* self);

#ifdef __cplusplus
}
#endif
