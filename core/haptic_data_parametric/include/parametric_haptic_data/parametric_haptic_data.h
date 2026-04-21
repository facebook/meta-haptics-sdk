// Copyright (c) Meta Platforms, Inc. and affiliates.

// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

#pragma once

#include <cstdint>
#include <optional>
#include <string_view>
#include <vector>

/// ParametricHapticClip is a slightly different representation of the data contained within a
/// .haptic file. It is structured this way for ParametricHapticRender. It is supported by
/// ParametricHapticPoint and ParametricHapticTransient. ParametricHapticClip does not currently
/// validate the .haptic data.
struct ParametricHapticPoint {
  int64_t timeNs;
  float value;
};

struct ParametricHapticTransient {
  int64_t timeNs;
  float amplitude;
  float frequency;
};

class ParametricHapticClip {
 public:
  // Creates a parametric haptic clip from the JSON describing a .haptic clip.
  static std::optional<ParametricHapticClip> fromHapticClip(const std::string_view& jsonString);

  std::vector<ParametricHapticPoint> amplitudePoints;
  std::vector<ParametricHapticPoint> frequencyPoints;
  std::vector<ParametricHapticTransient> transients;
};
