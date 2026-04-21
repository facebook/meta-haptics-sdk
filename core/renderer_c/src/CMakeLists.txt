# Copyright (c) Meta Platforms, Inc. and affiliates.

# This source code is licensed under the MIT license found in the
# LICENSE file in the root directory of this source tree.

add_library(renderer_c renderer.c continuous_oscillator.c emphasis_oscillator.c phasor.c ramped_value.c renderer.c)
target_include_directories(renderer_c PUBLIC
                                      "${PROJECT_BINARY_DIR}"
                                      "${PROJECT_SOURCE_DIR}/include"
                                      )
