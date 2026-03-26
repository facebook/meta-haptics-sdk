// Copyright (c) Meta Platforms, Inc. and affiliates.

// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

#pragma once

#include "IOpenXRExtensionPlugin.h"
#include "Modules/ModuleManager.h"

DEFINE_LOG_CATEGORY_STATIC(LogPCMHaptics, Log, All);

/// <summary>
/// A module and OpenXR extension plugin for PCM haptics. As the latter it has
/// Unreal's OpenXR hooks which it uses to activate the PCM haptics OpenXR
/// extension. It also exposes functionality to get the sample rate of attached
/// controllers and stream PCM buffers to them.
/// </summary>
class FPCMHapticsModule : public IModuleInterface, public IOpenXRExtensionPlugin {
 public:
  /// <summary>
  ///	Checks if the OpenXR PCM extension is available on the system.
  /// </summary>
  PCMHAPTICS_API bool IsPCMHapticsAvailableOnSystem() const;

  /// <summary>
  /// Gets the controller PCM sample rate, i.e. the sample rate at which the controller expects to
  /// receive PCM samples from the OpenXR runtime.
  /// </summary>
  ///
  /// <param name="Hand">The controller hand for which the sample rate is being queried.</param>
  ///
  /// <returns>The sample rate of the specified controller.</returns>
  PCMHAPTICS_API float GetControllerSampleRateHz(EControllerHand Hand) const;

  /// <summary>
  /// Streams a buffer of PCM haptic samples to the OpenXR haptics API (xrApplyHapticFeedback()).
  /// We recommend against streaming haptics continuously over the duration of an application;
  /// rather start a new stream for each haptic effect/event as it occurs.
  /// </summary>
  ///
  /// <remarks>
  /// This method should be called from some kind of callback or lifecycle function preferably at a
  /// fixed rate roughly every 30 to 40 milliseconds. The fundamental idea is that you send your
  /// initial PCM buffer with shouldAppend set to false and each subsequent PCM buffer with
  /// shouldAppend set to true until the haptic effect is complete. You do not need to send anything
  /// to indicate effect completion.
  ///
  /// However, you need to be mindful of overrunning or underrunning a stream. Overrunning occurs
  /// when you send more PCM data than can be handled by the OpenXR runtime at a given moment;
  /// underrunning is when you send too little. Overrunning produces latency and potentially lost
  /// data; underrunning produces dropouts.
  ///
  /// To provide a stream that doesn't produce either of these scenarios do the following:
  /// Prime the streaming pipeline in your initial buffer and fit each subsequent buffer to the time
  /// between calls. Let's say you are calling this function at a fixed rate of 40ms. Start the
  /// stream with a larger buffer of your haptic effect - an extra 50ms should suffice - so, 90ms in
  /// this case, and shouldAppend set to false. The second and each subsequent call should provide
  /// 40ms of PCM haptics at this fixed rate until the effect is played out (the final call can
  /// contain less).
  ///
  /// Regarding the signal sample rate, there is an advantage to rendering your PCM haptics at the
  /// sample rate expected by the controller (see GetControllerSampleRateHz(ControllerHand hand)).
  /// If you do this you save some runtime cost by mitigating resampling in the OpenXR runtime and
  /// optimising for internal buffers.
  /// </remarks>
  ///
  /// <param name="InPCMBuffer">A pointer to the initial sample in the buffer.</param>
  /// <param name="InBufferSize">The number of samples in the buffer.</param>
  /// <param name="InSampleRate">The signal sample rate, i.e., the rate at which the samples were
  /// created.</param> <param name="InShouldAppend">Set this to false when starting a stream; true
  /// when continuing one.</param> <param name="Hand">The controller hand to stream to.</param>
  ///
  /// <returns>Number of samples that were consumed.</returns>
  PCMHAPTICS_API uint32_t Stream(
      const float* InPCMBuffer,
      uint32_t InBufferSize,
      float InSampleRate,
      bool InShouldAppend,
      EControllerHand Hand);

  // IModuleInterface implementation.
  virtual void StartupModule() override;

  // IOpenXRExtensionPlugin implementation.
  virtual bool GetOptionalExtensions(TArray<const ANSICHAR*>& OutExtensions) override;
  virtual const void* OnCreateInstance(class IOpenXRHMDModule* InModule, const void* InNext)
      override;
  virtual void PostCreateInstance(XrInstance InInstance) override;
  virtual void PostCreateSession(XrSession InSession) override;
  virtual bool GetSuggestedBindings(
      XrPath InInteractionProfile,
      TArray<XrActionSuggestedBinding>& OutBindings) override;
  virtual void AttachActionSets(TSet<XrActionSet>& OutActionSets) override;
  virtual void GetActiveActionSetsForSync(TArray<XrActiveActionSet>& OutActiveSets) override;
  virtual bool GetInteractionProfile(
      XrInstance InInstance,
      FString& OutKeyPrefix,
      XrPath& OutPath,
      bool& OutHasHaptics) override;

 private:
  /// <summary>
  /// Creates the OpenXR action set required for PCM haptics.
  /// </summary>
  void CreateActionSet();

  /// <summary>
  /// Fetches the OpenXR function pointers needed for PCM haptics.
  /// </summary>
  void InitOpenXRFunctions();

  /// <summary>
  /// Destroys the OpenXR action set used for PCM haptics.
  /// </summary>
  void DestroyActionSet();

  /// <summary>
  /// Converts EControllerHand to an OpenXR path.
  /// </summary>
  XrPath ControllerHandToPath(EControllerHand hand) const;

  bool bExtFBHapticsPcmAvailable = false;

  // Pointer to the OpenXR function to get the controller PCM haptics sample rate.
  PFN_xrGetDeviceSampleRateFB xrGetDeviceSampleRateFB = nullptr;

  // OpenXR handles.
  XrInstance Instance = XR_NULL_HANDLE;
  XrSession Session = XR_NULL_HANDLE;
  XrActionSet ActionSet = XR_NULL_HANDLE;
  XrAction Action = XR_NULL_HANDLE;

  // The OpenXR paths relevant to PCM haptics.
  XrPath XrPathLeftHand = XR_NULL_PATH;
  XrPath XrPathLeftHandHaptics = XR_NULL_PATH;
  XrPath XrPathRightHand = XR_NULL_PATH;
  XrPath XrPathRightHandHaptics = XR_NULL_PATH;
  XrPath XrPathBothHands[2] = {XR_NULL_PATH, XR_NULL_PATH};
  XrPath XrPathBothHandsHaptics[2] = {XR_NULL_PATH, XR_NULL_PATH};
};
