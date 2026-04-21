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

/// The oscillator shape to use when rendering an emphasis event
///
/// See HapticRendererEmphasisFrequencySettings.
typedef enum {
  /// A descending saw shape
  HAPTIC_RENDERER_EMPHASIS_SHAPE_SAW,

  /// A sine wave
  HAPTIC_RENDERER_EMPHASIS_SHAPE_SINE,

  /// A rectangular wave starting with 'up'
  HAPTIC_RENDERER_EMPHASIS_SHAPE_SQUARE,

  /// A triangular wave, starting with the rising section
  HAPTIC_RENDERER_EMPHASIS_SHAPE_TRIANGLE
} HapticRendererEmphasisShape;

float haptic_renderer_emphasis_shape_process(HapticRendererEmphasisShape shape, float phase);

/// The settings to use for either minimum or maximum emphasis frequency
///
/// See HapticRendererEmphasisOscillatorSettings.
typedef struct {
  /// The output frequency in hertz of the emphasis oscillator
  float output_frequency;

  /// The duration in milliseconds of the emphasis event
  float duration_ms;

  /// The shape of the emphasis oscillator's output.
  HapticRendererEmphasisShape shape;
} HapticRendererEmphasisFrequencySettings;

/// The settings used by HapticRendererEmphasisOscillator
typedef struct {
  /// The gain of the emphasis oscillator
  ///
  /// Range: 0->1
  float gain;

  /// The amount that the emphasis signal should fade out while it's active
  ///
  /// The emphasis event will start at maximum amplitude and then fade to a percentage of
  /// the starting amplitude over its duration.
  /// For example, a value of '100' means "fade to silence over the event's duration",
  /// while '0' means "don't fade out, stay at maximum amplitude".
  float fade_out_percent;

  /// The settings to use when the emphasis frequency is at its minimum
  HapticRendererEmphasisFrequencySettings frequency_min;

  /// The settings to use when the emphasis frequency is at its maximum
  HapticRendererEmphasisFrequencySettings frequency_max;
} HapticRendererEmphasisOscillatorSettings;

/// The oscillator for rendering emphasis events
///
/// An emphasis event is represented as a short burst of an oscillator, with the oscillator's
/// frequency and output shape defined by the emphasis event's frequency.
typedef struct {
  /// The sample rate that the emphasis oscillator is running at
  float sample_rate;

  /// The type of output that should be provided by the oscillator
  HapticRendererMode render_mode;

  /// Used to generate the oscillator's phase during an emphasis event
  HapticRendererPhasor phasor;

  /// The number of samples remaining during an emphasis event
  int samples_remaining;

  /// The user-defined overall gain at the start of each emphasis event
  float gain_start;

  /// The user-defined overall gain at the end of each emphasis event
  float gain_end;

  /// Used to generate a gain ramp during an emphasis event
  ///
  /// The gain ramp will take into account both the user-defined start/end gains, and the event
  /// gain
  HapticRendererRampedValue gain;

  /// The user-defined minimum oscillator frequency, corresponding to a minimum emphasis frequency
  /// of 0
  float frequency_min;

  /// The oscillator frequency range, the maximum frequency of 1 corresponds to 'min + range'
  float frequency_range;

  /// The user-defined minimum event duration, corresponding to a minimum frequency of 0
  float duration_min;

  /// The size of the duration range, the maximum frequency of 1 corresponds to 'min + range'
  float duration_range;

  /// The oscillator shape to use to represent the minimum frequency of 0
  ///
  /// A crossfade is performed between the min/max shapes, based on the frequency value,
  /// e.g. a frequency of 0.5 will output an equal mix of the min and max shapes
  HapticRendererEmphasisShape shape_min;

  /// The oscillator shape to use to represent the maximum frequency of 1
  HapticRendererEmphasisShape shape_max;

  /// The crossfade gain to use with the minimum oscillator shape
  float shape_gain_min;

  /// The crossfade gain to use with the maximum oscillator shape
  float shape_gain_max;
} HapticRendererEmphasisOscillator;

void haptic_renderer_emphasis_oscillator_init(
    HapticRendererEmphasisOscillator* self,
    float sample_rate,
    HapticRendererMode render_mode,
    HapticRendererEmphasisOscillatorSettings* settings);

/// Resets the oscillator to its initial settings
void haptic_renderer_emphasis_oscillator_reset(HapticRendererEmphasisOscillator* self);

/// Starts a new emphasis event
///
/// The frequency is mapped to both the oscillator's frequency, and the duration of the event.
void haptic_renderer_emphasis_oscillator_start_emphasis(
    HapticRendererEmphasisOscillator* self,
    float amplitude,
    float frequency);

/// Produces the next sample of output
float haptic_renderer_emphasis_oscillator_process(HapticRendererEmphasisOscillator* self);

/// Returns true while an emphasis event is active
int haptic_renderer_emphasis_oscillator_is_active(HapticRendererEmphasisOscillator* self);

#ifdef __cplusplus
}
#endif
