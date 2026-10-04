use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::{
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering},
        mpsc::{self, SyncSender},
    },
};

pub struct Audio {
    _stream: cpal::Stream,
    tx: SyncSender<(u64,Vec<u8>)>,
    generation: Arc<AtomicU64>,
    panic: Arc<AtomicBool>,
    volume: Arc<AtomicU32>,
    click: Arc<AtomicU32>,
}

fn send(synth: &mut oxisynth::Synth, bytes: &[u8]) {
    use oxisynth::MidiEvent as E;
    let Some(&status) = bytes.first() else { return };
    let channel = status & 15;
    let a = bytes.get(1).copied().unwrap_or(0) & 127;
    let b = bytes.get(2).copied().unwrap_or(0) & 127;
    let event = match status & 240 {
        128 => E::NoteOff { channel, key: a },
        144 if b == 0 => E::NoteOff { channel, key: a },
        144 => E::NoteOn {
            channel,
            key: a,
            vel: b,
        },
        160 => E::PolyphonicKeyPressure {
            channel,
            key: a,
            value: b,
        },
        176 => E::ControlChange {
            channel,
            ctrl: a,
            value: b,
        },
        192 => E::ProgramChange {
            channel,
            program_id: a,
        },
        208 => E::ChannelPressure { channel, value: a },
        224 => E::PitchBend {
            channel,
            value: u16::from(a) | (u16::from(b) << 7),
        },
        _ => return,
    };
    let _ = synth.send_event(event);
}

impl Audio {
    pub fn open(path: &Path) -> Result<Self, String> {
        let device = cpal::default_host()
            .default_output_device()
            .ok_or("没有可用的音频输出设备")?;
        let supported = device.default_output_config().map_err(|e| e.to_string())?;
        let format = supported.sample_format();
        let config: cpal::StreamConfig = supported.into();
        let mut synth = oxisynth::Synth::new(oxisynth::SynthDescriptor {
            sample_rate: config.sample_rate as f32,
            gain: 0.35,
            ..Default::default()
        })
        .map_err(|e| e.to_string())?;
        let mut file = std::fs::File::open(path).map_err(|e| format!("无法读取钢琴音色：{e}"))?;
        let font =
            oxisynth::SoundFont::load(&mut file).map_err(|e| format!("钢琴音色文件无效：{e}"))?;
        synth.add_font(font, true);
        let (tx, rx) = mpsc::sync_channel::<(u64,Vec<u8>)>(2048);
        let panic = Arc::new(AtomicBool::new(false));
        let panic_callback = panic.clone();
        let volume = Arc::new(AtomicU32::new(1f32.to_bits()));
        let volume_callback = volume.clone();
        let click = Arc::new(AtomicU32::new(0));
        let click_callback = click.clone();
        let sample_rate = config.sample_rate as f32;
        let mut remaining = 0usize;
        let generation=Arc::new(AtomicU64::new(0));
        let generation_callback=generation.clone();
        let mut phase = 0f32;
        let mut frequency = 1000f32;
        let channels = config.channels as usize;
        let mut next = move || {
            if panic_callback.swap(false, Ordering::AcqRel) {
                remaining = 0;
                click_callback.store(0, Ordering::Relaxed);

                for channel in 0..16 {
                    send(&mut synth, &[176 | channel, 64, 0]);
                    let _ = synth.send_event(oxisynth::MidiEvent::AllSoundOff { channel });
                }
            }
            for _ in 0..64 {
                match rx.try_recv() {
                    Ok((g,b)) if g==generation_callback.load(Ordering::Acquire) => send(&mut synth, &b),
                    Ok(_) => {},
                    Err(_) => break,
                }
            }
            let request = click_callback.swap(0, Ordering::Relaxed);
            if request > 0 {
                remaining = (sample_rate * 0.04) as usize;
                phase = 0.;
                frequency = if request == 2 { 1500. } else { 1000. };
            }
            let tone = if remaining > 0 {
                remaining -= 1;
                phase += frequency / sample_rate * std::f32::consts::TAU;
                phase.sin() * 0.16 * (remaining as f32 / (sample_rate * 0.04))
            } else {
                0.
            };
            let (left, right) = synth.read_next();
            let (left, right) = (left + tone, right + tone);
            let gain = f32::from_bits(volume_callback.load(Ordering::Relaxed));
            (left * gain, right * gain)
        };
        macro_rules! stream {
            ($t:ty) => {
                device.build_output_stream(
                    config,
                    move |out: &mut [$t], _: &cpal::OutputCallbackInfo| {
                        for frame in out.chunks_mut(channels) {
                            let (l, r) = next();
                            for (i, sample) in frame.iter_mut().enumerate() {
                                *sample =
                                    <$t as cpal::FromSample<f32>>::from_sample_(if i % 2 == 0 {
                                        l
                                    } else {
                                        r
                                    });
                            }
                        }
                    },
                    |e| eprintln!("音频输出错误：{e}"),
                    None,
                )
            };
        }
        let stream = match format {
            cpal::SampleFormat::F32 => stream!(f32),
            cpal::SampleFormat::F64 => stream!(f64),
            cpal::SampleFormat::I16 => stream!(i16),
            cpal::SampleFormat::I32 => stream!(i32),
            cpal::SampleFormat::U16 => stream!(u16),
            _ => return Err(format!("暂不支持此音频格式：{format:?}")),
        }
        .map_err(|e| e.to_string())?;
        stream.play().map_err(|e| e.to_string())?;
        Ok(Self {
            _stream: stream,
            tx,
            generation,
            panic,
            volume,
            click,
        })
    }
    pub fn send(&self, bytes: &[u8]) {
        if self.tx.try_send((self.generation.load(Ordering::Acquire),bytes.to_vec())).is_err() {
            self.stop();
        }
    }
    pub fn stop(&self) {
        self.generation.fetch_add(1,Ordering::AcqRel);
        self.panic.store(true, Ordering::Release);
    }
    pub fn click(&self, accent: bool) {
        self.click
            .store(if accent { 2 } else { 1 }, Ordering::Relaxed);
    }
    pub fn set_volume(&self, value: f64) {
        self.volume
            .store((value as f32).to_bits(), Ordering::Relaxed);
    }
}

#[cfg(test)]mod tests {
 use super::*;
 #[test]fn track_volume_controls_rendered_sound_without_changing_velocity(){
  fn energy(volume:u8)->f64{let mut synth=oxisynth::Synth::new(Default::default()).unwrap();let path=std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../default.sf2");let font=oxisynth::SoundFont::load(&mut std::fs::File::open(path).unwrap()).unwrap();synth.add_font(font,true);send(&mut synth,&[193,0]);send(&mut synth,&[177,7,volume]);send(&mut synth,&[145,60,80]);(0..8192).map(|_|{let(l,r)=synth.read_next();f64::from(l*l+r*r)}).sum()}
  let full=energy(100);assert!(full>0.0001);assert!(energy(0)<full*0.001);
 }
}
