// Copyright (c) Meta Platforms, Inc. and affiliates.

// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

#include "parametric_haptic_renderer/parametric_haptic_renderer.h"

#include <cmath>
#include <iostream>

bool ParametricHapticRenderer::init(
    RenderSettings renderSettings,
    HapticRendererMode renderMode,
    float sampleRate,
    const std::vector<ParametricHapticPoint>& amplitudePoints,
    const std::vector<ParametricHapticPoint>& frequencyPoints,
    const std::vector<ParametricHapticTransient>& transients) {
  if (amplitudePoints.size() < 2) {
    return false;
  }
  if (amplitudePoints.at(0).timeNs != 0 ||
      (!frequencyPoints.empty() && frequencyPoints.at(0).timeNs != 0)) {
    return false;
  }

  amplitudePoints_ = amplitudePoints;
  frequencyPoints_ = frequencyPoints;
  transients_ = transients;
  positionNs_ = 0;
  nextFrequencyIndex_ = 0;
  nextAmplitudeIndex_ = 0;
  nextTransientIndex_ = 0;

  haptic_renderer_init(
      &renderer_, sampleRate, renderMode, &renderSettings.continuous, &renderSettings.emphasis);

  // At the start, pass ramps with duration 0 to the HapticRenderer, to immediately cause a jump
  // of the amplitude and frequency to the first value.
  if (!amplitudePoints_.empty()) {
    haptic_renderer_start_amplitude_ramp(&renderer_, amplitudePoints_.front().value, 0.0);
  }
  if (!frequencyPoints_.empty()) {
    haptic_renderer_start_frequency_ramp(&renderer_, frequencyPoints_.front().value, 0.0);
  }

  return true;
}

static bool validatePoint(const ParametricHapticPoint& point) {
  if (point.value < 0.0 || point.value > 1.0) {
    return false;
  }

  return true;
}

static bool validateNextPoint(
    const ParametricHapticPoint& previous,
    const ParametricHapticPoint& next) {
  if (next.timeNs < previous.timeNs) {
    return false;
  }

  return validatePoint(next);
}

static bool validateTransient(const ParametricHapticTransient& transient) {
  if (transient.amplitude < 0.0 || transient.amplitude > 1.0) {
    return false;
  }
  if (transient.frequency < 0.0 || transient.frequency > 1.0) {
    return false;
  }

  return true;
}

std::vector<float> ParametricHapticRenderer::renderNextBatch(
    std::chrono::nanoseconds updateDurationNs) {
  const float sampleRate = haptic_renderer_sample_rate(&renderer_);
  const size_t numSamples = ceil(sampleRate * (updateDurationNs.count() / 1e9f));

  std::vector<float> samples;
  samples.reserve(numSamples);

  for (size_t sampleIndex = 0; sampleIndex < numSamples; sampleIndex++) {
    // Start a new amplitude ramp once the next amplitude point is reached
    if (nextAmplitudeIndex_ < amplitudePoints_.size() &&
        positionNs_ >= amplitudePoints_[nextAmplitudeIndex_].timeNs) {
      const auto previousAmplitudePoint = amplitudePoints_[nextAmplitudeIndex_];
      nextAmplitudeIndex_++;
      if (nextAmplitudeIndex_ < amplitudePoints_.size()) {
        const auto nextAmplitudePoint = amplitudePoints_[nextAmplitudeIndex_];
        if (!validateNextPoint(previousAmplitudePoint, nextAmplitudePoint)) {
          // Amplitude point invalid, return empty vector.
          return {};
        }
        const float amplitude = nextAmplitudePoint.value;
        const float durationNs = nextAmplitudePoint.timeNs - positionNs_;
        haptic_renderer_start_amplitude_ramp(&renderer_, amplitude, durationNs / 1e9f);
      } else {
        // Stream has ended (stream ends when the amplitude envelope ends).
        break;
      }
    }

    // Start a new frequency ramp once the next frequency point is reached
    if (nextFrequencyIndex_ < frequencyPoints_.size() &&
        positionNs_ >= frequencyPoints_[nextFrequencyIndex_].timeNs) {
      const auto previousFrequencyPoint = frequencyPoints_[nextFrequencyIndex_];
      nextFrequencyIndex_++;
      if (nextFrequencyIndex_ < frequencyPoints_.size()) {
        const auto nextFrequencyPoint = frequencyPoints_[nextFrequencyIndex_];
        if (!validateNextPoint(previousFrequencyPoint, nextFrequencyPoint)) {
          // Frequency point invalid, return empty vector.
          return {};
        }
        const float frequency = nextFrequencyPoint.value;
        const float durationNs = nextFrequencyPoint.timeNs - positionNs_;
        haptic_renderer_start_frequency_ramp(&renderer_, frequency, durationNs / 1e9f);
      } else {
        // Frequency envelope has ended.
      }
    }

    // Start a new transient once the timeNs position is reached
    if (nextTransientIndex_ < transients_.size() &&
        positionNs_ >= transients_[nextTransientIndex_].timeNs) {
      const auto nextTransient = transients_[nextTransientIndex_];
      if (!validateTransient(nextTransient)) {
        return {};
      }
      const float amplitude = nextTransient.amplitude;
      const float frequency = nextTransient.frequency;

      haptic_renderer_start_emphasis(&renderer_, amplitude, frequency);

      nextTransientIndex_++;
      if (nextTransientIndex_ < transients_.size() &&
          transients_[nextTransientIndex_].timeNs <= nextTransient.timeNs) {
        // Transient invalid, return empty vector.
        return {};
      }
    }

    const float sample = haptic_renderer_process(&renderer_);
    samples.push_back(sample);
    positionNs_ += updateDurationNs.count() / numSamples;
  }

  return samples;
}
