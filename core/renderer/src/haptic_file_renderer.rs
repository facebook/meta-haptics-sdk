// Copyright (c) Meta Platforms, Inc. and affiliates.
//
// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

use std::io;
use std::marker::PhantomData;
use std::ops::Deref;

use dasp_sample::FromSample;
use dasp_sample::I24;
use dasp_sample::Sample;
use haptic_data::HapticData;

use crate::Error;
use crate::Result;
use crate::StreamingRenderer;
use crate::acf::Acf;
use crate::render_settings::OutputFormat;
use crate::render_settings::RenderMode;
use crate::render_settings::RenderSettings;
use crate::render_settings::SampleFormat;
use crate::streaming_event_reader::StreamingEventReader;

/// Renders a complete haptic to audio, with helpers for writing to audio files
pub struct HapticFileRenderer<H>
where
    H: Deref<Target = HapticData>,
{
    render_settings: RenderSettings,
    event_reader: StreamingEventReader<H>,
    renderer: StreamingRenderer,
    sample_duration: f32,
    sample_count: f32,
    samples_processed: f32,
}

impl<H> HapticFileRenderer<H>
where
    H: Deref<Target = HapticData>,
{
    /// Makes a new HapticFileRenderer with the given ACF and render settings
    pub fn new(haptic_data: H, acf: Acf, render_settings: RenderSettings) -> Result<Self> {
        let sample_rate = render_settings.sample_rate as f32;

        let amp_envelope = &haptic_data.signals.continuous.envelopes.amplitude;
        let sample_count = match (amp_envelope.first(), amp_envelope.last()) {
            (Some(first), Some(last)) => ((last.time - first.time) * sample_rate).floor(),
            _ => {
                return Err(Error::InvalidHapticData(
                    "Not enough amplitude points".to_string(),
                ));
            }
        };

        let event_reader = StreamingEventReader::new(haptic_data);
        let renderer = StreamingRenderer::new(acf, sample_rate, render_settings.render_mode);

        Ok(Self {
            render_settings,
            event_reader,
            renderer,
            sample_duration: 1.0 / sample_rate,
            sample_count,
            samples_processed: 0.0,
        })
    }

    /// Writes the renderer's output to a buffer
    ///
    /// The writer can be anything that implements std::io::Write and std::io::Seek,
    /// e.g. a file, or a in-memory buffer using std::io::Cursor.
    ///
    /// The output format depends on the [RenderSettings] passed into [HapticFileRenderer::new].
    pub fn write_to_buffer(&mut self, writer: &mut (impl io::Write + io::Seek)) -> Result<()> {
        match self.render_settings.output_format {
            OutputFormat::Raw => self.write_to_raw(writer).map_err(Error::IoError),
            OutputFormat::Csv => self.write_to_csv(writer).map_err(Error::IoError),
            OutputFormat::Wav => {
                #[cfg(feature = "wav")]
                {
                    self.write_to_wav(writer).map_err(Error::WavWriteError)
                }

                #[cfg(not(feature = "wav"))]
                {
                    unimplemented!("Rending to Wav requires the `wav` feature")
                }
            }
        }
    }

    /// Writes all output to a wav file using the given writer
    #[cfg(feature = "wav")]
    pub fn write_to_wav(&mut self, writer: &mut (impl io::Write + io::Seek)) -> hound::Result<()> {
        use SampleFormat::*;
        use hound::WavSpec;
        use hound::WavWriter;

        let mut wav_writer = WavWriter::new(
            writer,
            WavSpec {
                channels: 1,
                sample_rate: self.render_settings.sample_rate,
                bits_per_sample: self.render_settings.sample_format.bits(),
                sample_format: self.render_settings.sample_format.integer_or_float().into(),
            },
        )?;

        match self.render_settings.sample_format {
            Unsigned8 => {
                // Hound deals with signed 8-bit data in its interface,
                // and writes out to unsigned 8-bit values
                for output in self.output::<i8>() {
                    wav_writer.write_sample(output)?;
                }
            }
            Signed16 => {
                for output in self.output::<i16>() {
                    wav_writer.write_sample(output)?;
                }
            }
            Signed24 => {
                for output in self.output::<I24>() {
                    wav_writer.write_sample(output.inner())?;
                }
            }
            Signed32 => {
                for output in self.output::<i32>() {
                    wav_writer.write_sample(output)?;
                }
            }
            Float32 => {
                for output in self.output::<f32>() {
                    wav_writer.write_sample(output)?;
                }
            }
        }

        wav_writer.finalize()?;

        Ok(())
    }

    /// Writes all output as raw samples to the provided writer
    pub fn write_to_raw(&mut self, writer: &mut impl io::Write) -> io::Result<()> {
        use RenderMode::*;
        use SampleFormat::*;

        match (
            self.render_settings.sample_format,
            self.render_settings.render_mode,
        ) {
            (Unsigned8, AmpCurve) => {
                for output in self.output::<f32>() {
                    // To get a full-resolution u8 amplitude curve, we render the curve as f32,
                    // then scale it manually.
                    let output = (output * 255.0) as u8;
                    writer.write_all(&output.to_le_bytes())?;
                }
            }
            (Unsigned8, Synthesis) => {
                for output in self.output::<u8>() {
                    writer.write_all(&output.to_le_bytes())?;
                }
            }
            (Signed16, _) => {
                for output in self.output::<i16>() {
                    writer.write_all(&output.to_le_bytes())?;
                }
            }
            (Signed24, _) => {
                for output in self.output::<I24>() {
                    writer.write_all(&output.inner().to_le_bytes()[..3])?;
                }
            }
            (Signed32, _) => {
                for output in self.output::<i32>() {
                    writer.write_all(&output.to_le_bytes())?;
                }
            }
            (Float32, _) => {
                for output in self.output::<f32>() {
                    writer.write_all(&output.to_le_bytes())?;
                }
            }
        }

        Ok(())
    }

    /// Writes all output as CSV lines
    pub fn write_to_csv(&mut self, writer: &mut impl io::Write) -> io::Result<()> {
        use RenderMode::*;
        use SampleFormat::*;

        match (
            self.render_settings.sample_format,
            self.render_settings.render_mode,
        ) {
            (Unsigned8, AmpCurve) => {
                for output in self.output::<f32>() {
                    // To get a full-resolution u8 amplitude curve, we render the curve as f32,
                    // then scale it manually.
                    let output = (output * 255.0) as u8;
                    writeln!(writer, "{output}")?;
                }
            }
            (Unsigned8, Synthesis) => {
                for output in self.output::<u8>() {
                    writeln!(writer, "{output}")?;
                }
            }
            (Signed16, _) => {
                for output in self.output::<i16>() {
                    writeln!(writer, "{output}")?;
                }
            }
            (Signed24, _) => {
                for output in self.output::<I24>() {
                    writeln!(writer, "{}", output.inner())?;
                }
            }
            (Signed32, _) => {
                for output in self.output::<i32>() {
                    writeln!(writer, "{output}")?;
                }
            }
            (Float32, _) => {
                for output in self.output::<f32>() {
                    writeln!(writer, "{output}")?;
                }
            }
        }

        Ok(())
    }

    // Provides the next sample of output
    fn process(&mut self) -> f32 {
        let current_time = self.samples_processed * self.sample_duration;
        self.samples_processed += 1.0;
        self.renderer.process(current_time, &mut self.event_reader)
    }

    /// Provides an iterator that yields the renderer's output samples with the specified format
    pub fn output<S: FromSample<f32>>(&mut self) -> HapticFileRendererIterator<'_, H, S> {
        HapticFileRendererIterator::<H, S>::new(self)
    }

    // Returns true when the input haptic has been fully rendered
    fn is_finished(&self) -> bool {
        self.samples_processed == self.sample_count
    }
}

/// An iterator the provides the output of a haptic renderer, see `HapticFileRenderer::output`
pub struct HapticFileRendererIterator<'renderer, H, S>
where
    H: Deref<Target = HapticData>,
    S: FromSample<f32>,
{
    renderer: &'renderer mut HapticFileRenderer<H>,
    _phantom: PhantomData<S>,
}

impl<'renderer, H, S> HapticFileRendererIterator<'renderer, H, S>
where
    H: Deref<Target = HapticData>,
    S: FromSample<f32>,
{
    pub fn new(renderer: &'renderer mut HapticFileRenderer<H>) -> Self {
        Self {
            renderer,
            _phantom: PhantomData,
        }
    }
}

impl<'renderer, H, S> Iterator for HapticFileRendererIterator<'renderer, H, S>
where
    H: Deref<Target = HapticData>,
    S: FromSample<f32>,
{
    type Item = S;

    fn next(&mut self) -> Option<Self::Item> {
        if !self.renderer.is_finished() {
            let output = self.renderer.process();
            Some(output.to_sample::<S>())
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use std::f32::consts::FRAC_1_SQRT_2;

    use haptic_data::test_utils::*;
    use haptic_dsp::test_utils::is_near;

    use super::*;
    use crate::continuous_oscillator_settings::ContinuousOscillatorSettings;
    use crate::emphasis_oscillator_settings::EmphasisFrequencySettings;
    use crate::emphasis_oscillator_settings::EmphasisOscillatorSettings;
    use crate::emphasis_oscillator_settings::EmphasisShape;
    use crate::render_settings::RenderMode;

    const TEST_ACF: Acf = Acf {
        continuous: ContinuousOscillatorSettings {
            gain: 1.0,
            emphasis_ducking: 0.0,
            frequency_min: 0.0,
            frequency_max: 1.0,
        },
        emphasis: EmphasisOscillatorSettings {
            gain: 1.0,
            fade_out_percent: 0.0,
            frequency_min: EmphasisFrequencySettings {
                output_frequency: 1.0,
                duration_ms: 1000.0,
                shape: EmphasisShape::Square,
            },
            frequency_max: EmphasisFrequencySettings {
                output_frequency: 1.0,
                duration_ms: 1000.0,
                shape: EmphasisShape::Saw,
            },
        },
    };

    fn check_renderer_output<H>(renderer: &mut HapticFileRenderer<H>, expected_output: &[f32])
    where
        H: Deref<Target = HapticData>,
    {
        assert!(!renderer.is_finished());

        let allowed_error = 1.0e-6;

        for (i, (expected, actual)) in expected_output
            .iter()
            .zip(renderer.output::<f32>())
            .enumerate()
        {
            if !is_near(*expected, actual, allowed_error) {
                panic!(
                    "Unexpected renderer output at index {i}: \
                     the difference between '{expected}' and '{actual}' is greater than the allowed error of '{allowed_error}",
                );
            }
        }
    }

    #[test]
    fn simple_output() {
        let sample_rate = 4;
        let acf = TEST_ACF;
        let render_settings = RenderSettings {
            render_mode: RenderMode::Synthesis,
            output_format: OutputFormat::Raw,
            sample_rate,
            sample_format: SampleFormat::Float32,
        };
        let haptic_data = TestClip {
            amplitude: &[
                // Immediately set the amplitude to 1
                amp_bp(0.0, 1.0),
                // At time 1, trigger an emphasis event with a frequency of 0
                emphasis_bp(1.0, 1.0, 1.0, 0.0),
                // At time 3, jump in amplitude immediately from 1.0 to 0.5,
                // and trigger another emphasis event with frequency of 1 and half amplitude
                amp_bp(3.0, 1.0),
                emphasis_bp(3.0, 0.5, 0.5, 1.0),
                // Place an end event at time 5
                amp_bp(5.0, 0.5),
            ],
            frequency: &[
                // Immediately set the frequency to 1.0
                freq_bp(0.0, 1.0),
                // At time 4, jump without ramping to a frequency of 0.5
                freq_bp(4.0, 1.0),
                freq_bp(4.0, 0.5),
            ],
        }
        .into();

        let mut renderer = HapticFileRenderer::new(&haptic_data, acf, render_settings).unwrap();

        // Time 0-1 - continuous output at amplitude 1
        check_renderer_output(&mut renderer, &[0.0, 1.0, 0.0, -1.0]);

        // Time 1-2 - emphasis event, square shape
        check_renderer_output(&mut renderer, &[1.0, 1.0, -1.0, -1.0]);

        // Time 2-3 - continuous output as before
        check_renderer_output(&mut renderer, &[0.0, 1.0, 0.0, -1.0]);

        // Time 3-4 - emphasis event, saw shape, half amplitude
        check_renderer_output(&mut renderer, &[0.5, 0.25, 0.0, -0.25]);

        // Time 4-5 - continuous output with half amplitude and half frequency
        check_renderer_output(
            &mut renderer,
            &[0.0, FRAC_1_SQRT_2 / 2.0, 0.5, FRAC_1_SQRT_2 / 2.0, 0.0],
        );

        assert!(renderer.is_finished());
    }

    #[test]
    fn amp_curve_raw_u8() {
        let sample_rate = 4;
        let acf = TEST_ACF;
        let render_settings = RenderSettings {
            render_mode: RenderMode::AmpCurve,
            output_format: OutputFormat::Raw,
            sample_rate,
            sample_format: SampleFormat::Unsigned8,
        };
        let haptic_data = TestClip {
            amplitude: &[
                // Immediately set the amplitude to 1
                amp_bp(0.0, 1.0),
                // At time 1, trigger an emphasis event with a frequency of 0
                emphasis_bp(1.0, 1.0, 1.0, 0.0),
                // At time 3, jump in amplitude immediately from 1.0 to 0.5,
                // and trigger another emphasis event with frequency of 1 and half amplitude
                amp_bp(3.0, 1.0),
                emphasis_bp(3.0, 0.5, 0.5, 1.0),
                // Place an end event at time 5
                amp_bp(5.0, 0.5),
            ],
            frequency: &[
                // Immediately set the frequency to 1.0
                freq_bp(0.0, 1.0),
                // At time 4, jump without ramping to a frequency of 0.5
                freq_bp(4.0, 1.0),
                freq_bp(4.0, 0.5),
            ],
        }
        .into();

        let mut renderer = HapticFileRenderer::new(&haptic_data, acf, render_settings).unwrap();

        // Time 0-3 - continuous output at amplitude 1
        check_renderer_output(
            &mut renderer,
            &[1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0],
        );

        // Time 3-5 - amplitude ramps from 1 down to 0.5
        check_renderer_output(
            &mut renderer,
            &[0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5],
        );

        assert!(renderer.is_finished());
    }

    #[test]
    fn check_long_clip_sample_count() {
        let sample_rate = 2000;
        let acf = Acf::default();
        let render_settings = RenderSettings {
            render_mode: RenderMode::Synthesis,
            output_format: OutputFormat::Raw,
            sample_rate,
            sample_format: SampleFormat::Float32,
        };

        // Set up a clip that lasts for 10 seconds
        let clip_duration = 10.0;
        let haptic_data = TestClip {
            amplitude: &[amp_bp(0.0, 0.0), amp_bp(clip_duration, 1.0)],
            frequency: &[],
        }
        .into();

        let mut renderer = HapticFileRenderer::new(&haptic_data, acf, render_settings).unwrap();

        let expected_sample_count = ((sample_rate as f32) * clip_duration) as usize;
        assert_eq!(expected_sample_count, renderer.output::<f32>().count());

        assert!(renderer.is_finished());
    }
}
