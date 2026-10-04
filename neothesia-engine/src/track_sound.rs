use crate::Player;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TrackSound {
    pub volume: u8,
    pub pan: Option<u8>,
    pub program: Option<u8>,
}
impl Default for TrackSound {
    fn default() -> Self {
        Self {
            volume: 100,
            pan: None,
            program: None,
        }
    }
}
#[derive(Default, Clone)]
pub struct Routing {
    pub lanes: BTreeMap<(usize, u8), u8>,
}
impl Routing {
    pub fn build(events: &[midi_file::MidiEvent]) -> Result<Self, String> {
        let mut keys = std::collections::BTreeSet::new();
        for e in events {
            if matches!(
                e.message,
                midi_file::midly::MidiMessage::NoteOn { .. }
                    | midi_file::midly::MidiMessage::NoteOff { .. }
            ) {
                keys.insert((e.track_id, e.channel));
            }
        }
        let mut available = (1..16).filter(|c| *c != 9);
        let mut percussion = false;
        let mut lanes = BTreeMap::new();
        for key in keys {
            let channel = if key.1 == 9 {
                if percussion {
                    return Err("多个独立打击乐声部暂不支持单独混音".into());
                }
                percussion = true;
                9
            } else {
                available
                    .next()
                    .ok_or("独立混音最多支持 14 个旋律通道和 1 个打击乐通道；此曲通道过多")?
            };
            lanes.insert(key, channel);
        }
        Ok(Self { lanes })
    }
    pub fn transform(
        &self,
        track: usize,
        bytes: &[u8],
        settings: &BTreeMap<usize, TrackSound>,
    ) -> Vec<Vec<u8>> {
        if bytes.is_empty() {
            return vec![];
        }
        let original = bytes[0] & 15;
        let kind = bytes[0] & 240;
        let direct = self.lanes.get(&(track, original)).copied();
        let targets: Vec<_> = if let Some(channel) = direct {
            vec![(track, channel)]
        } else if matches!(kind, 176 | 192 | 208 | 224) {
            self.lanes
                .iter()
                .filter(|((_, c), _)| *c == original)
                .map(|((t, _), c)| (*t, *c))
                .collect()
        } else {
            vec![]
        };
        targets
            .into_iter()
            .flat_map(|(t, c)| {
                let s = settings.get(&t).cloned().unwrap_or_default();
                let mut b = bytes.to_vec();
                b[0] = kind | c;
                if kind == 192 {
                    if let Some(program) = s.program {
                        b[1] = program;
                    }
                }
                if kind == 176 && b.len() > 2 {
                    match b[1] {
                        7 => b[2] = ((u16::from(b[2]) * u16::from(s.volume) + 50) / 100) as u8,
                        10 => {
                            if let Some(pan) = s.pan {
                                b[2] = pan
                            }
                        }
                        0 | 32 if s.program.is_some() => b[2] = 0,
                        _ => {}
                    }
                }
                let mut out = vec![b];
                if kind == 176 && bytes.get(1) == Some(&121) {
                    out.push(vec![176 | c, 7, s.volume]);
                    if let Some(p) = s.pan {
                        out.push(vec![176 | c, 10, p]);
                    }
                }
                out
            })
            .collect()
    }
}
#[derive(Serialize)]
pub struct Preset {
    pub program: u8,
    pub name: String,
}
pub fn presets(path: &std::path::Path) -> Vec<Preset> {
    fn chunks(bytes: &[u8]) -> Vec<Preset> {
        let mut out = vec![];
        let mut at = 0;
        while at + 8 <= bytes.len() {
            let size = u32::from_le_bytes(bytes[at + 4..at + 8].try_into().unwrap()) as usize;
            let Some(end) = (at + 8).checked_add(size).filter(|e| *e <= bytes.len()) else {
                break;
            };
            let id = &bytes[at..at + 4];
            let data = &bytes[at + 8..end];
            if id == b"LIST" && data.len() >= 4 && &data[..4] == b"pdta" {
                out.extend(chunks(&data[4..]));
            }
            if id == b"phdr" {
                for r in data.chunks_exact(38) {
                    let program = u16::from_le_bytes([r[20], r[21]]);
                    let bank = u16::from_le_bytes([r[22], r[23]]);
                    let name = String::from_utf8_lossy(&r[..20])
                        .trim_end_matches('\0')
                        .to_string();
                    if bank == 0 && program < 128 && name != "EOP" {
                        out.push(Preset {
                            program: program as u8,
                            name,
                        });
                    }
                }
            }
            at = end + (size % 2);
        }
        out
    }
    if std::fs::metadata(path).map_or(true, |m| m.len() > 256 * 1024 * 1024) {
        return vec![];
    }
    let Ok(bytes) = std::fs::read(path) else {
        return vec![];
    };
    if bytes.len() < 12 || &bytes[..4] != b"RIFF" || &bytes[8..12] != b"sfbk" {
        return vec![];
    }
    let mut out = chunks(&bytes[12..]);
    out.sort_by_key(|p| p.program);
    out.dedup_by_key(|p| p.program);
    out
}
impl Player {
    pub(crate) fn track_sound_settings(&self) -> BTreeMap<usize, TrackSound> {
        let mut s = self
            .file
            .as_ref()
            .and_then(|f| self.preferences.track_sounds.get(&f.content_id))
            .cloned()
            .unwrap_or_default();
        s.retain(|id, s| {
            s.volume <= 100
                && s.pan.is_none_or(|p| p <= 127)
                && s.program.is_none_or(|p| p <= 127)
                && self.file.as_ref().is_some_and(|f| {
                    f.tracks
                        .iter()
                        .any(|t| t.track_id == *id && !t.notes.is_empty())
                })
        });
        s
    }
    pub(crate) fn effective_track_sound(&self) -> BTreeMap<usize, TrackSound> {
        let mut s = self.track_sound_settings();
        if self.vst.is_some() || self.output_connection.is_some() {
            for sound in s.values_mut() {
                sound.program = None;
            }
        }
        s
    }
    pub(crate) fn send_track(&mut self, track: usize, bytes: &[u8]) {
        if self.track_routing.lanes.is_empty() {
            self.send(bytes)
        } else {
            for b in self
                .track_routing
                .transform(track, bytes, &self.effective_track_sound())
            {
                self.send(&b)
            }
        }
    }
    pub(crate) fn restore_track_sound(&mut self) {
        let settings = self.effective_track_sound();
        self.track_routing = if settings.is_empty() {
            Routing::default()
        } else {
            match Routing::build(&self.events) {
                Ok(r) => r,
                Err(e) => {
                    self.state.error = Some(e);
                    Routing::default()
                }
            }
        };
        let channels: Vec<u8> = if self.track_routing.lanes.is_empty() {
            (0..16).collect()
        } else {
            self.track_routing
                .lanes
                .values()
                .copied()
                .chain(std::iter::once(0))
                .collect()
        };
        for c in channels {
            for b in [
                vec![176 | c, 121, 0],
                vec![176 | c, 0, 0],
                vec![176 | c, 32, 0],
                vec![192 | c, 0],
                vec![176 | c, 7, 100],
                vec![176 | c, 10, 64],
                vec![224 | c, 0, 64],
            ] {
                self.send(&b)
            }
        }
        let lanes: Vec<_> = self
            .track_routing
            .lanes
            .iter()
            .map(|((t, _), c)| (*t, *c))
            .collect();
        for (t, c) in lanes {
            let s = settings.get(&t).cloned().unwrap_or_default();
            self.send(&[176 | c, 7, s.volume]);
            if let Some(p) = s.pan {
                self.send(&[176 | c, 10, p])
            }
            if let Some(p) = s.program {
                self.send(&[192 | c, p])
            }
        }
        let mut latest = BTreeMap::new();
        for (i, e) in self.events.iter().take(self.event_cursor).enumerate() {
            use midi_file::midly::MidiMessage as M;
            let key = match e.message {
                M::Controller { controller, .. }
                    if !matches!(controller.as_int(), 64 | 120 | 123) =>
                {
                    u16::from(controller.as_int())
                }
                M::ProgramChange { .. } => 128,
                M::PitchBend { .. } => 129,
                _ => continue,
            };
            latest.insert((e.track_id, e.channel, key), (i, e.clone()));
        }
        let mut events: Vec<_> = latest.into_values().collect();
        events.sort_by_key(|(i, _)| *i);
        let events = events.into_iter().map(|(_, e)| e);
        for e in events {
            use midi_file::midly::MidiMessage as M;
            let b = match e.message {
                M::Controller { controller, value }
                    if !matches!(controller.as_int(), 64 | 120 | 123) =>
                {
                    vec![176 | e.channel, controller.as_int(), value.as_int()]
                }
                M::ProgramChange { program } => vec![192 | e.channel, program.as_int()],
                M::PitchBend { bend } => vec![
                    224 | e.channel,
                    (bend.0.as_int() & 127) as u8,
                    (bend.0.as_int() >> 7) as u8,
                ],
                _ => continue,
            };
            self.send_track(e.track_id, &b);
        }
    }
    pub(crate) fn set_track_sound(
        &mut self,
        id: usize,
        sound: Option<TrackSound>,
    ) -> Result<serde_json::Value, String> {
        if self.routine.is_some() || self.ladder.is_some() {
            return Err("请先结束当前计划项目再调整音轨声音".into());
        }
        if !self.config.tracks.iter().any(|t| t.track_id == id) {
            return Err("找不到音轨".into());
        }
        if let Some(s) = &sound {
            if s.volume > 100 || s.pan.is_some_and(|p| p > 127) {
                return Err("音量或声像超出范围".into());
            }
            if s.program
                .is_some_and(|p| !presets(&self.soundfont).iter().any(|a| a.program == p))
            {
                return Err("当前钢琴音源不含此预设".into());
            }
            Routing::build(&self.events)?;
        }
        let cid = self.file.as_ref().ok_or("请先选择曲目")?.content_id.clone();
        let old = self.preferences.clone();
        let map = self
            .preferences
            .track_sounds
            .entry(cid.clone())
            .or_default();
        if let Some(s) = sound {
            map.insert(id, s);
        } else {
            map.remove(&id);
        }
        if map.is_empty() {
            self.preferences.track_sounds.remove(&cid);
        }
        if let Err(e) = self.preferences.save(&self.preferences_path) {
            self.preferences = old;
            return Err(e);
        }
        self.reset(self.state.position);
        Ok(serde_json::json!({"ok":true}))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn independent_channels_and_expression() {
        let r = Routing {
            lanes: BTreeMap::from([((1, 0), 1), ((2, 0), 2)]),
        };
        let s = BTreeMap::from([(
            1,
            TrackSound {
                volume: 50,
                pan: Some(10),
                program: Some(22),
            },
        )]);
        assert_eq!(r.transform(1, &[144, 60, 83], &s), vec![vec![145, 60, 83]]);
        assert_eq!(r.transform(1, &[176, 7, 80], &s), vec![vec![177, 7, 40]]);
        assert_eq!(r.transform(2, &[176, 7, 80], &s), vec![vec![178, 7, 80]]);
        assert_eq!(
            r.transform(0, &[192, 8], &s),
            vec![vec![193, 22], vec![194, 8]]
        );
        assert_eq!(r.transform(1, &[176, 10, 64], &s), vec![vec![177, 10, 10]]);
    }
}

#[cfg(test)]
mod flow_tests {
    use super::*;
    use crate::{Command, Player};
    #[test]
    fn capacity_persistence_and_portable_settings() {
        let events: Vec<_> = (0..15)
            .map(|t| midi_file::MidiEvent {
                track_id: t,
                track_color_id: 0,
                channel: 0,
                timestamp: std::time::Duration::ZERO,
                message: midi_file::midly::MidiMessage::NoteOn {
                    key: 60.into(),
                    vel: 80.into(),
                },
            })
            .collect();
        assert!(Routing::build(&events).is_err());
        assert_eq!(Routing::build(&events[..14]).unwrap().lanes.len(), 14);
        let data = std::env::temp_dir().join(format!(
            "neothesia-mix-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let font = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../default.sf2");
        let presets = presets(&font);
        assert!(!presets.is_empty());
        let mut p = Player::new(data.clone(), font.clone(), true);
        p.command(Command::Generate {
            spec: Default::default(),
        })
        .unwrap();
        let id = p
            .config
            .tracks
            .iter()
            .find(|t| {
                p.file
                    .as_ref()
                    .unwrap()
                    .tracks
                    .iter()
                    .any(|a| a.track_id == t.track_id && !a.notes.is_empty())
            })
            .unwrap()
            .track_id;
        let sound = TrackSound {
            volume: 45,
            pan: Some(32),
            program: Some(presets[0].program),
        };
        p.command(Command::TrackSound {
            id,
            sound: Some(sound.clone()),
        })
        .unwrap();
        assert!(!p.track_routing.lanes.is_empty());
        assert_eq!(p.track_sound_settings()[&id], sound);
        let package = p.export_piece_package(false).unwrap();
        let bytes: Vec<u8> = serde_json::from_value(package["bytes"].clone()).unwrap();
        let decoded = crate::piece_package::Package::decode(&bytes).unwrap();
        assert_eq!(decoded.manifest.track_sounds[&id], sound);
        let snapshot = p.practice_backup_snapshot(&Default::default()).unwrap();
        assert!(
            snapshot["trackSounds"]
                .as_object()
                .is_some_and(|m| !m.is_empty())
        );
        let mut reopened = Player::new(data, font, true);
        reopened.restore();
        assert_eq!(reopened.track_sound_settings()[&id], sound);
        assert_ne!(reopened.state.status, "playing");
        p.command(Command::TrackSound { id, sound: None }).unwrap();
        assert!(p.track_routing.lanes.is_empty());
    }
}
