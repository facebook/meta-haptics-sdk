// Copyright (c) Meta Platforms, Inc. and affiliates.

// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

using System;
using System.Runtime.InteropServices;
using UnityEngine;

// Aliases for XrPath and XrAction.
using XrPath = System.UInt64;
using XrAction = System.UInt64;

// The methods below test for XrResult so it is defined here.
public enum XrResult : int
{
    XR_SUCCESS = 0,
}

/// <summary>
/// The definitions of the types of OpenXR structs that are relevant to PCM haptics.
/// </summary>
public enum XrStructureType : int
{
    XR_TYPE_UNKNOWN = 0,
    XR_TYPE_HAPTIC_ACTION_INFO = 59,
    XR_TYPE_HAPTIC_PCM_VIBRATION_FB = 1000209001,
    XR_TYPE_DEVICE_PCM_SAMPLE_RATE_STATE_FB = 1000209002,
}

/// <summary>
/// The definition of the type of XR action relevant to haptics, i.e. a haptic
/// vibration.
/// </summary>
public enum XrActionType
{
    XR_ACTION_TYPE_VIBRATION_OUTPUT = 100
}

/// <summary>
/// The struct passed to a number of OpenXR functions.
/// </summary>
public struct XrHapticActionInfo
{
    public XrStructureType type;
    public IntPtr next;
    public XrAction action;
    public XrPath subactionPath;
}

/// <summary>
/// The struct passed to xrGetDeviceSampleRateFB().
/// </summary>
public struct XrDevicePcmSampleRateGetInfoFB
{
    public XrStructureType type;
    public IntPtr next;
    public float sampleRate;
}

/// <summary>
/// The struct passed to XrApplyHapticFeedback() for PCM haptics.
/// </summary>
public struct XrHapticPcmVibrationFB
{
    public XrStructureType type;
    public IntPtr next;
    public uint bufferSize;
    public IntPtr buffer;
    public float sampleRate;
    public uint append;
    public IntPtr samplesConsumed;
}

/// <summary>
/// Streams blocks of PCM samples to the OpenXR PCM Haptics extension.
/// This class should not be instantiated directly, but rather through the InitializePCMHaptics()
/// method of the OpenXRFeature class defined in PCMHapticsFeature.cs which provides the required
/// OpenXR handles to it.
///
/// Once instantiated through the OpenXRFeature class, the other methods can be used directly
/// to get the sample rate of the attached controller and to stream PCM samples to it.
/// </summary>
public class PCMHaptics
{
    /// <summary>
    /// The controller hand that the haptics should play on.
    /// </summary>
    public enum ControllerHand : uint
    {
        Left = 0,
        Right = 1
    }

    // OpenXR constants.
    const int XR_NULL_HANDLE = 0;
    const int XR_NULL_PATH = 0;

    // OpenXR handles.
    ulong _instance = XR_NULL_HANDLE;
    ulong _session = XR_NULL_HANDLE;
    XrAction _action = XR_NULL_HANDLE;
    public XrPath[] _pathBothHands = { XR_NULL_PATH, XR_NULL_PATH };


    // Delegates to OpenXR functions. These allow PCMHaptics to call into the native OpenXR runtime.
    delegate XrResult xrGetInstanceProcAddr(
        ulong instance,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string name, // OpenXR uses UTF-8 strings (C# doesn't), so tell the marshaler to convert the string
        out IntPtr function);
    xrGetInstanceProcAddr _xrGetInstanceProcAddr;

    delegate XrResult xrGetDeviceSampleRateFB(
        ulong session,
        in XrHapticActionInfo hapticActionInfo,
        ref XrDevicePcmSampleRateGetInfoFB deviceSampleRate);
    xrGetDeviceSampleRateFB _xrGetDeviceSampleRateFB;

    delegate XrResult xrApplyHapticFeedback(
        ulong session,
        in XrHapticActionInfo hapticActionInfo,
        ref XrHapticPcmVibrationFB hapticFeedback);
    xrApplyHapticFeedback _xrApplyHapticFeedback;

    /// <summary>
    /// This should only be called by an OpenXRFeature class. The OpenXRFeature uses this to
    /// pass in a pointer to the OpenXR runtime's xrGetInstanceProcAddr() function. This is
    /// used, in turn, to get pointers to other OpenXR functions.
    /// </summary>
    public void SetInstanceProcAddr(IntPtr func)
    {
        // Turn the native function pointer into a delegate we can invoke from managed code.
        _xrGetInstanceProcAddr = Marshal.GetDelegateForFunctionPointer<xrGetInstanceProcAddr>(func);
    }

    /// <summary>
    /// This should only be called by an OpenXRFeature class. It initialises an instance of this class
    /// with the set of OpenXR handles required for PCM streaming. Subsequent calls after the initial
    /// one will not repeat initialization and will produce a return value of true.
    /// </summary>
    ///
    /// <param name="instance">A handle to the OpenXR instance.</param>
    /// <param name="session">A handle to the OpenXR session.</param>
    /// <param name="action">A handle to the XrAction for haptics.</param>
    /// <param name="leftHand">A handle to the OpenXR path to the left hand for haptic output.</param>
    /// <param name="rightHand">A handle to the OpenXR path to the right hand for haptic output.</param>
    ///
    /// <returns><c>true</c> if initialization was successful or previously carried out; <c>false</c> otherwise.</returns>
    public bool Initialize(ulong instance, ulong session, XrAction action, XrPath leftHand, XrPath rightHand)
    {
        if (_xrGetInstanceProcAddr == null || instance == XR_NULL_HANDLE || action == XR_NULL_HANDLE || leftHand == XR_NULL_HANDLE || rightHand == XR_NULL_HANDLE)
        {
            Debug.LogWarning("PCMHaptics.Initialize: Cannot initialize, one or more parameters is invalid: _xrGetInstanceProcAddr: " + _xrGetInstanceProcAddr + ", instance: " + instance + ", action: " + action + ", leftHand: " + leftHand + ", rightHand: " + rightHand);
            return false;
        }

        if (_instance != XR_NULL_HANDLE && _session != XR_NULL_HANDLE && _action != XR_NULL_HANDLE && _pathBothHands[0] != XR_NULL_HANDLE && _pathBothHands[1] != XR_NULL_HANDLE)
        {
            Debug.Log("PCMHaptics.Initialize: Already initialized.");
            return true;
        }

        _instance = instance;
        _session = session;
        _action = action;
        _pathBothHands[0] = leftHand;
        _pathBothHands[1] = rightHand;

        // Set OpenXR PCM haptics function pointers.
        if (_xrGetInstanceProcAddr(_instance, nameof(xrGetDeviceSampleRateFB), out var functionPointerGetDeviceSampleRateFB) == XrResult.XR_SUCCESS)
        {
            _xrGetDeviceSampleRateFB = Marshal.GetDelegateForFunctionPointer<xrGetDeviceSampleRateFB>(functionPointerGetDeviceSampleRateFB);
        }
        else
        {
            Debug.LogWarning($"PCMHaptics.Initialize: Unable to get function pointer for {nameof(xrGetDeviceSampleRateFB)}");
        }

        if (_xrGetInstanceProcAddr(_instance, nameof(xrApplyHapticFeedback), out var functionPointerApplyHapticFeedback) == XrResult.XR_SUCCESS)
        {
            _xrApplyHapticFeedback = Marshal.GetDelegateForFunctionPointer<xrApplyHapticFeedback>(functionPointerApplyHapticFeedback);
        }
        else
        {
            Debug.LogWarning(
                $"PCMHaptics.Initialize: Unable to get function pointer for {nameof(xrApplyHapticFeedback)}");
        }

        return true;
    }

    /// <summary>
    /// Gets the controller PCM sample rate, i.e. the sample rate at which the controller expects to receive PCM samples from
    /// the OpenXR runtime.
    /// </summary>
    ///
    /// <param name="hand">The controller hand for which the sample rate is being queried.</param>
    ///
    /// <returns>The sample rate of the specified controller.</returns>
    public float GetControllerSampleRateHz(ControllerHand hand)
    {
        if (_xrGetDeviceSampleRateFB == null || _instance == XR_NULL_HANDLE || _session == XR_NULL_HANDLE || _action == XR_NULL_HANDLE)
        {
            Debug.LogWarning("PCMHaptics.GetControllerSampleRateHz: Cannot get controller sample rate. Entry conditions not met.");
            return 0.0f;
        }

        unsafe
        {
            XrHapticActionInfo hapticActionInfo = default;
            hapticActionInfo.type = XrStructureType.XR_TYPE_HAPTIC_ACTION_INFO;
            hapticActionInfo.action = _action;
            hapticActionInfo.subactionPath = _pathBothHands[(uint)hand];
            hapticActionInfo.next = IntPtr.Zero;

            XrDevicePcmSampleRateGetInfoFB deviceSampleRate = default;
            deviceSampleRate.type = XrStructureType.XR_TYPE_DEVICE_PCM_SAMPLE_RATE_STATE_FB;
            deviceSampleRate.next = IntPtr.Zero;
            deviceSampleRate.sampleRate = 0.0f;

            XrResult result = _xrGetDeviceSampleRateFB(_session, hapticActionInfo, ref deviceSampleRate);

            if (result != XrResult.XR_SUCCESS)
            {
                Debug.LogError("PCMHaptics.GetControllerSampleRateHz failed: " + result);
                return 0.0f;
            }

            return deviceSampleRate.sampleRate;
        }
    }

    /// <summary>
    /// Streams a buffer of PCM haptic samples to the OpenXR haptics API (xrApplyHapticFeedback()).
    /// We recommend against streaming haptics continuously over the duration of an application; rather start
    /// a new stream for each haptic effect/event as it occurs.
    /// </summary>
    ///
    /// <remarks>
    /// This method should be called from some kind of callback or lifecycle function preferably at a fixed rate
    /// roughly every 30 to 40 milliseconds.
    /// The fundamental idea is that you send your initial PCM buffer with shouldAppend set to false and each subsequent
    /// PCM buffer with shouldAppend set to true until the haptic effect is complete. You do not need to send anything
    /// to indicate effect completion.
    ///
    /// However, you need to be mindful of overrunning or underrunning a stream. Overrunning occurs when you send
    /// more PCM data than can be handled by the OpenXR runtime at a given moment; underrunning is when you send
    /// too little. Overrunning produces latency and potentially lost data; underrunning produces dropouts.
    ///
    /// To provide a stream that doesn't produce either of these scenarios do the following:
    /// Prime the streaming pipeline in your initial buffer and fit each subsequent buffer to the time between calls.
    /// Let's say you are calling this function at a fixed rate of 40ms. Start the stream with a larger buffer
    /// of your haptic effect - an extra 50ms should suffice - so, 90ms in this case, and shouldAppend set to false.
    /// The second and each subsequent call should provide 40ms of PCM haptics at this fixed rate until the effect
    /// is played out (the final call can contain less).
    ///
    /// Regarding the signal sample rate, there is an advantage to rendering your PCM haptics at the sample rate expected
    /// by the controller (see GetControllerSampleRateHz(ControllerHand hand)). If you do this you save some runtime
    /// cost by mitigating resampling in the OpenXR runtime and optimising for internal buffers.
    /// </remarks>
    ///
    /// <param name="pcmBuffer">A pointer to the initial sample in the buffer.</param>
    /// <param name="bufferSize">The number of samples in the buffer.</param>
    /// <param name="sampleRate">The signal sample rate, i.e., the rate at which the samples were created.</param>
    /// <param name="shouldAppend">Set this to false when starting a stream; true when continuing one.</param>
    /// <param name="hand">The controller hand to stream to.</param>
    ///
    /// <returns>Number of samples that were consumed.</returns>
    public uint Stream(IntPtr pcmBuffer, uint bufferSize, float sampleRate, bool shouldAppend, ControllerHand hand)
    {
        if (_xrApplyHapticFeedback == null || _instance == XR_NULL_HANDLE || _session == XR_NULL_HANDLE || _action == XR_NULL_HANDLE)
        {
            Debug.LogWarning("PCMHaptics.Stream: Cannot stream. Entry conditions not met.");
            return 0;
        }

        if (sampleRate <= 0.0f)
        {
            Debug.LogWarning("PCMHaptics.Stream: Cannot stream. Sample rate provided is too low.");
            return 0;
        }

        uint samplesConsumed = 0;

        XrHapticPcmVibrationFB hapticPcmVibration = default;

        unsafe
        {
            uint* pSamplesConsumed = &samplesConsumed;

            hapticPcmVibration.type = XrStructureType.XR_TYPE_HAPTIC_PCM_VIBRATION_FB;

            hapticPcmVibration.buffer = pcmBuffer;
            hapticPcmVibration.bufferSize = bufferSize;
            hapticPcmVibration.sampleRate = sampleRate;
            hapticPcmVibration.samplesConsumed = (IntPtr)pSamplesConsumed;
            hapticPcmVibration.append = shouldAppend ? 1u : 0u;
            hapticPcmVibration.next = IntPtr.Zero;
        }

        XrHapticActionInfo hapticActionInfo = default;
        hapticActionInfo.type = XrStructureType.XR_TYPE_HAPTIC_ACTION_INFO;
        hapticActionInfo.action = _action;
        hapticActionInfo.subactionPath = _pathBothHands[(uint)hand];
        hapticActionInfo.next = IntPtr.Zero;

        XrResult result = _xrApplyHapticFeedback(_session, hapticActionInfo, ref hapticPcmVibration);

        if (result != XrResult.XR_SUCCESS)
        {
            Debug.LogError("PCMHaptics.Stream failed: " + result);
            return 0;
        }

        return samplesConsumed;
    }
}
