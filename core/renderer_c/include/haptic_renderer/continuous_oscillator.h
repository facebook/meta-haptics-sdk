// Copyright (c) Meta Platforms, Inc. and affiliates.

// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

#pragma once

#include "phasor.h"
#include "ramped_value.h"
#include "render_mode.h"

#ifdef __cplusplus
extern "C" {
#endif

/// The settings used by HapticRendererContinuousOscillator
typedef struct {
  /// The gain of the continuous oscillator
  ///
  /// Range: 0->1
  float gain;

  /// The amount of ducking to apply to the continuous signal when emphasis is active
  ///
  /// This is an additional gain applied to the continuous signal, for example a value of '1' means
  /// 'no ducking', while '0' means 'drop to silence while an emphasis event is active'.
  float emphasis_ducking;

  /// The minimum frequency in Hz of the continuous oscillator.
  float frequency_min;

  /// The maximum frequency in Hz of the continuous oscillator.
  float frequency_max;
} HapticRendererContinuousOscillatorSettings;

/// Renders a haptic's continuous signal using a sine wave with modulated amplitude and frequency
typedef struct {
  /// The sample rate that the oscillator runs at
  float sample_rate;

  /// The type of output that should be provided by the oscillator
  HapticRendererMode render_mode;

  /// The gain applied to the output when ducking is active
  float emphasis_ducking;

  /// Used to generate the oscillator's phase
  HapticRendererPhasor phasor;

  /// The current oscillator amplitude
  HapticRendererRampedValue amplitude;

  /// The current oscillator frequency
  HapticRendererRampedValue frequency;

  /// The user-defined overall gain used by the oscillator
  float amplitude_gain;

  /// Either 1.0 (when ducking is inactive), or emphasis_ducking when ducking is active
  float ducking_gain;

  /// The minimum oscillator frequency, corresponding to an input haptic frequency of '0'
  float frequency_min;

  /// The frequency range, with min + frequency_range corresponding to an input frequency of '1'
  float frequency_range;
} HapticRendererContinuousOscillator;

/// Initializes a ContinuousOscillator with the given sample rate, render mode, and settings
void haptic_renderer_continuous_oscillator_init(
    HapticRendererContinuousOscillator* self,
    float sample_rate,
    HapticRendererMode render_mode,
    HapticRendererContinuousOscillatorSettings* settings);

/// Resets the oscillator to its initial settings
void haptic_renderer_continuous_oscillator_reset(HapticRendererContinuousOscillator* self);

void haptic_renderer_continuous_oscillator_set_amplitude(
    HapticRendererContinuousOscillator* self,
    float amplitude,
    float duration);

void haptic_renderer_continuous_oscillator_set_frequency(
    HapticRendererContinuousOscillator* self,
    float frequency,
    float duration);

void haptic_renderer_continuous_oscillator_set_emphasis_is_active(
    HapticRendererContinuousOscillator* self,
    int active);

float haptic_renderer_continuous_oscillator_process(HapticRendererContinuousOscillator* self);

#ifdef __cplusplus
}
#endif
