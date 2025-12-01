//! Audio Decoder Module
//!
//! Provides audio decoding capabilities using Symphonia
//! Supports MP3, WAV, M4A, AAC, OGG, and FLAC formats

use anyhow::{Result, anyhow};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::time::Duration;
use tracing::{info, error, warn, debug};

use symphonia::core::audio::{AudioBufferRef, Signal};
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

use crate::audio::{AudioFrameData, AudioMetadata, AudioSampleFormat, AudioBuffer};

/// Audio decoder for various audio formats
pub struct AudioDecoder {
    metadata: AudioMetadata,
    current_position: Duration,
}

impl AudioDecoder {
    /// Create a new audio decoder
    pub fn new() -> Self {
        Self {
            metadata: AudioMetadata::new(),
            current_position: Duration::from_millis(0),
        }
    }

    /// Open and analyze an audio file
    pub fn open<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        let path = path.as_ref();
        info!("Opening audio file: {}", path.display());

        // Create a file reader
        let file = File::open(path)
            .map_err(|e| anyhow!("Failed to open audio file: {}", e))?;

        // Create a media source stream
        let mss = MediaSourceStream::new(Box::new(file), Default::default());

        // Create a probe hint
        let mut hint = Hint::new();
        if let Some(extension) = path.extension().and_then(|ext| ext.to_str()) {
            hint.with_extension(extension);
        }

        // Probe the format
        let meta_opts: MetadataOptions = Default::default();
        let fmt_opts: FormatOptions = Default::default();

        let probed = symphonia::default::get_probe()
            .format(&hint, mss, &fmt_opts, &meta_opts)
            .map_err(|e| anyhow!("Failed to probe audio format: {}", e))?;

        let mut format = probed.format;

        // Find the first audio track
        let track = format.tracks()
            .iter()
            .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
            .ok_or_else(|| anyhow!("No audio track found in file"))?;

        let track_id = track.id;

        // Extract metadata
        self.metadata = self.extract_metadata(track, &format);

        info!("Audio track loaded: {} channels, {} Hz, codec: {}",
              self.metadata.channels,
              self.metadata.sample_rate,
              self.metadata.codec);

        Ok(())
    }

    /// Decode entire audio file into frames
    pub fn decode_all<P: AsRef<Path>>(&mut self, path: P, buffer_size: usize) -> Result<Vec<AudioFrameData>> {
        let path = path.as_ref();
        info!("Decoding audio file: {}", path.display());

        // Open file for decoding
        let file = File::open(path)
            .map_err(|e| anyhow!("Failed to open audio file for decoding: {}", e))?;

        let mss = MediaSourceStream::new(Box::new(file), Default::default());

        // Create a probe hint
        let mut hint = Hint::new();
        if let Some(extension) = path.extension().and_then(|ext| ext.to_str()) {
            hint.with_extension(extension);
        }

        // Probe the format
        let meta_opts: MetadataOptions = Default::default();
        let fmt_opts: FormatOptions = Default::default();

        let probed = symphonia::default::get_probe()
            .format(&hint, mss, &fmt_opts, &meta_opts)
            .map_err(|e| anyhow!("Failed to probe audio format: {}", e))?;

        let mut format = probed.format;

        // Find the first audio track
        let track = format.tracks()
            .iter()
            .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
            .ok_or_else(|| anyhow!("No audio track found in file"))?;

        let track_id = track.id;

        // Create decoder
        let dec_opts: DecoderOptions = Default::default();
        let mut decoder = symphonia::default::get_codecs()
            .make(&track.codec_params, &dec_opts)
            .map_err(|e| anyhow!("Failed to create decoder: {}", e))?;

        let mut frames = Vec::new();
        let mut frame_number = 0u64;
        let mut current_timestamp = Duration::from_millis(0);

        // Decode loop
        loop {
            // Get the next packet from the format
            let packet = match format.next_packet() {
                Ok(packet) => packet,
                Err(SymphoniaError::ResetRequired) => {
                    warn!("Decoder reset required");
                    continue;
                }
                Err(SymphoniaError::IoError(ref err)) if err.kind() == std::io::ErrorKind::UnexpectedEof => {
                    debug!("End of file reached");
                    break;
                }
                Err(err) => {
                    error!("Error reading packet: {}", err);
                    break;
                }
            };

            // If the packet does not belong to the selected track, skip it
            if packet.track_id() != track_id {
                continue;
            }

            // Decode the packet
            match decoder.decode(&packet) {
                Ok(decoded) => {
                    let audio_frame = self.process_decoded_audio(
                        decoded,
                        track,
                        frame_number,
                        current_timestamp,
                        buffer_size,
                    )?;

                    if let Some(frame) = audio_frame {
                        current_timestamp += frame.duration;
                        frame_number += 1;
                        frames.push(frame);
                    }
                }
                Err(SymphoniaError::IoError(ref err)) if err.kind() == std::io::ErrorKind::UnexpectedEof => {
                    debug!("End of stream");
                    break;
                }
                Err(SymphoniaError::DecodeError(_)) => {
                    // Skip decode errors but continue
                    warn!("Decode error, skipping packet");
                    continue;
                }
                Err(err) => {
                    error!("Decoding error: {}", err);
                    break;
                }
            }
        }

        info!("Decoded {} audio frames", frames.len());
        Ok(frames)
    }

    /// Process decoded audio data into an AudioFrameData
    fn process_decoded_audio(
        &self,
        decoded: AudioBufferRef,
        track: &symphonia::core::formats::Track,
        frame_number: u64,
        timestamp: Duration,
        buffer_size: usize,
    ) -> Result<Option<AudioFrameData>> {
        match decoded {
            AudioBufferRef::U8(buf) => {
                self.convert_audio_buffer(buf, track, frame_number, timestamp, buffer_size)
            }
            AudioBufferRef::U16(buf) => {
                self.convert_audio_buffer(buf, track, frame_number, timestamp, buffer_size)
            }
            AudioBufferRef::U24(buf) => {
                self.convert_audio_buffer(buf, track, frame_number, timestamp, buffer_size)
            }
            AudioBufferRef::U32(buf) => {
                self.convert_audio_buffer(buf, track, frame_number, timestamp, buffer_size)
            }
            AudioBufferRef::S8(buf) => {
                self.convert_audio_buffer(buf, track, frame_number, timestamp, buffer_size)
            }
            AudioBufferRef::S16(buf) => {
                self.convert_audio_buffer(buf, track, frame_number, timestamp, buffer_size)
            }
            AudioBufferRef::S24(buf) => {
                self.convert_audio_buffer(buf, track, frame_number, timestamp, buffer_size)
            }
            AudioBufferRef::S32(buf) => {
                self.convert_audio_buffer(buf, track, frame_number, timestamp, buffer_size)
            }
            AudioBufferRef::F32(buf) => {
                self.convert_audio_buffer(buf, track, frame_number, timestamp, buffer_size)
            }
            AudioBufferRef::F64(buf) => {
                self.convert_audio_buffer(buf, track, frame_number, timestamp, buffer_size)
            }
        }
    }

    /// Convert audio buffer to AudioFrameData
    fn convert_audio_buffer<T>(
        &self,
        buf: &symphonia::core::audio::AudioBuffer<T>,
        track: &symphonia::core::formats::Track,
        frame_number: u64,
        timestamp: Duration,
        buffer_size: usize,
    ) -> Result<Option<AudioFrameData>>
    where
        T: symphonia::core::sample::Sample,
        T: symphonia::core::sample::IntoSample<f32>,
    {
        if buf.frames() == 0 {
            return Ok(None);
        }

        let channels = buf.spec().channels.count() as u32;
        let sample_rate = buf.spec().rate;
        let frames = buf.frames();
        let duration = Duration::from_secs_f64(frames as f64 / sample_rate as f64);

        // Convert to interleaved f32 samples
        let mut interleaved = Vec::with_capacity(frames * channels as usize);

        for frame_idx in 0..frames {
            for ch_idx in 0..channels {
                if let Some(sample) = buf.chans().get(ch_idx as usize) {
                    if let Some(sample_value) = sample.get(frame_idx) {
                        let f32_sample = sample_value.into_sample();
                        interleaved.push(f32_sample);
                    }
                }
            }
        }

        // Convert f32 samples to bytes
        let data: Vec<u8> = unsafe {
            std::slice::from_raw_parts(
                interleaved.as_ptr() as *const u8,
                interleaved.len() * 4, // 4 bytes per f32
            ).to_vec()
        };

        let audio_frame = AudioFrameData::new(
            data,
            channels,
            sample_rate,
            AudioSampleFormat::F32,
            timestamp,
            frame_number,
            duration,
        );

        Ok(Some(audio_frame))
    }

    /// Extract metadata from audio track
    fn extract_metadata(
        &self,
        track: &symphonia::core::formats::Track,
        format: &symphonia::core::formats::BoxFormatReader,
    ) -> AudioMetadata {
        let mut metadata = AudioMetadata::new();

        // Get codec parameters
        if let Some(codec_params) = track.codec_params.as_ref() {
            metadata.channels = codec_params.channels.map_or(0, |c| c.count() as u32);
            metadata.sample_rate = codec_params.sample_rate.unwrap_or(0);
            metadata.bit_rate = codec_params.max_bit_rate;

            if let Some(codec) = codec_params.codec {
                metadata.codec = format!("{:?}", codec);
            }
        }

        // Get format information
        metadata.format = format.to_string();

        // Try to get duration from the format
        // Note: This might not be available for all formats
        // In a full implementation, you'd scan the entire file to calculate duration

        metadata
    }

    /// Get metadata for the currently loaded file
    pub fn get_metadata(&self) -> &AudioMetadata {
        &self.metadata
    }

    /// Get current playback position
    pub fn get_position(&self) -> Duration {
        self.current_position
    }

    /// Set current playback position
    pub fn set_position(&mut self, position: Duration) {
        self.current_position = position;
    }
}

impl Default for AudioDecoder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::AudioValidator;
    use std::path::PathBuf;

    #[test]
    fn test_audio_decoder_creation() {
        let decoder = AudioDecoder::new();
        assert_eq!(decoder.get_position(), Duration::from_millis(0));
        assert_eq!(decoder.get_metadata().channels, 0);
    }

    #[test]
    fn test_metadata_extraction() {
        let decoder = AudioDecoder::new();
        let metadata = decoder.get_metadata();
        assert_eq!(metadata.channels, 0);
        assert_eq!(metadata.sample_rate, 0);
        assert!(metadata.codec.is_empty());
    }

    // Note: Integration tests that require actual audio files
    // would need to be placed in a separate test file with test fixtures
}