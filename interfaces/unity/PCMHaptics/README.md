# PCM Haptics for Unity

This library allows you to stream PCM haptics to supported OpenXR devices (e.g.
Meta Quest) in Unity.

The `PCMHapticsFeature` class hooks into the OpenXR runtime and enables PCM
haptics if available. The `PCMHaptics` class allows you to stream blocks of PCM
haptic samples into the runtime. You could use
[Parametric Haptic Renderer](../../../core/renderer_parametric/README.md#initialization)
or some generative algorithm to produce your PCM haptics in batches.

The implementation here is C# only as we leave it up to you to decide where the
managed/unmanaged code boundary is in your implementation. `PCMHaptics` could be
ported to native code if that is a good implementation for you (indeed the parts
that consume the OpenXR API are written in `unsafe` blocks as the OpenXR API is
native code). `PCMHapticsFeature` must remain in C# because that is how Unity
provides the OpenXR hooks.

For more details on PCM Haptics in OpenXR see
[the specification](https://registry.khronos.org/OpenXR/specs/1.1/html/xrspec.html#XR_FB_haptic_pcm).

# Example code for consuming Parametric Haptic Renderer

1. Create or open a Unity project for VR. E.g.:
   - A
     [3D project with URP](https://docs.unity3d.com/Packages/com.unity.render-pipelines.universal@7.1/manual/creating-a-new-project-with-urp.html).
   - [XR Plugin Management installed](https://docs.unity3d.com/6000.2/Documentation/Manual/xr-plugin-management.html).
   - The OpenXR Plugin provider enabled for Android (and Windows if applicable).
2. Enable "Allow Unsafe Code" in
   [Unity Player settings](https://docs.unity3d.com/Manual/class-PlayerSettings.html).
   This allows the `PCMHaptics` class to consume the OpenXR API. As mentioned
   above, alternatively you can port PCMHaptics to native code in which case you
   do not need to enable this setting.
3. Copy this directory into the Assets folder of your Unity project.
4. Once Unity has compiled the code, you need to enable the "Haptics PCM"
   feature under Edit > Project Settings > XR Plug-in Management > OpenXR within
   the
   [OpenXR Feature Groups section](https://docs.unity3d.com/Packages/com.unity.xr.openxr@1.15/manual/features.html)
   for your target platforms.

In this example we will see two sample rates: that of the controllers and that
of the batches of PCM haptics. For optimal haptics playback we will match our
sampling rate to the controller sample rate. This minimizes unnecessary
processing (resampling).

## Initialization

Here we get the `PCMHapticsFeature` and initialize `PCMHaptics` through it. It
is necessary to perform initialization this way because `PCMHapticsFeature` has
the OpenXR hooks that `PCMHaptics` needs. It is also necessary to initialize and
confirm a positive controller sample rate in a polling fashion (in a loop or
callback) because the controllers must be active before we send any haptics to
them. This is only the case once the user has picked the controllers up (in the
case of Meta Quest).

The code that calls into `PCMHapticsFeature` here must remain in C#. Other code
could be ported to native code as required.

```C#
using System;
using System.Collections;
using System.Runtime.InteropServices;
using UnityEngine;
using UnityEngine.XR.OpenXR;

public class GameHaptics : MonoBehaviour
{
    private PCMHaptics pcmHaptics;

    // The rate at which your batches of PCM haptics get sampled.
    private float sampleRate = 0.0f;

    // The controller hand we want to play haptics on.
    private ControllerHand controllerHand = PCMHaptics.ControllerHand.Left;

    void Awake()
    {
        StartCoroutine(InitializePCMHaptics());
    }

    IEnumerator InitializePCMHaptics()
    {
        // Keep trying to initialize PCMHaptics.
        if (pcmHaptics == null)
        {
            pcmHaptics ??= OpenXRSettings.Instance.GetFeature<PCMHapticsFeature>()?.InitializePCMHaptics();
        }

        // Check if PCMHaptics initialized successfully this time.
        if (pcmHaptics == null)
        {
            yield return new WaitForSeconds(0.5f);
            StartCoroutine(InitializePCMHaptics());
        }

        StartCoroutine(GetSampleRate());
    }

    IEnumerator GetSampleRate()
    {
        // Query PCMHaptics for the sample rate of the selected controller.
        if (pcmHaptics != null && sampleRate == 0.0f)
        {
            sampleRate = pcmHaptics.GetControllerSampleRateHz(controllerHand);
            yield return new WaitForSeconds(0.5f);
            StartCoroutine(GetSampleRate());
        }

        InitializeHapticsRenderer()

        yield return null;
    }

    private void InitializeHapticsRenderer()
    {
        // Once a positive sample rate is returned, we can start producing PCM samples.
        // Use the controller sample rate as our sampling rate here. This is the optimal
        // sample rate to use but the API will also resample to the sample rate of the controllers
        // if you have to use a different one.

        // Initialize Parametric Haptic Renderer (../../../core/renderer_parametric/README.md#initialization)
        // or whichever code you are using to produce PCM haptics.

        // Then enable your code that produces batches of PCM haptics, i.e. that which calls
        // GenerateBatch() below.
    }
}
```

## Process loop

Run this code from an update loop or callback at regular intervals, preferably
at a fixed clock rate (i.e. `bufferSize`/ `sampleRate` seconds in this example).
This code can be ported to native code if needed.

```C#
    /// <summary>
    /// Generates the next batch of PCM haptics.
    /// </summary>
    ///
    /// <param name="shouldAppend">Set this to false when starting a new stream (i.e. haptic effect); true when continuing one.</param>
    /// <param name="bufferSize">The number of samples that should be produced and streamed.</param>
    void GenerateBatch(bool shouldAppend, uint bufferSize)
    {
        // pcmHaptics and sampleRate are data members.
        if (pcmHaptics == null || sampleRate == 0)
        {
            // Log a warning or error:
            // "Cannot generate batch of PCM haptics; entry requirements are not met"
            // or similar.
            return;
        }

        // Produce the next batch of PCM haptics, e.g. by using [Parametric Haptic Renderer](../../../core/renderer_parametric/README.md#process-loop)
        // or a generative algorithm.

        // A pointer to the first sample in your batch.
        private IntPtr unmanagedPcmBufferPtr = ...;

        // Stream and note how many samples were consumed.
        uint samplesConsumed = pcmHaptics.Stream(unmanagedPcmBufferPtr, bufferSize, sampleRate, shouldAppend, controllerHand);

        // If some samples were not consumed you have to decide whether to resend or not. Read the comment on
        // PCMHaptics.Stream() to understand how to manage buffer sizes and make sure you don't encounter underrun or overrun.
        var samplesToResend = bufferSize - samplesConsumed;
    }
```
