# `interfaces/studio`

This folder contains the Haptics SDK interface used by Haptics Studio.

## Directory Structure
- `audio_analysis`
  - Contains support for generating haptics from audio data.
- `napi`
  - Contains a Node-API interface for the Haptics SDK, along with additional
    functionality from `audio_analysis`.

## Historical Note

In the old days of Lofelt Studio, all of the heavy-lifting algorithmic work was
bundled together into a single Wasm package for ease of importing and
deployment. We've carried over the same bundled structure for the time being,
so this interface includes support for audio data decoding and waveform
generation that wouldn't typically belong in an SDK for haptics.

## `haptics_sdk_napi`

The `haptics_sdk_napi` library contains a Node-API interface to the Haptics SDK,
along with additional functionality from [`audio_analysis`](./audio_analysis/).
