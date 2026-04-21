# Parametric Haptic Renderer

Parametric Haptic Renderer takes
[Parametric Haptic Data](../haptic_data_parametric/README.md) and renders it to
the given rendering mode and settings. It performs batch-based rendering where
each batch corresponds to a time slice of the haptic data. Your application code
should drive it from an update loop or callback preferably at a constant clock
rate.

# Example code for consuming Parametric Haptic Renderer

The following example demonstrates how to consume and drive
ParametricHapticRenderer. We have intentionally left the JSON loading blank for
you to fill with calls to your library of choice. Regarding the render settings,
these come from an ACF (Actuator Configuration File). To learn more read the
[ACF README](../../resources/acf/README.md).

See also the unit tests at `core/renderer_parametric/tests`.

## Initialization

This example renders PCM haptics (`HAPTIC_RENDERER_MODE_SYNTHESIS`) at 2kHz.

```

// Convert the `.haptic` to a ParametricHapticClip using the Parametric Haptic Data library.
const char* const hapticData = ... // Load JSON from .haptic
const auto parametricHapticClip = ParametricHapticClip::fromHapticClip(hapticData);

// Initialize ParametricHapticRenderer with the render settings from the ACF and the ParametricHapticClip.
ParametricHapticRenderer parametricHapticRenderer;
RenderSettings renderSettings = ... // Load JSON from ACF
bool rendererSuccessfullyInitialized = parametricHapticRenderer.init(
      renderSettings,
      HAPTIC_RENDERER_MODE_SYNTHESIS,
      2000,
      parametricHapticClip->amplitudePoints,
      parametricHapticClip->frequencyPoints,
      parametricHapticClip->transients);
```

## Process loop

Run this code from an update loop or callback at regular intervals, preferably
at a fixed clock rate (`updateDurationNs`).

```
parametricHapticRenderer.renderNextBatch(updateDurationNs);
```
