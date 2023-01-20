//! Implements the SoundData trait for the Kira audio library.

use anyhow::{anyhow, Result};
use kira::clock::clock_info::ClockInfoProvider;
use kira::dsp::Frame;
use kira::sound::{Sound, SoundData};
use kira::track::TrackId;
use ringbuf::{HeapConsumer, HeapProducer, HeapRb};
use shin_core::format::audio::{AudioDecoder, AudioFile};
use shin_core::time::{Ticks, Tween, Tweener};
use std::f32::consts::SQRT_2;
use std::sync::atomic::AtomicU32;
use std::sync::Arc;
use tracing::{debug, warn};

use super::resampler::Resampler;
use super::{Audio, AudioParams, AudioWaitStatus};

impl Audio {
    pub fn to_kira_data(self: Arc<Self>, params: AudioParams) -> AudioData {
        AudioData(ArcAudio(self), params)
    }
}

// more newtypes to the newtype god
struct ArcAudio(Arc<Audio>);

impl AsRef<AudioFile> for ArcAudio {
    fn as_ref(&self) -> &AudioFile {
        &self.0 .0
    }
}

const COMMAND_BUFFER_CAPACITY: usize = 8;

/// Unfortunately, it's not possible to implement SoundData for Arc<AudioData>, so we use a newtype
pub struct AudioData(ArcAudio, AudioParams);

#[derive(Debug, Clone, Copy, PartialEq)]
enum Command {
    SetVolume(f32, Tween),
    SetPanning(f32, Tween),
    Stop(Tween),
    // TODO: how should BGMWAIT be implemented
}

struct Shared {
    wait_status: AtomicU32,
    // TODO: in what unit
    position: AtomicU32,
    // used for lip sync
    amplitude: AtomicU32,
}

impl Shared {
    fn new() -> Self {
        Self {
            wait_status: AtomicU32::new(0),
            position: AtomicU32::new(0),
            amplitude: AtomicU32::new(0),
        }
    }
}

pub struct AudioHandle {
    command_producer: HeapProducer<Command>,
    shared: Arc<Shared>,
}

impl AudioHandle {
    pub fn get_wait_status(&self) -> AudioWaitStatus {
        AudioWaitStatus::from_bits_truncate(
            self.shared
                .wait_status
                .load(std::sync::atomic::Ordering::SeqCst),
        )
    }

    pub fn get_amplitude(&self) -> f32 {
        f32::from_bits(
            self.shared
                .amplitude
                .load(std::sync::atomic::Ordering::SeqCst),
        )
    }

    /// Sets the volume of the sound.
    /// The volume is a value between 0.0 and 1.0, on the linear scale.
    pub fn set_volume(&mut self, volume: f32, tween: Tween) -> Result<()> {
        let volume = volume.clamp(0.0, 1.0); // TODO: warn if clamped

        self.command_producer
            .push(Command::SetVolume(volume, tween))
            .map_err(|_| anyhow!("Command queue full"))
    }

    /// Sets the panning of the sound, where `0.0` is the center and `-1.0` is the hard left and `1.0` is the hard right.
    pub fn set_panning(&mut self, panning: f32, tween: Tween) -> Result<()> {
        let panning = panning.clamp(-1.0, 1.0); // TODO: warn if clamped

        self.command_producer
            .push(Command::SetPanning(panning, tween))
            .map_err(|_| anyhow!("Command queue full"))
    }

    /// Fades out the sound to silence with the given tween and then
    /// stops playback.
    ///
    /// Once the sound is stopped, it cannot be restarted.
    pub fn stop(&mut self, tween: Tween) -> Result<()> {
        self.command_producer
            .push(Command::Stop(tween))
            .map_err(|_| anyhow!("Command queue full"))
    }
}

impl SoundData for AudioData {
    type Error = anyhow::Error;
    type Handle = AudioHandle;

    fn into_sound(self) -> Result<(Box<dyn Sound>, Self::Handle), Self::Error> {
        let (sound, handle) = self.split();
        Ok((Box::new(sound), handle))
    }
}

impl AudioData {
    fn split(self) -> (AudioSound, AudioHandle) {
        let (command_producer, command_consumer) = HeapRb::new(COMMAND_BUFFER_CAPACITY).split();

        debug!("Creating audio sound for track {:?}", self.1.track);

        let mut volume_fade = Tweener::new(0.0);
        volume_fade.enqueue_now(1.0, self.1.fade_in);

        let shared = Arc::new(Shared::new());
        let sound = AudioSound {
            track_id: self.1.track,
            command_consumer,
            shared: shared.clone(),
            state: PlaybackState::Playing,
            volume: Tweener::new(self.1.volume.clamp(0.0, 1.0)), // TODO: warn if clamped
            panning: Tweener::new(self.1.pan.clamp(-1.0, 1.0)),  // TODO: warn if clamped
            volume_fade,
            sample_provider: SampleProvider::new(self.0, self.1.repeat),
        };
        (
            sound,
            AudioHandle {
                command_producer,
                shared,
            },
        )
    }
}

/// The playback state of a sound.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlaybackState {
    /// The sound is playing normally.
    Playing,
    /// The sound is fading out, and when the fade-out
    /// is finished, playback will stop.
    Stopping,
    /// The sound has stopped and can no longer be resumed.
    Stopped,
}

struct SampleProvider {
    decoder: AudioDecoder<ArcAudio>,
    resampler: Resampler,
    buffer_offset: usize,
    fractional_position: f64,
    end_of_file: bool,
    repeat: bool,
}

impl SampleProvider {
    fn new(audio: ArcAudio, repeat: bool) -> Self {
        Self {
            decoder: AudioDecoder::new(audio).expect("Could not create audio decoder"),
            repeat,
            resampler: Resampler::new(0),
            buffer_offset: 0,
            fractional_position: 0.0,
            end_of_file: false,
        }
    }

    fn position(&self) -> i64 {
        // TODO: seeking???
        self.decoder.samples_position() + self.buffer_offset as i64
    }

    fn push_next_frame(&mut self) {
        let buffer = self.decoder.buffer();
        let buffer = &buffer[self.buffer_offset * self.decoder.info().channel_count as usize..];
        if !buffer.is_empty() {
            // TODO: handle non-stereo audio?
            self.buffer_offset += 1;

            let frame = match self.decoder.info().channel_count {
                1 => Frame {
                    left: buffer[0],
                    right: buffer[0],
                },
                2 => Frame {
                    left: buffer[0],
                    right: buffer[1],
                },
                _ => panic!("Unsupported channel count"),
            };

            self.resampler.push_frame(frame, self.position());
        } else {
            match self.decoder.decode_frame() {
                Some(pos) => self.buffer_offset = pos,
                None => {
                    // TODO: start outputting silence instead of just stopping?
                    self.end_of_file = true;
                    return;
                }
            }

            self.push_next_frame()
        }
    }

    fn next(&mut self, dt: f64) -> Option<Frame> {
        let out = self.resampler.get(self.fractional_position as f32);
        self.fractional_position += dt * self.decoder.info().sample_rate as f64;
        while self.fractional_position >= 1.0 {
            self.fractional_position -= 1.0;
            self.push_next_frame();
        }

        if self.end_of_file {
            if self.repeat {
                warn!("TODO: repeat audio (need to impl seeking)");
            }
            None
        } else {
            Some(out)
        }
    }
}

struct AudioSound {
    track_id: TrackId,
    command_consumer: HeapConsumer<Command>,
    shared: Arc<Shared>,
    state: PlaybackState,
    volume: Tweener,
    panning: Tweener,
    volume_fade: Tweener,
    sample_provider: SampleProvider,
}

impl AudioSound {
    fn stop(&mut self, fade_out_tween: Tween) {
        self.state = PlaybackState::Stopping;
        self.volume_fade.enqueue_now(0.0, fade_out_tween);
    }

    fn wait_status(&self) -> AudioWaitStatus {
        let mut result = AudioWaitStatus::empty();

        if self.state == PlaybackState::Stopped {
            result |= AudioWaitStatus::STOPPED;
        }
        if self.state == PlaybackState::Playing {
            result |= AudioWaitStatus::PLAYING;
        }
        if self.volume.is_idle() {
            result |= AudioWaitStatus::VOLUME_TWEENER_IDLE;
        }
        if self.panning.is_idle() {
            result |= AudioWaitStatus::PANNING_TWEENER_IDLE;
        }
        result |= AudioWaitStatus::PLAY_SPEED_TWEENER_IDLE;

        result
    }
}

impl Sound for AudioSound {
    fn track(&mut self) -> TrackId {
        self.track_id
    }

    fn on_start_processing(&mut self) {
        while let Some(command) = self.command_consumer.pop() {
            match command {
                // note: unlike in the layer props, we do the "enqueue_now" thing here
                // bacause we don't want to wait for previous audio changes to be applied
                // ideally, this should never allocate the tweener queue
                Command::SetVolume(volume, tween) => self.volume.enqueue_now(volume, tween),
                Command::SetPanning(panning, tween) => self.panning.enqueue_now(panning, tween),
                Command::Stop(tween) => self.stop(tween),
            }
        }

        self.shared
            .wait_status
            .store(self.wait_status().bits, std::sync::atomic::Ordering::SeqCst);
        // TODO: compute the amplitude
        // TODO: provide the position
    }

    fn process(&mut self, dt: f64, _clock_info_provider: &ClockInfoProvider) -> Frame {
        let dt_ticks = Ticks::from_seconds(dt as f32);

        // update tweeners
        self.volume.update(dt_ticks);
        self.panning.update(dt_ticks);
        self.volume_fade.update(dt_ticks);

        if self.state == PlaybackState::Stopping && self.volume_fade.is_idle() {
            self.state = PlaybackState::Stopped
        }

        match self.sample_provider.next(dt) {
            None => {
                // TODO loop around
                self.state = PlaybackState::Stopped;
                Frame::ZERO
            }
            Some(f) => {
                let f = f * self.volume_fade.value() as f32 * self.volume.value() as f32;
                let f = match self.panning.value() {
                    0.0 => f,
                    pan => Frame::new(f.left * (1.0 - pan).sqrt(), f.right * pan.sqrt()) * SQRT_2,
                };
                f
            }
        }
    }

    fn finished(&self) -> bool {
        self.state == PlaybackState::Stopped
    }
}
