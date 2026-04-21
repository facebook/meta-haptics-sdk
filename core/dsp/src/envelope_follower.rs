// (c) Meta Platforms, Inc. and affiliates. Confidential and proprietary.

use crate::WindowedMovingRms;
use crate::flush_f32_to_zero;

/// An envelope follower with fixed length stages and exponential ramping
///
/// The ramping is exponential, with the stages reaching 99% of their target by the end of the
/// specified stage duration.
///
/// # Example
/// ```
/// use haptic_dsp::EnvelopeFollower;
///
/// // Set up an envelope follower with stages that are 2 samples in length
/// let attack_time = 1.0;
/// let hold_time = 1.0;
/// let release_time = 1.0;
/// let sample_rate = 2.0;
/// let mut follower = EnvelopeFollower::new(attack_time, hold_time, release_time, sample_rate);
///
/// // Attack stage, ramping up towards 1.0
/// assert_eq!(follower.process(1.0), 0.9);
/// assert_eq!(follower.process(1.0), 0.99);
///
/// // Hold stage, maintaining the level while the input level is lower
/// assert_eq!(follower.process(0.0), 0.99);
/// assert_eq!(follower.process(0.0), 0.99);
///
/// // Release stage, dropping down towards 0.0
/// assert_eq!(follower.process(0.0), 0.099);
/// assert_eq!(follower.process(0.0), 0.0099);
/// ```
///
/// # See also
/// [`RmsEnvelopeFollower`](struct.RmsEnvelopeFollower.html)
pub struct EnvelopeFollower {
    envelope_level: f32,
    attack_coefficent: f32,
    release_coefficient: f32,
    hold_counter: u32,
    hold_time_in_samples: u32,
}

impl EnvelopeFollower {
    /// Initializes an envelope follower with specified stage duration times
    pub fn new(attack_time: f32, hold_time: f32, release_time: f32, sample_rate: f32) -> Self {
        let smoothing_coefficient_99_percent = |time| {
            let time_in_samples = time * sample_rate;
            let result = 1e-2f32.powf(1.0 / time_in_samples);
            flush_f32_to_zero(result)
        };

        Self {
            envelope_level: 0.0,
            attack_coefficent: smoothing_coefficient_99_percent(attack_time),
            release_coefficient: smoothing_coefficient_99_percent(release_time),
            hold_counter: 0,
            hold_time_in_samples: (hold_time * sample_rate) as u32,
        }
    }

    /// Processes a single sample of input, with output depending on the stage of the envelope
    pub fn process(&mut self, input: f32) -> f32 {
        if input > self.envelope_level {
            // Attack
            self.envelope_level =
                flush_f32_to_zero(self.attack_coefficent * (self.envelope_level - input) + input);
            // Reset the hold counter so that a hold is started as soon as the attack is over
            self.hold_counter = 0;
        } else if self.hold_counter < self.hold_time_in_samples {
            // Hold, don't adjust the envelope level
            self.hold_counter += 1;
        } else {
            // Release
            self.envelope_level =
                flush_f32_to_zero(self.release_coefficient * (self.envelope_level - input) + input);
        }

        self.envelope_level
    }
}

/// An envelope follower that tracks the RMS of the input signal
///
/// Internally this provides the result of passing the input signal through a [`WindowedMovingRms`],
/// followed by an [`EnvelopeFollower`].
///
/// See [`EnvelopeFollower`] for a usage example.
///
/// [`EnvelopeFollower`]: struct.EnvelopeFollower.html
/// [`WindowedMovingRms`]: struct.WindowedMovingRms.html
pub struct RmsEnvelopeFollower {
    rms: WindowedMovingRms,
    envelope_follower: EnvelopeFollower,
}

impl RmsEnvelopeFollower {
    /// Initializes the `RmsEnvelopeFollower` with specified windowing and stage duration times
    pub fn new(
        rms_windowing_time: f32,
        attack_time: f32,
        hold_time: f32,
        release_time: f32,
        sample_rate: f32,
    ) -> Self {
        Self {
            rms: WindowedMovingRms::new(rms_windowing_time, sample_rate),
            envelope_follower: EnvelopeFollower::new(
                attack_time,
                hold_time,
                release_time,
                sample_rate,
            ),
        }
    }

    /// Takes a single sample of input and returns its windowed RMS smoothed by an envelope follower
    pub fn process(&mut self, input: f32) -> f32 {
        let rms = self.rms.process(input);
        self.envelope_follower.process(rms)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn envelope_follower_with_one_second_stages() {
        let stage_time = 1.0;
        let sample_rate = 4.0;
        let mut follower = EnvelopeFollower::new(stage_time, stage_time, stage_time, sample_rate);

        // Silence in, silence out
        assert_eq!(follower.process(0.0), 0.0);
        assert_eq!(follower.process(0.0), 0.0);
        assert_eq!(follower.process(0.0), 0.0);
        assert_eq!(follower.process(0.0), 0.0);

        // 1.0 as input shows a ramped attack stage, reaching 99% of the input level after a second
        assert_eq!(follower.process(1.0), 0.6837722);
        assert_eq!(follower.process(1.0), 0.9);
        assert_eq!(follower.process(1.0), 0.96837723);
        assert_eq!(follower.process(1.0), 0.99);

        // Dropping the input level to 0.5 shows the hold stage maintaining the level for a second
        assert_eq!(follower.process(0.5), 0.99);
        assert_eq!(follower.process(0.5), 0.99);
        assert_eq!(follower.process(0.5), 0.99);
        assert_eq!(follower.process(0.5), 0.99);

        // The hold stage is now over, so we see a release towards the 0.5 input level
        assert_eq!(follower.process(0.5), 0.6549516);
        assert_eq!(follower.process(0.5), 0.54899997);
        assert_eq!(follower.process(0.5), 0.5154951);
        assert_eq!(follower.process(0.5), 0.5049);

        // 1.0 again as input to start a new attack
        assert_eq!(follower.process(1.0), 0.84343565);
        assert_eq!(follower.process(1.0), 0.95049);

        // Interrupt the attack with a drop to 0.0, triggering a hold
        assert_eq!(follower.process(0.0), 0.95049);
        assert_eq!(follower.process(0.0), 0.95049);
        assert_eq!(follower.process(0.0), 0.95049);
        assert_eq!(follower.process(0.0), 0.95049);

        // Now that the hold is over we see the release stage dropping towards zero
        assert_eq!(follower.process(0.0), 0.30057132);
        assert_eq!(follower.process(0.0), 0.095048994);
        assert_eq!(follower.process(0.0), 0.03005713);
        assert_eq!(follower.process(0.0), 0.009504899);
    }

    #[test]
    fn envelope_follower_with_no_hold_and_different_times() {
        let attack_time = 0.5;
        let hold_time = 0.0;
        let release_time = 1.0;
        let sample_rate = 8.0;
        let mut follower = EnvelopeFollower::new(attack_time, hold_time, release_time, sample_rate);

        // Attack stage, ramping up towards 1.0
        assert_eq!(follower.process(1.0), 0.6837722);
        assert_eq!(follower.process(1.0), 0.9);
        assert_eq!(follower.process(1.0), 0.96837723);
        assert_eq!(follower.process(1.0), 0.99);

        // Dropping the input level to 0.0 shows an immediate release
        // (due to the hold stage having a duration of 0.0)
        // with a length double that of the attack stage.
        assert_eq!(follower.process(0.0), 0.55671793);
        assert_eq!(follower.process(0.0), 0.3130655);
        assert_eq!(follower.process(0.0), 0.17604966);
        assert_eq!(follower.process(0.0), 0.099);
        assert_eq!(follower.process(0.0), 0.055671792);
        assert_eq!(follower.process(0.0), 0.03130655);
        assert_eq!(follower.process(0.0), 0.017604968);
        assert_eq!(follower.process(0.0), 0.009900001);
    }

    #[test]
    fn rms_envelope_follower_with_one_second_windowing_and_stages() {
        let windowing_time = 1.0;
        let stage_time = 1.0;
        let sample_rate = 4.0;
        let mut rms_follower = RmsEnvelopeFollower::new(
            windowing_time,
            stage_time,
            stage_time,
            stage_time,
            sample_rate,
        );

        // Silence in, silence out
        assert_eq!(rms_follower.process(0.0), 0.0);
        assert_eq!(rms_follower.process(0.0), 0.0);
        assert_eq!(rms_follower.process(0.0), 0.0);
        assert_eq!(rms_follower.process(0.0), 0.0);

        // 1.0 as input shows a ramped attack stage
        assert_eq!(rms_follower.process(1.0), 0.3418861);
        assert_eq!(rms_follower.process(1.0), 0.5916138);
        assert_eq!(rms_follower.process(1.0), 0.77924883);
        assert_eq!(rms_follower.process(1.0), 0.93019235);

        // Dropping the input level to 0.5 shows the hold stage maintaining the level for a second
        assert_eq!(rms_follower.process(0.5), 0.93019235);
        assert_eq!(rms_follower.process(0.5), 0.93019235);
        assert_eq!(rms_follower.process(0.5), 0.93019235);
        assert_eq!(rms_follower.process(0.5), 0.93019235);

        // The hold stage is now over, so we see a ramped release towards the 0.5 input level
        assert_eq!(rms_follower.process(0.5), 0.6360388);
        assert_eq!(rms_follower.process(0.5), 0.54301924);
        assert_eq!(rms_follower.process(0.5), 0.51360387);
        assert_eq!(rms_follower.process(0.5), 0.5043019);

        // 1.0 again as input to start a new attack
        assert_eq!(rms_follower.process(1.0), 0.6117471);
        assert_eq!(rms_follower.process(1.0), 0.7340208);

        // Interrupt the attack with a drop to 0.0, triggering a hold.
        // Because the windowed RMS still has a slightly higher value than the input, the hold
        // stage takes an extra sample to kick in, so here we pass in 5 samples of input to reach
        // the end of the hold stage.
        assert_eq!(rms_follower.process(0.0), 0.74494696);
        assert_eq!(rms_follower.process(0.0), 0.74494696);
        assert_eq!(rms_follower.process(0.0), 0.74494696);
        assert_eq!(rms_follower.process(0.0), 0.74494696);
        assert_eq!(rms_follower.process(0.0), 0.74494696);

        // Now that the hold is over we see the release stage dropping towards zero
        assert_eq!(rms_follower.process(0.0), 0.2355729);
        assert_eq!(rms_follower.process(0.0), 0.07449469);
        assert_eq!(rms_follower.process(0.0), 0.023557289);
        assert_eq!(rms_follower.process(0.0), 0.0074494686);
    }
}
