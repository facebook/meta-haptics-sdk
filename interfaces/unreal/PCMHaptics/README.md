# PCM Haptics for Unreal

This Unreal plugin allows you to stream PCM haptics to supported OpenXR devices (e.g. Meta Quest) in Unreal.

The `PCMHaptics` class is both an Unreal module and an OpenXR extension plugin for PCM haptics. This means it receives calls to Unreal's OpenXR hooks and enables PCM haptics if available. It also allows you to stream blocks of PCM haptic samples into the runtime. You could use [Parametric Haptic Renderer](../../../core/renderer_parametric/README.md#initialization) or some generative algorithm to produce your PCM haptics in batches.

For more details on PCM Haptics in OpenXR see [the specification](https://registry.khronos.org/OpenXR/specs/1.1/html/xrspec.html#XR_FB_haptic_pcm).

# Example code for consuming Parametric Haptic Renderer

1. Create or open an Unreal project for VR, e.g. with the [VR Template](https://dev.epicgames.com/documentation/en-us/unreal-engine/vr-template-in-unreal-engine). See the [Unreal docs](https://dev.epicgames.com/documentation/en-us/unreal-engine/developing-for-xr-experiences-in-unreal-engine?application_version=5.6) for more on VR development.
2. Copy this directory into the Plugins folder of your Unreal project.

In this example we will see two sample rates: that of the controllers and that of the batches of PCM haptics. For optimal haptics playback we will match our sampling rate to the controller sample rate. This minimizes unnecessary processing (resampling).

## Initialization

We need to initialize `PCMHaptics` via Unreal's [FModuleManager](https://dev.epicgames.com/documentation/en-us/unreal-engine/API/Runtime/Core/FModuleManager). In this example we do so in an [Actor](https://dev.epicgames.com/documentation/en-us/unreal-engine/actors-in-unreal-engine). We also need to confirm a positive sample rate in a polling fashion (in a loop or callback) because the controllers must be active before we send any haptics to them. This is only the case once the user has picked the controllers up (in the case of Meta Quest).

GameHaptics.h
```C++
#pragma once

#include "CoreMinimal.h"
#include "PCMHaptics.h"
#include "GameFramework/Actor.h"
#include "GameHaptics.generated.h"

UCLASS()
class MY_GAME_API AGameHaptics : public AActor
{
    GENERATED_BODY()

public:
    AGameHaptics();
    virtual void Tick(float DeltaTime) override;

protected:
    virtual void BeginPlay() override;

private:
    EControllerHand ControllerHand = EControllerHand::Left;
    FPCMHapticsModule* PCMHapticsPlugin;
    float SampleRate = 0.0f;
};

```

GameHaptics.cpp
```C++
#include "GameHaptics.h"

AGameHaptics::AGameHaptics()
{
    PrimaryActorTick.bCanEverTick = true;

    FModuleManager& ModuleManager = FModuleManager::Get();
    if (ModuleManager.IsModuleLoaded("PCMHaptics"))
    {
        PCMHaptics = ModuleManager.LoadModulePtr<FPCMHapticsModule>("PCMHaptics");
    }
}

void AGameHaptics::BeginPlay()
{
    Super::BeginPlay();
}

void AGameHaptics::Tick(float DeltaTime)
{
    Super::Tick(DeltaTime);

    if (PCMHapticsPlugin && PCMHapticsPlugin->IsPCMHapticsAvailableOnSystem())
    {
        if (SampleRate == 0.0f)
        {
            // Query PCMHaptics for the sample rate of the selected controller.
            SampleRate = PCMHapticsPlugin->GetControllerSampleRateHz(ControllerHand);
        }

        if (SampleRate > 0.0f)
        {
            // Once a positive sample rate is returned, we can start producing PCM samples.
            // Use the controller sample rate as our sampling rate here. This is the optimal sample rate to use but
            // the API will also resample to the sample rate of the controllers.

            // [Initialize Parametric Haptic Renderer](../../../core/renderer_parametric/README.md#initialization) if using
            // it to produce PCM haptics.

            // Then enable your code that produces batches of PCM haptics.
        }
    }
}
```

## Process loop
Run this code from an update loop or callback at regular intervals, preferably at a fixed clock rate (i.e. `InBufferSize`/ `SampleRate` seconds in this example).

```C++
void GenerateBatch(bool InShouldAppend, uint32_t InBufferSize)
{
    if (!PCMHaptics || !PCMHaptics->IsPCMHapticsAvailableOnSystem() || SampleRate == 0.0f)
    {
        // Log a warning or error:
        // "Cannot generate batch of PCM haptics; entry requirements are not met"
        return;
    }

    // Produce the next batch of PCM haptics, e.g. by using [Parametric Haptic Renderer](../../../core/renderer_parametric/README.md#process-loop) or a generative algorithm.
    // A pointer to the first sample in your batch.
    std::vector<float> PCMBuffer = ...;

    // Stream and note how many samples were consumed.
    uint32_t SamplesConsumed = PCMHaptics->Stream(PCMBuffer.data(), InBufferSize, SampleRate, InShouldAppend, ControllerHand);

    // If some samples were not consumed you have to decide whether to resend or not. Read the comment on
    // FPCMHapticsModule::Stream() to understand how to manage buffer sizes and make sure you don't encounter underrun or overrun.
    uint32_t SamplesToResend = InBufferSize - SamplesConsumed;
}
```
