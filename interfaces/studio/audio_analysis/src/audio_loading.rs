// (c) Meta Platforms, Inc. and affiliates. Confidential and proprietary.

use std::error::Error;
use std::io::Cursor;

use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::CODEC_TYPE_NULL;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::FormatOptions;
use symphonia::core::formats::Track;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

/// Contains monophonic audio data decoded to f32 samples, along with the data's sample rate
pub struct MonoAudioData {
    /// Monophonic f32 audio samples
    pub data: Vec<f32>,
    /// The sample rate of the data
    pub sample_rate: usize,
}

/// Behavior when extracting audio channels from audio with multiple channels
pub enum MultiChannelBehavior {
    /// Sum and average over all channels
    Downmix,
    /// Use only first channel of audio
    ExtractFirstChannel,
}

/// Loads audio data from a given byte slice, using the provided extension as a format hint
pub fn load_audio_data(
    bytes: &[u8],
    extension: Option<&str>,
    multichannel_behavior: &MultiChannelBehavior,
) -> Result<MonoAudioData, Box<dyn Error>> {
    let reader = Cursor::new(bytes.to_vec());
    load_audio_from_source_stream(
        MediaSourceStream::new(Box::new(reader), Default::default()),
        extension,
        multichannel_behavior,
    )
}

fn load_audio_from_source_stream(
    source_stream: MediaSourceStream,
    extension: Option<&str>,
    multichannel_behavior: &MultiChannelBehavior,
) -> Result<MonoAudioData, Box<dyn Error>> {
    let mut format_reader = {
        // Prepare a hint for the format probe based on the file extension
        let probe_hint = {
            let mut hint = Hint::new();
            if let Some(extension) = extension {
                hint.with_extension(extension);
            }
            hint
        };

        // Probe the file data to discover its format
        let probe = symphonia::default::get_probe().format(
            &probe_hint,
            source_stream,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )?;

        probe.format
    };

    let track = first_supported_track(format_reader.tracks())
        .ok_or_else(|| "Failed to get default audio track".to_string())?;

    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions { verify: true })?;

    let track_id = track.id;
    let sample_rate = track
        .codec_params
        .sample_rate
        .ok_or_else(|| "Failed to get sample rate".to_string())? as usize;
    let channels = track
        .codec_params
        .channels
        .ok_or_else(|| "Failed to get number of channels".to_string())?
        .count();
    let total_frames = track
        .codec_params
        .n_frames
        .ok_or_else(|| "Failed to get number of frames".to_string())?
        as usize;

    let mut sample_buffer = None;
    let mut mono_data = Vec::new();

    // We need to keep track of the decoded frame count in case the decoder provides additional
    // unexpected frames, e.g. see https://github.com/pdeljanov/Symphonia/issues/85
    let mut frame_count = 0;

    // Read each packet in the format reader
    while let Ok(packet) = format_reader.next_packet() {
        if packet.track_id() != track_id {
            // Skip over packets that belong to another track
            continue;
        }

        // Decode the packet, resulting in an audio buffer
        match decoder.decode(&packet) {
            Ok(audio_buffer) => {
                // Prepare the sample buffer that we'll copy data into from the audio buffer
                let sample_buffer = sample_buffer.get_or_insert_with(|| {
                    let spec = *audio_buffer.spec();
                    let duration = audio_buffer.capacity() as u64;
                    SampleBuffer::<f32>::new(duration, spec)
                });

                // Limit the number of frames to avoid running past the end of the file's declared
                // frame count.
                let packet_frames = audio_buffer.frames().min(total_frames - frame_count);
                frame_count += packet_frames;

                sample_buffer.copy_interleaved_ref(audio_buffer);
                let samples = &sample_buffer.samples()[..(packet_frames * channels)];

                if channels == 1 {
                    mono_data.extend_from_slice(samples);
                } else {
                    match multichannel_behavior {
                        // Mix the frame's samples to mono
                        MultiChannelBehavior::Downmix => {
                            mono_data.extend(samples.chunks_exact(channels).map(|frame| {
                                frame.iter().fold(0.0, |result, sample| result + sample)
                                    / channels as f32
                            }))
                        }
                        // Extract only first channel
                        MultiChannelBehavior::ExtractFirstChannel => {
                            mono_data.extend(samples.chunks_exact(channels).map(|frame| frame[0]))
                        }
                    }
                }
            }
            Err(error) => return Err(error.into()),
        }
    }

    Ok(MonoAudioData {
        data: mono_data,
        sample_rate,
    })
}

fn first_supported_track(tracks: &[Track]) -> Option<&Track> {
    tracks
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;
    use std::path::PathBuf;

    use test_case::test_case;

    use super::*;

    fn path_to_test_file(path: &str) -> PathBuf {
        match option_env!("CARGO_MANIFEST_DIR") {
            Some(dir) => Path::new(dir).join("../../../resources/audio").join(path),
            None => {
                let test_file_path = std::env::var("TEST_FILES").unwrap();
                Path::new(&test_file_path).join(path)
            }
        }
    }

    #[test_case("sine_16_44100", "flac", 44100, 1.0)]
    #[test_case("sine_16_44100", "mp3", 44100, 1.0)]
    #[test_case("sine_16_44100", "ogg", 44100, 1.0)]
    #[test_case("sine_16_44100", "wav", 44100, 1.0)]
    #[test_case("sine_16_44100", "aiff", 44100, 1.0)]
    #[test_case("sine_8_16000", "flac", 16000, 2.0)]
    #[test_case("sine_8_16000", "mp3", 16000, 2.0)]
    #[test_case("sine_8_16000", "ogg", 16000, 2.0)]
    #[test_case("sine_8_16000", "wav", 16000, 2.0)]
    #[test_case("sine_8_16000", "aiff", 16000, 2.0)]
    fn load_test_audio_file(
        name: &str,
        extension: &str,
        expected_sample_rate: usize,
        expected_duration: f64,
    ) {
        let path = path_to_test_file(&format!("{name}.{extension}"));
        let data = fs::read(path).unwrap();
        let decoded =
            load_audio_data(&data, Some(extension), &MultiChannelBehavior::Downmix).unwrap();

        assert_eq!(expected_sample_rate, decoded.sample_rate);

        let expected_sample_count = (expected_duration * expected_sample_rate as f64) as usize;
        let actual_sample_count = decoded.data.len();
        if matches!(extension, "mp3" | "ogg") {
            // Give lossy formats a 5% margin for the expected sample count
            assert!(actual_sample_count >= expected_sample_count);
            assert!(actual_sample_count <= (expected_sample_count as f64 * 1.05) as usize);
        } else {
            assert_eq!(expected_sample_count, actual_sample_count);
        }
    }

    #[test_case("constant_2ch_16_44100", "wav", &MultiChannelBehavior::Downmix)]
    #[test_case("constant_2ch_16_44100", "wav", &MultiChannelBehavior::ExtractFirstChannel)]
    fn multichannel_behavior(name: &str, extension: &str, behavior: &MultiChannelBehavior) {
        let path = path_to_test_file(&format!("{name}.{extension}"));
        let data = fs::read(path).unwrap();
        let decoded = load_audio_data(&data, Some(extension), behavior).unwrap();

        let mut mean_data = 0.0f32;
        let num_samples = decoded.data.len() as f32;
        for data in decoded.data {
            mean_data += data;
        }
        mean_data /= num_samples;

        match behavior {
            // NB: this is specific to the files used in this test
            // where the left channel is 0.5 constant and the right channel is -0.5 constant
            // downmixing should result in 0.0
            // using only the first channel should be 0.5
            MultiChannelBehavior::Downmix => assert!(mean_data.abs() < 1e-10),
            MultiChannelBehavior::ExtractFirstChannel => {
                assert!((mean_data - 0.5f32).abs() < 1e-10)
            }
        }
    }
}
