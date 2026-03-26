// Copyright (c) Meta Platforms, Inc. and affiliates.

// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

#include "parametric_haptic_data/parametric_haptic_data.h"

#include <gtest/gtest.h>

/// Test data.
const char* const TEST_CLIP = R"(
{
  "version": {
    "major": 1,
    "minor": 0,
    "patch": 0
  },
  "metadata": {
    "editor": "VSCode",
    "author": "SDK Team",
    "source": "",
    "project": "",
    "tags": [
      "Test"
    ],
    "description": "Testing"
  },
  "signals": {
    "continuous": {
      "envelopes": {
        "amplitude": [
          {
            "time": 0.0,
            "amplitude": 0.2
          },
          {
            "time": 0.1234,
            "amplitude": 0.3
          },
          {
            "time": 0.2,
            "amplitude": 0.2,
            "emphasis": {
              "amplitude": 0.6,
              "frequency": 0.7
            }
          },
          {
            "time": 0.3,
            "amplitude": 0.5
          },
          {
            "time": 1.3,
            "amplitude": 0.5
          }
        ],
        "frequency": [
          {
            "time": 0.0,
            "frequency": 0.4
          },
          {
            "time": 0.2,
            "frequency": 0.25
          },
          {
            "time": 1.3,
            "frequency": 0.0
          }
        ]
      }
    }
  }
}
)";

const char* const MINIMAL_CLIP = R"(
{
  "version": {
    "major": 1,
    "minor": 0,
    "patch": 0
  },
  "signals": {
    "continuous": {
      "envelopes": {
        "amplitude": [
          {
            "time": 0.0,
            "amplitude": 1.0
          },
          {
            "time": 1.0,
            "amplitude": 0.0
          }
        ]
      }
    }
  }
}
)";

const char* const INVALID_CLIP_NO_AMPLITUDE_ENVELOPE = R"(
{
  "version": {
    "major": 1,
    "minor": 0,
    "patch": 0
  },
  "signals": {
    "continuous": {
      "envelopes": {
      }
    }
  }
}
)";

const char* const INVALID_CLIP_WRONG_TIME_TYPE = R"(
{
  "version": {
    "major": 1,
    "minor": 0,
    "patch": 0
  },
  "signals": {
    "continuous": {
      "envelopes": {
        "amplitude": [
          {
            "time": 0.0,
            "amplitude": 1.0
          },
          {
            "time": "one dot zero",
            "amplitude": 0.0
          }
        ]
      }
    }
  }
}
)";

const char* const INVALID_CLIP_MISSING_AMPLITUDE_POINT_MEMBER = R"(
{
  "version": {
    "major": 1,
    "minor": 0,
    "patch": 0
  },
  "signals": {
    "continuous": {
      "envelopes": {
        "amplitude": [
          {
            "time": 0.0,
            "amplitude": 1.0
          },
          {
            "amplitude": 0.0
          }
        ]
      }
    }
  }
}
)";

const char* const INVALID_CLIP_MISSING_EMPHASIS_MEMBER = R"(
{
  "version": {
    "major": 1,
    "minor": 0,
    "patch": 0
  },
  "signals": {
    "continuous": {
      "envelopes": {
        "amplitude": [
          {
            "time": 0.0,
            "amplitude": 1.0
          },
          {
            "time": 0.5,
            "amplitude": 0.2,
            "emphasis": {
              "amplitude": 0.6
            }
          },
          {
            "time": 1.0,
            "amplitude": 0.0
          }
        ]
      }
    }
  }
}
)";

const float TIME_EPSILON = 0.0001;
const float VALUE_EPSILON = 0.001;

TEST(ParametricHapticClip, ParseValid) {
  const auto clip = ParametricHapticClip::fromHapticClip(TEST_CLIP);
  ASSERT_TRUE(clip.has_value());

  ASSERT_EQ(clip->amplitudePoints.size(), 5);
  ASSERT_EQ(clip->frequencyPoints.size(), 3);
  ASSERT_EQ(clip->transients.size(), 1);

  ASSERT_NEAR(clip->amplitudePoints[1].timeNs / 1e9, 0.1234, TIME_EPSILON);
  ASSERT_NEAR(clip->amplitudePoints[1].value, 0.3, VALUE_EPSILON);

  ASSERT_NEAR(clip->frequencyPoints[1].timeNs / 1e9, 0.2, TIME_EPSILON);
  ASSERT_NEAR(clip->frequencyPoints[1].value, 0.25, VALUE_EPSILON);

  ASSERT_NEAR(clip->transients[0].timeNs / 1e9, 0.2, TIME_EPSILON);
  ASSERT_NEAR(clip->transients[0].amplitude, 0.6, VALUE_EPSILON);
  ASSERT_NEAR(clip->transients[0].frequency, 0.7, VALUE_EPSILON);
}

TEST(ParametricHapticClip, ParseMinimal) {
  ASSERT_TRUE(ParametricHapticClip::fromHapticClip(MINIMAL_CLIP).has_value());
}

TEST(ParametricHapticClip, ParseInvalid) {
  ASSERT_FALSE(
      ParametricHapticClip::fromHapticClip(INVALID_CLIP_NO_AMPLITUDE_ENVELOPE).has_value());
  ASSERT_FALSE(ParametricHapticClip::fromHapticClip(INVALID_CLIP_WRONG_TIME_TYPE).has_value());
  ASSERT_FALSE(
      ParametricHapticClip::fromHapticClip(INVALID_CLIP_MISSING_AMPLITUDE_POINT_MEMBER)
          .has_value());
  ASSERT_FALSE(
      ParametricHapticClip::fromHapticClip(INVALID_CLIP_MISSING_EMPHASIS_MEMBER).has_value());
}
