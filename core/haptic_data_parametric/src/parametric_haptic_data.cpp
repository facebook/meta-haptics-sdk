// Copyright (c) Meta Platforms, Inc. and affiliates.

// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

#include "parametric_haptic_data/parametric_haptic_data.h"

#include <json/json.h>

std::optional<ParametricHapticClip> ParametricHapticClip::fromHapticClip(
    const std::string_view& jsonString) {
  Json::Reader reader;
  Json::Value root;
  if (!reader.parse(jsonString.data(), jsonString.data() + jsonString.size(), root)) {
    return std::nullopt;
  }

  if (!root.isMember("signals") || !root["signals"].isMember("continuous") ||
      !root["signals"]["continuous"].isMember("envelopes")) {
    return std::nullopt;
  }
  const Json::Value& envelopes = root["signals"]["continuous"]["envelopes"];
  if (!envelopes.isMember("amplitude")) {
    return std::nullopt;
  }
  const Json::Value& amplitudeEnvelope = envelopes["amplitude"];
  if (!amplitudeEnvelope.isArray()) {
    return std::nullopt;
  }

  ParametricHapticClip result;

  result.amplitudePoints.reserve(amplitudeEnvelope.size());
  for (const auto& amplitudePoint : amplitudeEnvelope) {
    if (!amplitudePoint.isMember("time") || !amplitudePoint.isMember("amplitude")) {
      return std::nullopt;
    }
    const Json::Value& time = amplitudePoint["time"];
    const Json::Value& amplitude = amplitudePoint["amplitude"];
    if (!time.isNumeric() || !amplitude.isNumeric()) {
      return std::nullopt;
    }

    ParametricHapticPoint point;
    point.timeNs = amplitudePoint["time"].asFloat() * 1e9;
    point.value = amplitudePoint["amplitude"].asFloat();
    result.amplitudePoints.push_back(point);

    if (amplitudePoint.isMember("emphasis")) {
      const Json::Value& emphasis = amplitudePoint["emphasis"];
      if (!emphasis.isMember("amplitude") || !emphasis.isMember("frequency")) {
        return std::nullopt;
      }
      const Json::Value& emphasisAmplitude = emphasis["amplitude"];
      const Json::Value& emphasisFrequency = emphasis["frequency"];
      if (!emphasisAmplitude.isNumeric() || !emphasisFrequency.isNumeric()) {
        return std::nullopt;
      }

      ParametricHapticTransient transient;
      transient.timeNs = point.timeNs;
      transient.amplitude = emphasisAmplitude.asFloat();
      transient.frequency = emphasisFrequency.asFloat();
      result.transients.push_back(transient);
    }
  }

  if (envelopes.isMember("frequency")) {
    const Json::Value& frequencyEnvelope = envelopes["frequency"];
    if (!frequencyEnvelope.isArray()) {
      return std::nullopt;
    }
    result.frequencyPoints.reserve(frequencyEnvelope.size());
    for (const auto& frequencyPoint : frequencyEnvelope) {
      if (!frequencyPoint.isMember("time") || !frequencyPoint.isMember("frequency")) {
        return std::nullopt;
      }
      const Json::Value& time = frequencyPoint["time"];
      const Json::Value& frequency = frequencyPoint["frequency"];
      if (!time.isNumeric() || !frequency.isNumeric()) {
        return std::nullopt;
      }

      ParametricHapticPoint point;
      point.timeNs = time.asFloat() * 1e9;
      point.value = frequency.asFloat();
      result.frequencyPoints.push_back(point);
    }
  }

  return result;
}
