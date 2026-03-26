// Copyright (c) Meta Platforms, Inc. and affiliates.

// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

#pragma once

#ifdef __cplusplus
extern "C" {
#endif

/// A linear ramped value
///
/// The value will ramp linearly to the target value so that when ramping from [x -> y]
/// over N samples, the first value of the ramp will be x, and the Nth will be y.
///
/// HapticRendererRampedValue is precise with float precision up to durations of around 2 minutes.
/// For longer durations the ramp may finish slightly early.
typedef struct {
  /// The ramp's current value
  float current;

  /// The ramp's target value
  float target;

  /// The amount that the ramp should change with each processed output
  float ramp_increment;

  /// The number of samples remaining until the target is reached
  ///
  /// Q: Why is this a float rather than an integer?
  /// A: We don't want to distort the shape of a ramp through its duration, so the duration and
  ///    corresponding ramp increment match the parameters passed to
  ///    haptic_renderer_ramped_value_ramp_to_value(). Something needs to happen to make the ramp
  ///    fit quantized samples, so a non-integer duration will be truncated at the last sample of
  ///    output.
  /// Q: Why truncate the last step? Why not extend the ramp duration?
  /// A: We want to err on the side of 'the target value has been reached' before a new ramp
  ///    is started, so that the following ramp starts from the correct value.
  float samples_remaining_to_target;
} HapticRendererRampedValue;

void haptic_renderer_ramped_value_init_default(HapticRendererRampedValue* self);

/// Initializes a HapticRendererRampedValue with the provided value as its starting point
void haptic_renderer_ramped_value_init_with_value(HapticRendererRampedValue* self, float value);

/// Cancels any current ramp and jumps immediately to the provided value
void haptic_renderer_ramped_value_set_value(HapticRendererRampedValue* self, float value);

/// Starts a new ramp from the current value to the target value over the specified duration
void haptic_renderer_ramped_value_ramp_to_value(
    HapticRendererRampedValue* self,
    float target,
    float duration,
    float sample_rate);

/// Provides the next sample of output
float haptic_renderer_ramped_value_process(HapticRendererRampedValue* self);

/// Returns true if the target value has not yet been reached
int haptic_renderer_ramped_value_is_ramping(HapticRendererRampedValue* self);

/// Returns the target value
float haptic_renderer_ramped_value_target_value(HapticRendererRampedValue* self);

#ifdef __cplusplus
}
#endif
