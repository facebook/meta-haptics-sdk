// Copyright (c) Meta Platforms, Inc. and affiliates.

// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

#include <chrono>
#include <cstdint>
#include <vector>

#include "haptic_renderer/renderer.h"
#include "parametric_haptic_data/parametric_haptic_data.h"

struct RenderSettings {
  HapticRendererContinuousOscillatorSettings continuous;
  HapticRendererEmphasisOscillatorSettings emphasis;
};

/// Once a .haptic has been loaded into ParametricHapticClip, ParametricHapticRenderer can render it
/// either into PCM samples or an amplitude curve, i.e. whichever format is expected by the platform
/// haptics API. ParametricHapticRenderer performs partial data validation while rendering in
/// the renderNextBatch() function.
class ParametricHapticRenderer {
 public:
  /// Initializes a new ParametricHapticRenderer with the given settings.
  /// Returns false if validation fails.
  ///
  /// @param renderSettings Settings for continuous and emphasis oscillators. Load these from the
  /// ACF you are using for rendering.
  /// @param renderMode The rendering mode (e.g., synthesis or amplitude curve).
  /// @param sampleRate The sample rate in Hz for rendering.
  /// @param amplitudePoints Vector of amplitude envelope points.
  /// @param frequencyPoints Vector of frequency envelope points.
  /// @param transients Vector of transient haptic events.
  /// @return true if initialization succeeds; false otherwise.
  bool init(
      RenderSettings renderSettings,
      HapticRendererMode renderMode,
      float sampleRate,
      const std::vector<ParametricHapticPoint>& amplitudePoints,
      const std::vector<ParametricHapticPoint>& frequencyPoints,
      const std::vector<ParametricHapticTransient>& transients);

  /// Renders and returns the next batch of samples for a given updateDuration.
  ///
  /// @param updateDuration The duration in nanoseconds for this batch of samples
  /// @return Vector of rendered float samples (matching the render mode), or empty vector if
  /// validation fails.
  std::vector<float> renderNextBatch(std::chrono::nanoseconds updateDuration);

 private:
  std::vector<ParametricHapticPoint> amplitudePoints_; // Amplitude envelope points
  std::vector<ParametricHapticPoint> frequencyPoints_; // Frequency envelope points
  std::vector<ParametricHapticTransient> transients_; // Transient haptic events

  /// Index of the next amplitude point in the envelope
  size_t nextAmplitudeIndex_ = 0;
  /// Index of the next frequency point in the envelope
  size_t nextFrequencyIndex_ = 0;
  /// Index of the next transient event
  size_t nextTransientIndex_ = 0;

  uint64_t positionNs_ = 0;

  /// Underlying haptic renderer instance.
  mutable HapticRenderer renderer_;
};
