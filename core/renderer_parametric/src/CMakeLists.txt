# Copyright (c) Meta Platforms, Inc. and affiliates.

# This source code is licensed under the MIT license found in the
# LICENSE file in the root directory of this source tree.

cmake_minimum_required(VERSION 3.10)

# Set C++ standard
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

# Add the renderer_c and haptic_data_parametric directories as subdirectories
add_subdirectory(../../renderer_c renderer_c_build)
add_subdirectory(../../haptic_data_parametric haptic_data_parametric_build)

# Create parametric_haptic_renderer library
add_library(parametric_haptic_renderer
    parametric_haptic_renderer.cpp
)

target_include_directories(parametric_haptic_renderer PUBLIC
                                      "${PROJECT_BINARY_DIR}"
                                      "${PROJECT_SOURCE_DIR}/include"
                                      )

target_link_libraries(parametric_haptic_renderer PUBLIC
    parametric_haptic_data
)

target_link_libraries(parametric_haptic_renderer PUBLIC renderer_c)
