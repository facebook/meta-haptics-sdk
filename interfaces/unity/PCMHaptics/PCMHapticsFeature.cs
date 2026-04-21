// Copyright (c) Meta Platforms, Inc. and affiliates.

// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

using System;
using UnityEngine;
using UnityEngine.XR.OpenXR;
using UnityEngine.XR.OpenXR.Features;
using UnityEngine.InputSystem;

#if UNITY_EDITOR
using UnityEditor;
using UnityEditor.XR.OpenXR.Features;
#endif

using XrPath = System.UInt64;

/// <summary>
/// An OpenXRFeature for PCM Haptics. This OpenXRFeature requests the OpenXR PCM Haptics extension (XR_FB_haptic_pcm)
/// to be enabled. This feature defines the hooks that receive OpenXR handles which it then uses to initialize the PCMHaptics class.
///
/// When adding this to your codebase, add your company name, documentation
/// link, version and feature ID to the attributes list below.
/// </summary>
#if UNITY_EDITOR
[OpenXRFeature(UiName = "Haptics PCM",
    BuildTargetGroups = new[] { BuildTargetGroup.Android, BuildTargetGroup.Standalone },
    Company = "(insert company name)",
    Desc = "OpenXR PCM Haptics extension",
    DocumentationLink = "(insert docs link)",
    OpenxrExtensionStrings = "XR_FB_haptic_pcm",
    Version = "1.0.0",
    FeatureId = "com.company.feature.haptics")]
#endif
public class PCMHapticsFeature : OpenXRFeature
{
    /// <summary>
    /// OpenXR constants.
    /// </summary>
    const int XR_NULL_HANDLE = 0;
    const int XR_NULL_PATH = 0;

    /// <summary>
    /// OpenXR handles.
    /// </summary>
    ulong _instance = XR_NULL_HANDLE;
    ulong _session = XR_NULL_HANDLE;

    PCMHaptics _pcmHaptics = new PCMHaptics();

    /// <summary>
    /// Initializes the PCMHaptics using the handles that have been received via the
    /// various OpenXR hooks.
    /// </summary>
    ///
    /// <returns>PCMHaptics instance if the initialization was successful; null otherwise.</returns>
    public PCMHaptics InitializePCMHaptics()
    {
        // Get the OpenXR haptics action (it doesn't matter which hand is used)
        // by first getting an action from UnityEngine.InputSystem and then getting the
        // OpenXR action from that.
        var hapticAction = new InputAction(
            name: "Haptic",
            type: InputActionType.PassThrough,
            binding: "<XRController>{LeftHand}/haptic"
        );
        hapticAction.Enable();
        var action = this.GetAction(hapticAction);

        if (action != XR_NULL_HANDLE && _pcmHaptics.Initialize(_instance, _session, action, StringToPath("/user/hand/left"), StringToPath("/user/hand/right")))
        {
            return _pcmHaptics;
        }

        return null;
    }

    /// <summary>
    /// Overrides the HookGetInstanceProcAddr() OpenXR hook passing the received function pointer
    /// to the PCMHaptics instance. This function pointer allows PCMHaptics to access any OpenXR function.
    /// </summary>
    protected override IntPtr HookGetInstanceProcAddr(IntPtr func)
    {
        _pcmHaptics.SetInstanceProcAddr(func);

        return base.HookGetInstanceProcAddr(func);
    }

    /// <summary>
    /// Overrides the OnInstanceCreate() OpenXR hook and caches the received XrInstance handle which
    /// will be used later in execution by InitializePCMHaptics().
    /// </summary>
    ///
    /// <returns>true if the PCM haptics extension is loaded; false otherwise which causes the OpenXRLoader to abort and try another loader.</returns>
    protected override bool OnInstanceCreate(ulong xrInstance)
    {
        if (!OpenXRRuntime.IsExtensionEnabled("XR_FB_haptic_pcm"))
        {
            Debug.LogWarning("PCMHapticsFeature.OnInstanceCreate: XR_FB_haptic_pcm is not enabled, disabling PCM Haptics.");
            return false;
        }

        _instance = xrInstance;

        return true;
    }

    /// <summary>
    /// Overrides the OnSessionCreate() OpenXR hook and caches the received XrSession handle which
    /// will be used later in execution by InitializePCMHaptics().
    /// </summary>
    protected override void OnSessionCreate(ulong xrSession)
    {
        _session = xrSession;

        base.OnSessionCreate(xrSession);
    }
}
