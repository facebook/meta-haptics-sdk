# Copyright (c) Meta Platforms, Inc. and affiliates.

# This source code is licensed under the MIT license found in the
# LICENSE file in the root directory of this source tree.

cmake_minimum_required(VERSION 3.10)

# Set C++ standard
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

# Create parametric_haptic_data library
add_library(parametric_haptic_data
    parametric_haptic_data.cpp
)

# Add jsoncpp directory
add_subdirectory(../../../third-party/jsoncpp third-party/jsoncpp_build)

# Set C++ standard for jsoncpp library and tests
set_target_properties(jsoncpp_lib PROPERTIES
    CXX_STANDARD 17
    CXX_STANDARD_REQUIRED ON
    CXX_EXTENSIONS OFF)
set_target_properties(jsontestrunner_exe PROPERTIES
    CXX_STANDARD 17
    CXX_STANDARD_REQUIRED ON
    CXX_EXTENSIONS OFF)
set_target_properties(jsoncpp_test PROPERTIES
    CXX_STANDARD 17
    CXX_STANDARD_REQUIRED ON
    CXX_EXTENSIONS OFF)

target_link_libraries(parametric_haptic_data PRIVATE jsoncpp_lib)

target_include_directories(parametric_haptic_data PUBLIC
    ${CMAKE_CURRENT_SOURCE_DIR}
    ${CMAKE_CURRENT_SOURCE_DIR}/../include
    ${CMAKE_CURRENT_SOURCE_DIR}/../../../third-party/openxr/include
    ${CMAKE_CURRENT_SOURCE_DIR}/../../../third-party/jsoncpp/include
)
