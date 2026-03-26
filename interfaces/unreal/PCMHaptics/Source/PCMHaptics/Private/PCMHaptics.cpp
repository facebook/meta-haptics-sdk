// Copyright (c) Meta Platforms, Inc. and affiliates.

// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

#include "PCMHaptics.h"

#include "IOpenXRHMDModule.h"
#include "OpenXRCore.h"

#define LOCTEXT_NAMESPACE "FPCMHapticsModule"

/// <summary>
/// Registers this OpenXR extension modular feature.
/// </summary>
void FPCMHapticsModule::StartupModule() {
  RegisterOpenXRExtensionModularFeature();
}

bool FPCMHapticsModule::IsPCMHapticsAvailableOnSystem() const {
  return bExtFBHapticsPcmAvailable;
}

/// <summary>
/// Unreal's OpenXR hook that allows us to state that we want the PCM extension to be
/// enabled.
/// </summary>
bool FPCMHapticsModule::GetOptionalExtensions(TArray<const ANSICHAR*>& OutExtensions) {
  OutExtensions.Add(XR_FB_HAPTIC_PCM_EXTENSION_NAME);

  return true;
}

/// <summary>
/// Unreal's OpenXR hook that gets called when the OpenXR instance is being created. At this
/// point we can check if it was possible for the PCM extension to be enabled.
/// </summary>
const void* FPCMHapticsModule::OnCreateInstance(
    class IOpenXRHMDModule* InModule,
    const void* InNext) {
  bExtFBHapticsPcmAvailable = InModule->IsExtensionEnabled(XR_FB_HAPTIC_PCM_EXTENSION_NAME);

  if (!bExtFBHapticsPcmAvailable) {
    UE_LOG(
        LogPCMHaptics,
        Warning,
        TEXT("FPCMHapticsModule::OnCreateInstance: It wasn't possible to enable PCM Haptics."));
  }

  return InNext;
}

/// <summary>
/// Unreal's OpenXR hook that gets called when the OpenXR instance has been created. This is
/// when we initialize the OpenXR functions we need.
/// </summary>
void FPCMHapticsModule::PostCreateInstance(XrInstance InInstance) {
  Instance = InInstance;

  InitOpenXRFunctions();
}

void FPCMHapticsModule::InitOpenXRFunctions() {
  if (bExtFBHapticsPcmAvailable) {
    if (XR_FAILED(xrGetInstanceProcAddr(
            Instance,
            "xrGetDeviceSampleRateFB",
            (PFN_xrVoidFunction*)(&xrGetDeviceSampleRateFB)))) {
      UE_LOG(
          LogPCMHaptics,
          Fatal,
          TEXT(
              "FPCMHapticsModule::InitOpenXRFunctions: Failed to bind OpenXR entry xrGetDeviceSampleRateFB."));
    }
  }
}

float FPCMHapticsModule::GetControllerSampleRateHz(EControllerHand Hand) const {
  if (xrGetDeviceSampleRateFB == nullptr || !Instance || !Session || !bExtFBHapticsPcmAvailable) {
    UE_LOG(
        LogPCMHaptics,
        Warning,
        TEXT(
            "FPCMHapticsModule::GetControllerSampleRateHz: Cannot get controller sample rate: Entry conditions not met."));
    return 0.0f;
  }

  const XrPath SubactionPath = ControllerHandToPath(Hand);

  if (SubactionPath == XR_NULL_PATH) {
    UE_LOG(
        LogPCMHaptics,
        Error,
        TEXT(
            "FPCMHapticsModule::GetControllerSampleRateHz: Cannot get controller sample rate: OpenXR path for the given controller hand is invalid."));
    return 0.f;
  }

  XrHapticActionInfo HapticActionInfo = {XR_TYPE_HAPTIC_ACTION_INFO};
  HapticActionInfo.action = Action;
  HapticActionInfo.subactionPath = SubactionPath;
  HapticActionInfo.next = nullptr;

  XrDevicePcmSampleRateGetInfoFB DeviceSampleRate = {XR_TYPE_DEVICE_PCM_SAMPLE_RATE_GET_INFO_FB};

  const XrResult Result = xrGetDeviceSampleRateFB(Session, &HapticActionInfo, &DeviceSampleRate);

  if (XR_FAILED(Result)) {
    UE_LOG(
        LogPCMHaptics,
        Error,
        TEXT("FPCMHapticsModule::xrGetDeviceSampleRateFB failed: %d."),
        Result);
    return 0.f;
  }

  return DeviceSampleRate.sampleRate;
}

uint32_t FPCMHapticsModule::Stream(
    const float* InPCMBuffer,
    uint32_t InBufferSize,
    float InSampleRate,
    bool InShouldAppend,
    EControllerHand Hand) {
  if (!bExtFBHapticsPcmAvailable || InSampleRate <= 0.0f) {
    UE_LOG(
        LogPCMHaptics,
        Error,
        TEXT("FPCMHapticsModule::Stream: Cannot stream. Entry conditions not met."));
    return 0;
  }

  const XrPath SubactionPath = ControllerHandToPath(Hand);

  if (SubactionPath == XR_NULL_PATH) {
    UE_LOG(
        LogPCMHaptics,
        Error,
        TEXT(
            "FPCMHapticsModule::Stream: Cannot stream: OpenXR path for the given controller hand is invalid."));
    return 0.f;
  }

  uint32_t SamplesConsumed = 0;

  XrHapticPcmVibrationFB HapticPcmVibration = {XR_TYPE_HAPTIC_PCM_VIBRATION_FB};
  HapticPcmVibration.buffer = InPCMBuffer;
  HapticPcmVibration.bufferSize = InBufferSize;
  HapticPcmVibration.sampleRate = InSampleRate;
  HapticPcmVibration.samplesConsumed = &SamplesConsumed;
  HapticPcmVibration.append = InShouldAppend;
  HapticPcmVibration.next = nullptr;

  XrHapticActionInfo HapticActionInfo{XR_TYPE_HAPTIC_ACTION_INFO};
  HapticActionInfo.action = Action;
  HapticActionInfo.subactionPath = SubactionPath;
  HapticActionInfo.next = nullptr;

  XrResult Result = xrApplyHapticFeedback(
      Session, &HapticActionInfo, reinterpret_cast<const XrHapticBaseHeader*>(&HapticPcmVibration));

  if (XR_FAILED(Result)) {
    UE_LOG(LogPCMHaptics, Error, TEXT("FPCMHapticsModule::Stream: Failed: %d."), Result);
  }

  return SamplesConsumed;
}

XrPath FPCMHapticsModule::ControllerHandToPath(EControllerHand Hand) const {
  int HandIndex = -1;
  switch (Hand) {
    case EControllerHand::Left:
      HandIndex = 0;
      break;
    case EControllerHand::Right:
      HandIndex = 1;
      break;
    default:
      UE_LOG(
          LogPCMHaptics,
          Error,
          TEXT("FPCMHapticsModule::ControllerHandToPath: Cannot get path, none defined for %s."),
          *UEnum::GetValueAsString(Hand));
      return XR_NULL_PATH;
  }

  return XrPathBothHands[HandIndex];
}

void FPCMHapticsModule::CreateActionSet() {
  XrActionSetCreateInfo ActionSetCreateInfo = {
      .type = XR_TYPE_ACTION_SET_CREATE_INFO,
      .next = nullptr,
      .actionSetName = "pcm-haptics-action-set",
      .localizedActionSetName = "PCMHapticsActionSet",
  };

  XR_ENSURE(xrCreateActionSet(Instance, &ActionSetCreateInfo, &ActionSet));

  // Create hand haptics paths
  XR_ENSURE(xrStringToPath(Instance, "/user/hand/left", &XrPathLeftHand));
  XR_ENSURE(xrStringToPath(Instance, "/user/hand/left/output/haptic", &XrPathLeftHandHaptics));
  XR_ENSURE(xrStringToPath(Instance, "/user/hand/right", &XrPathRightHand));
  XR_ENSURE(xrStringToPath(Instance, "/user/hand/right/output/haptic", &XrPathRightHandHaptics));
  XrPathBothHands[0] = XrPathLeftHand;
  XrPathBothHands[1] = XrPathRightHand;
  XrPathBothHandsHaptics[0] = XrPathLeftHandHaptics;
  XrPathBothHandsHaptics[1] = XrPathRightHandHaptics;

  XrActionCreateInfo ActionCreateInfo = {
      .type = XR_TYPE_ACTION_CREATE_INFO,
      .next = nullptr,
      .actionName = "pcm-haptics-action",
      .actionType = XR_ACTION_TYPE_VIBRATION_OUTPUT,
      .countSubactionPaths = sizeof(XrPathBothHands) / sizeof(XrPath),
      .subactionPaths = XrPathBothHands,
      .localizedActionName = "PCMHapticsAction",
  };

  XR_ENSURE(xrCreateAction(ActionSet, &ActionCreateInfo, &Action));
}

void FPCMHapticsModule::PostCreateSession(XrSession InSession) {
  UE_LOG(LogPCMHaptics, Display, TEXT("FPCMHapticsModule::PostCreateSession"));
  Session = InSession;
}

bool FPCMHapticsModule::GetSuggestedBindings(
    XrPath InInteractionProfile,
    TArray<XrActionSuggestedBinding>& OutBindings) {
  UE_LOG(LogPCMHaptics, Display, TEXT("FPCMHapticsModule::GetSuggestedBindings"));

  if (Action == XR_NULL_HANDLE) {
    return false;
  }

  OutBindings.Add({Action, XrPathLeftHandHaptics});
  OutBindings.Add({Action, XrPathRightHandHaptics});

  return true;
}

void FPCMHapticsModule::AttachActionSets(TSet<XrActionSet>& OutActionSets) {
  UE_LOG(LogPCMHaptics, Display, TEXT("FPCMHapticsModule::AttachActionSets"));

  if (ActionSet != XR_NULL_HANDLE) {
    OutActionSets.Add(ActionSet);
  }
}

void FPCMHapticsModule::GetActiveActionSetsForSync(TArray<XrActiveActionSet>& OutActiveSets) {
  if (ActionSet != XR_NULL_HANDLE) {
    OutActiveSets.Add({ActionSet, XR_NULL_PATH});
  }
}

/// <summary>
/// Unreal's OpenXR hook that gets called at the start of Unreal's input action creation. This is
/// when we create our action set.
/// </summary>
bool FPCMHapticsModule::GetInteractionProfile(
    XrInstance InInstance,
    FString& OutKeyPrefix,
    XrPath& OutPath,
    bool& OutHasHaptics) {
  if (ActionSet != XR_NULL_HANDLE) {
    DestroyActionSet();
  }

  CreateActionSet();

  return true;
}

void FPCMHapticsModule::DestroyActionSet() {
  if (ActionSet != XR_NULL_HANDLE) {
    xrDestroyActionSet(ActionSet);
    Action = XR_NULL_HANDLE;
    ActionSet = XR_NULL_HANDLE;
  }
}

#undef LOCTEXT_NAMESPACE

IMPLEMENT_MODULE(FPCMHapticsModule, PCMHaptics)
