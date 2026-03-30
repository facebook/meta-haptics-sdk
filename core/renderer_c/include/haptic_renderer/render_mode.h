// Copyright (c) Meta Platforms, Inc. and affiliates.

// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

#pragma once

#ifdef __cplusplus
extern "C" {
#endif

/// Settings that define the output of the haptic renderer
typedef enum {
  /// Synthesis
  ///
  /// The output will be synthesized PCM audio data in the range -1.0..=1.0 that can directly
  /// drive an actuator.
  HAPTIC_RENDERER_MODE_SYNTHESIS,

  /// Amplitude Curve
  ///
  /// The output will be a curve in the range 0..=1.0 that represents the intensity of an actuator
  /// over time.
  HAPTIC_RENDERER_MODE_AMP_CURVE,
} HapticRendererMode;

#ifdef __cplusplus
}
#endif
