# Copyright (c) Meta Platforms, Inc. and affiliates.

# This source code is licensed under the MIT license found in the
# LICENSE file in the root directory of this source tree.

if (NOT TARGET GTest::gtest_main)
    add_subdirectory("${PROJECT_SOURCE_DIR}/../../third-party/googletest" "extern/googletest")
endif()

# Set C++ standard for test executables
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

enable_testing()

# Create test executable for parametric_haptic_data
add_executable(
    parametric_haptic_data_test
    parametric_haptic_data_test.cpp
)

target_link_libraries(
    parametric_haptic_data_test
    parametric_haptic_data
    GTest::gtest_main
)

include(GoogleTest)
set(gtest_force_shared_crt ON CACHE BOOL "" FORCE)
gtest_discover_tests(parametric_haptic_data_test)
