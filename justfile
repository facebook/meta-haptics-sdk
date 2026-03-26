default: test-all

build-all: build-renderer-c build-haptic-data-parametric build-renderer-parametric

build-renderer-c:
  just core/renderer_c/build

build-haptic-data-parametric:
  just core/haptic_data_parametric/build

build-renderer-parametric:
  just core/renderer_parametric/build

clean:
  just core/renderer_c/clean
  just core/haptic_data_parametric/clean
  just core/renderer_parametric/clean

test-all:
  just core/renderer_c/test
  just core/haptic_data_parametric/test
  just core/renderer_parametric/test
