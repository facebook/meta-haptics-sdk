# Copyright (c) Meta Platforms, Inc. and affiliates.

# This source code is licensed under the MIT license found in the
# LICENSE file in the root directory of this source tree.

add_subdirectory("${PROJECT_SOURCE_DIR}/../../third-party/googletest" "extern/googletest")

enable_testing()

add_executable(
  test_continuous_oscillator
  test_continuous_oscillator.cpp
)

target_link_libraries(
  test_continuous_oscillator
  renderer_c
  GTest::gtest_main
)

add_executable(
  test_emphasis_oscillator
  test_emphasis_oscillator.cpp
)

target_link_libraries(
  test_emphasis_oscillator
  renderer_c
  GTest::gtest_main
)

add_executable(
  test_phasor
  test_phasor.cpp
)

target_link_libraries(
  test_phasor
  renderer_c
  GTest::gtest_main
)

add_executable(
  test_ramped_value
  test_ramped_value.cpp
)

target_link_libraries(
  test_ramped_value
  renderer_c
  GTest::gtest_main
)

include(GoogleTest)
set(gtest_force_shared_crt ON CACHE BOOL "" FORCE)
