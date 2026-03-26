// Copyright (c) Meta Platforms, Inc. and affiliates.

// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

#pragma once

#include "continuous_oscillator.h"
#include "emphasis_oscillator.h"
#include "render_mode.h"

#ifdef __cplusplus
extern "C" {
#endif

/// Renders sparse haptic events into a float data stream intended for actuator playback
///
/// The renderer contains oscillators that respond to continuous and emphasis events, and produces
/// rendered float samples one at a time as output, with the output range depending on the
/// HapticRendererMode that's being used.
///
/// For the input, the sparse haptic events need to be passed to the renderer with
/// haptic_renderer_start_amplitude_ramp(), haptic_renderer_start_frequency_ramp() and
/// haptic_renderer_start_emphasis().
/// For the output, the samples can be retrieved with haptic_renderer_process().
typedef struct {
  HapticRendererContinuousOscillator continuous_oscillator;
  HapticRendererEmphasisOscillator emphasis_oscillator;
} HapticRenderer;

/// Initializes a new HapticRenderer with the given settings
void haptic_renderer_init(
    HapticRenderer* self,
    float sample_rate,
    HapticRendererMode render_mode,
    HapticRendererContinuousOscillatorSettings* continuous_settings,
    HapticRendererEmphasisOscillatorSettings* emphasis_settings);

/// Resets the renderer to its initial settings
void haptic_renderer_reset(HapticRenderer* self);

void haptic_renderer_start_amplitude_ramp(HapticRenderer* self, float target, float duration);

void haptic_renderer_start_frequency_ramp(HapticRenderer* self, float target, float duration);

void haptic_renderer_start_emphasis(HapticRenderer* self, float amplitude, float frequency);

/// Provides a sample of output
float haptic_renderer_process(HapticRenderer* self);

HapticRendererMode haptic_renderer_render_mode(HapticRenderer* self);

float haptic_renderer_sample_rate(HapticRenderer* self);

#ifdef __cplusplus
}
#endif
