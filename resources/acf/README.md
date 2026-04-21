# Actuator Configuration File (ACF) Specification and Usage

# What is an ACF?

An Actuator Configuration File (ACF) is a device-specific configuration file that defines how haptic effects are rendered on your hardware. It allows the haptic rendering engine to translate abstract haptic data (e.g. envelopes & transients from .haptic files) into actuator-specific waveforms or amplitude curves that match your device’s capabilities.

The ACF is essential for:

* Device adaptation: Ensuring haptic effects are tuned for your actuator’s physical properties.
* Consistency: Providing a single technology to enable the best possible experience for each device.
* Customization: Allowing developers to fine-tune the tactile feel for their application or hardware.

The ACF is written in JSON5 format and contains two main sets of parameters:

* Continuous signals: For ongoing vibrations (e.g., engine rumble, background feedback).
* Emphasis (transient) signals: For short, distinct events (e.g., button clicks, impacts).


# File Structure

## Top-Level Sections

- metadata: Informational, for tracking and documentation.
- continuous: Parameters for continuous vibration.
- emphasis: Parameters for transient (emphasis) events.

### Example ACF

```json5
{
  metadata: {
    version: "1.0.0",
    device: "Your Device Name",
    date: "2024-06-01",
    author: "Your Name"
  },
  continuous: {
    gain: 0.8,                // Range: 0.0–1.0.
    emphasis_ducking: 0.5,    // Range: 0.0–1.0 (0 = no ducking, 1 = full ducking).
    frequency_min: 55.0,      // Hz, actuator's lowest effective frequency.
    frequency_max: 200.0,     // Hz, actuator's highest effective frequency.
  },
  emphasis: {
    gain: 1.0,                // Range: 0.0–1.0.
    fade_out_percent: 0.0,    // Range: 0.0–1.0 (0 = no fade, 1 = fade to silence).
    frequency_min: {
      output_frequency: 55.0, // Hz.
      duration_ms: 36.4,      // ms.
      shape: 'sine',          // 'saw', 'sine', 'square', 'triangle'.
    },
    frequency_max: {
      output_frequency: 165.0,
      duration_ms: 12.1,
      shape: 'square',
    }
  }
}
```

---

# Field Definitions

## 1\. Metadata

| Field | Type | Description |
| :---- | :---- | :---- |
| version | string | File version |
| device | string | The device name that this ACF targets (e.g. Quest) |
| date | string | Date of file creation (YYYY-MM-DD) |
| author | string | Author or point of contact |

## 2\. Continuous

| Field | Type | Description |
| :---- | :---- | :---- |
| gain | float | Signal gain for continuous vibration. |
| emphasis\_ducking | float | Additional gain reduction during transients. 1.0 \= no ducking, 0.0 \= full ducking |
| frequency\_min | float | Minimum frequency (Hz) for continuous vibration |
| frequency\_max | float | Maximum frequency (Hz) for continuous vibration |


### Setting Continuous Signal Values

* `gain`:
  * Signal gain for continuous vibration.
  * Set to 1 for full motor strength. Can be reduced to mitigate skin fatigue or emphasize transients.
* `emphasis_ducking`:
  * Reduces the continuous signal’s gain when a transient (emphasis) event occurs.
  * 0.0: No ducking; continuous signal stays at full strength during transients.
  * 1.0: Full ducking; continuous signal drops to zero during transients.
  * Typical value: 0.5 (continuous signal is halved during transients).
  * When to use:
    * Use lower values if you want transients to stand out sharply against the background.
    * Use higher values if you want transients to blend with the background or maximise continuous curve strength.

#### Determining `frequency_min`/`frequency_max`

Note: These values are ignored when RenderMode is `HAPTIC_RENDERER_MODE_AMP_CURVE`

##### Actuator Frequency Response

**Datasheet:**

Most actuator datasheets provide a frequency response curve or specify a frequency range where the actuator produces significant output (often measured in acceleration, G, or displacement).

**Key Range:**
The range where the actuator’s output is at least 1G (or another meaningful threshold) is a good starting point.

**Resonance:**
The resonance frequency (f₀) is where the actuator is most efficient, but the usable range is typically broader.

##### Practical Perceptual Bandwidth

Lower Bound (`frequency_min`):

* Set to the lowest frequency where the actuator produces a perceptible, non-distorted vibration.
* For wideband actuators it can be as low as 20–30 Hz.

Upper Bound (`frequency_max`):

* Set to the highest frequency where the actuator still produces a meaningful tactile sensation. Above this, output drops off or becomes more audible than tactile.
* This is 150–300 Hz; for wideband actuators, it can be up to 500 Hz or more, but practical haptics rarely go above 200–250 Hz.

##### Avoiding Audibility

* Frequencies above \~250–300 Hz can become audible, especially on plastic enclosures or in quiet environments.
* To avoid unwanted noise, it’s common to cap `frequency_max` at 200–250 Hz, even if the actuator can technically go higher.

##### Subjective Testing

* Tune by feel.
* After initial selection, test with real haptic content. If the low end feels “muddy” or the high end is “buzzy” or “inaudible,” adjust accordingly.

## 3\. Emphasis

| Field | Type | Description |
| :---- | :---- | :---- |
| gain | float | Signal gain for emphasis (transient) events. Typically 1 for most actuators. |
| fade\_out\_percent | float | Fade out percentage over event duration. 0 \= no fade, 1 \= fade to silence Typically 0 for most actuators. |
| frequency\_min | object | Settings for minimum sharpness (see below) |
| frequency\_max | object | Settings for maximum sharpness (see below) |

frequency\_min / frequency\_max (object)

| Field | Type | Description |
| :---- | :---- | :---- |
| output\_frequency | float | Output frequency (Hz) |
| duration\_ms | float | Duration (ms) |
| shape | string | Waveform shape: 'saw', 'sine', 'square', 'triangle' |

### Determining Emphasis Values

* `gain`:
  * Controls the strength of transient events (e.g., clicks, impacts).
  * Higher values: Stronger, more pronounced transients.
  * Lower values: Softer, less intrusive transients.
  * Usually set to 1.0 to get maximum strength impacts.
* `fade_out_percent`:
  * Determines how much the transient fades out over its duration.
  * 0: No fade; transient stays at full amplitude.
  * 1: Fades to silence by the end of the event.
  * Use 0 for most cases, but you can apply higher values for softer, less abrupt transients.
* `frequency_min / frequency_max`:
  * min corresponds to the low-end (0.0) of the transient sharpness, the max value to the high-end of the transient sharpness.

* For both:
  * Define the range of frequencies and durations for transients.
    * `output_frequency`:
      * These values should be set around the motor's (f₀) to keep the emphasis events strong.
      * Emphasis rendering will map sharpness to a point between the min and max values to make the transients and sharper or rounder.
      * It's important not to stray too far from (f₀) to maintain strong, impactful transients. For example, increasing `frequency_max` will make the transient feel sharper, but the signal strength will decrease.
      * Ultimately, these values should be tuned by feel
      * E.g. calculation with f0 at 65hz on a wide band VCM
        * min: 65 Hz × 0.8 \= 52 Hz
        * max 65 Hz × 1.8 \= 117 Hz
    * `duration_ms`:
      * Emphasis signals should be kept as brief as possible. Their duration can be measured in periods or cycles, but must not be shorter than the motor's rise time.
      * E.g with motor rise time of 20ms
        * `frequency_min`: Period at 52 Hz \= 19.2 ms \- bump to 1.5 cycles to account for rise time \= 29ms
        * `frequency_max`: Period at 117 Hz: 8.5ms ≈  bump to 20ms to keep the signal as short, sharp as possible.
    * `shape`:
      * 'sine': Smooth, rounded feel
      * 'square': Sharp, abrupt feel
      * 'triangle': Linear ramp up/down, in-between sine and square.
      * 'saw': Asymmetric, “buzzier” feel.

---

# Understanding the Trade-Off: Continuous vs. Emphasis Gain

What is "Gain"?

* Gain in the ACF controls the overall strength (amplitude) of the vibration signal sent to the actuator.
* There are two gain fields:
  * `continuous.gain`: The strength of ongoing vibrations.
  * `emphasis.gain`: The strength of short, transient events (like clicks or impacts).

Why is There a Trade-Off?

* Actuators have a maximum output—if you set both gains to their maximum, transients (emphasis events) may not stand out against the background, or you may hit hardware limits causing distortion or clipping.
* Perceptual contrast is key: Users should be able to feel the difference between a background rumble and a sharp click.

---

## The Role of Emphasis Ducking

* `emphasis_ducking` further controls the contrast:
  * Lower values (e.g., 0.3): Continuous curve drops when a transient occurs, making the transient "pop."
  * Higher values (e.g., 0.8): Continuous stays strong, transients blend in.

---

# Example ACFs

## Example Console (PCM)

```json5
{
  metadata: {
    version: "1",
    device: "VCM Generic f0 65Hz",
    date: "2025-08-29",
    author: "Neil Burdock"
  },
  continuous: {
    gain: 1.0,                 // Full use of the motor strength.
    emphasis_ducking: 0.5,     // Duck continuous haptics by 50% during an emphasis event to pop out the latter.
    frequency_min: 20.0,
    frequency_max: 250.0       // Wide use of motor for good texture, but without too much audible noise.
  },
  emphasis: {
    gain: 1.0,                 // Full use of the motor strength for emphasis events.
    fade_out_percent: 0.0,
    frequency_min: {
      output_frequency: 55.0,   // Just below the resonant frequency of the motor - round, strong.
      duration_ms: 27.0,        // 1.5 cycles at 55 Hz.
      shape: 'sine'             // Rounder waveform.
    },
    frequency_max: {
      output_frequency: 130.0,  // Very sharp, high frequency for 'clickiness'.
      duration_ms: 20.0,        // As short as possible = assumed rise time of the motor.
      shape: 'square'           // Sharper waveform.
    }
  }
}
```

## Console (Simple Haptics: Amplitude Curves Only)

```json5
{
  metadata: {
    version: "1.0.0",
    device: "Generic Console: Simple Haptics (amplitude only)",
    date: "2024-11-11",
    author: "Max"
  },
  continuous: {
    gain: 0.8,                // Reduces the amplitude of the continuous vibration to 80% to leave some head room for emphasis.
    emphasis_ducking: 0.8,    // Reduces the amplitude of the continuous vibration by 20% when an emphasis is played.
    frequency_min: 60,        // Usually the frequency range the continuous vibration plays back at, but set to the same value here
    frequency_max: 60,        // as we expect this ACF to be used a simple haptics backend, which only plays back amplitude values.
  },
  emphasis: {
    gain: 1.0,                // Emphasis are played back at 100% amplitude.
    fade_out_percent: 0,
    // This ACF targets a generic console that only plays back amplitude values.
    // The frequency values below are arbitrary and have no effect on the rendering.
    frequency_min: {
      output_frequency: 60,
      duration_ms: 40,
      shape: 'sine',
    },
    frequency_max: {
      output_frequency: 60,
      duration_ms: 20,
      shape: 'square',
    },
  },
}

```

# Integrating ACFs with the Renderer

1\. Loading the ACF

* Parse the ACF JSON5 file.
* Map the continuous and emphasis sections to the corresponding renderer settings.

2\. Initializing the Renderer

* For PCM-based systems (e.g., Meta Quest 3/3S):
  * Use `HAPTIC_RENDERER_MODE_SYNTHESIS` to output full waveform (frequency \+ amplitude).
  * The renderer will synthesize the waveform using the ACF’s frequency and shape fields.
* For amplitude curve systems (e.g., Android Simple Haptics):
  * Use `HAPTIC_RENDERER_MODE_AMP_CURVE` to output only the amplitude envelope.
  * The renderer **will ignore frequency and shape fields,** using only amplitude ramps and transients.
* Sample Rate:
  * Set the sample rate of the renderer's output to the appropriate rate for the APIs and systems you are calling.

3\. Rendering Loop

* Call `renderNextBatch(updateDurationNs)` to generate the next batch of haptic samples.
* Send the samples to your haptic driver or platform API.

---
