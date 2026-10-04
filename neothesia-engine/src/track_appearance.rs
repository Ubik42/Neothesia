use crate::Player;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackAppearance {
    pub name: Option<String>,
    pub color: Option<String>,
}
impl TrackAppearance {
    pub fn validate(&self) -> Result<(), String> {
        if self.name.as_ref().is_some_and(|n| {
            n.trim().is_empty() || n.chars().count() > 80 || n.chars().any(char::is_control)
        }) {
            return Err("音轨名称须为 1–80 个字符，不能包含换行或控制符".into());
        }
        if self.color.as_ref().is_some_and(|c| {
            c.len() != 7
                || !c.starts_with('#')
                || !c.as_bytes()[1..].iter().all(u8::is_ascii_hexdigit)
        }) {
            return Err("音轨颜色无效".into());
        }
        Ok(())
    }
}
impl Player {
    pub(crate) fn track_appearances(&self) -> BTreeMap<usize, TrackAppearance> {
        let mut map = self
            .file
            .as_ref()
            .and_then(|f| self.preferences.track_appearances.get(&f.content_id))
            .cloned()
            .unwrap_or_default();
        map.retain(|id, a| {
            a.validate().is_ok()
                && self.file.as_ref().is_some_and(|f| {
                    f.tracks
                        .iter()
                        .any(|t| t.track_id == *id && !t.notes.is_empty())
                })
        });
        map
    }
    pub(crate) fn set_track_appearance(
        &mut self,
        id: usize,
        appearance: Option<TrackAppearance>,
    ) -> Result<serde_json::Value, String> {
        let file = self.file.as_ref().ok_or("请先选择曲目")?;
        if !file
            .tracks
            .iter()
            .any(|t| t.track_id == id && !t.notes.is_empty())
        {
            return Err("找不到音轨".into());
        }
        let cid = file.content_id.clone();
        let appearance = appearance.map(|mut a| {
            a.name = a.name.map(|n| n.trim().to_string());
            a.color = a.color.map(|c| c.to_ascii_lowercase());
            a
        });
        if let Some(a) = &appearance {
            a.validate()?;
        }
        let old = self.preferences.clone();
        let rows = self
            .preferences
            .track_appearances
            .entry(cid.clone())
            .or_default();
        if let Some(a) = appearance.filter(|a| a.name.is_some() || a.color.is_some()) {
            rows.insert(id, a);
        } else {
            rows.remove(&id);
        }
        if rows.is_empty() {
            self.preferences.track_appearances.remove(&cid);
        }
        if let Err(e) = self.preferences.save(&self.preferences_path) {
            self.preferences = old;
            return Err(e);
        }
        Ok(serde_json::json!({"ok":true}))
    }
    pub(crate) fn decorate_tracks(&self, v: &mut serde_json::Value) {
        let map = self.track_appearances();
        if let Some(tracks) = v["tracks"].as_array_mut() {
            for t in tracks {
                if let Some(a) = t["id"].as_u64().and_then(|id| map.get(&(id as usize))) {
                    if let Some(name) = &a.name {
                        t["name"] = serde_json::json!(name)
                    }
                    t["color"] = serde_json::json!(a.color);
                    t["appearanceCustom"] = serde_json::json!(true);
                } else {
                    t["color"] = serde_json::Value::Null;
                    t["appearanceCustom"] = serde_json::json!(false);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Command;
    #[test]
    fn source_names_appearance_persistence_and_no_transport_reset() {
        use midi_file::midly::{
            Format, Header, MetaMessage, MidiMessage, Smf, Timing, TrackEvent, TrackEventKind,
        };
        let smf = Smf {
            header: Header {
                format: Format::SingleTrack,
                timing: Timing::Metrical(480.into()),
            },
            tracks: vec![vec![
                TrackEvent {
                    delta: 0.into(),
                    kind: TrackEventKind::Meta(MetaMessage::TrackName("钢琴右手".as_bytes())),
                },
                TrackEvent {
                    delta: 0.into(),
                    kind: TrackEventKind::Midi {
                        channel: 0.into(),
                        message: MidiMessage::NoteOn {
                            key: 60.into(),
                            vel: 80.into(),
                        },
                    },
                },
                TrackEvent {
                    delta: 480.into(),
                    kind: TrackEventKind::Midi {
                        channel: 0.into(),
                        message: MidiMessage::NoteOff {
                            key: 60.into(),
                            vel: 0.into(),
                        },
                    },
                },
                TrackEvent {
                    delta: 0.into(),
                    kind: TrackEventKind::Meta(MetaMessage::EndOfTrack),
                },
            ]],
        };
        let file = midi_file::MidiFile::from_smf("test", &smf).unwrap();
        assert_eq!(file.tracks[0].name.as_deref(), Some("钢琴右手"));
        let data = std::env::temp_dir().join(format!(
            "neothesia-appearance-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let mut p = Player::new(data.clone(), std::path::PathBuf::new(), true);
        p.command(Command::Generate {
            spec: Default::default(),
        })
        .unwrap();
        let id = p
            .file
            .as_ref()
            .unwrap()
            .tracks
            .iter()
            .find(|t| !t.notes.is_empty())
            .unwrap()
            .track_id;
        p.state.status = "playing".into();
        p.state.position = 0.25;
        let a = TrackAppearance {
            name: Some("旋律练习".into()),
            color: Some("#f1ad67".into()),
        };
        p.command(Command::TrackAppearance {
            id,
            appearance: Some(a.clone()),
        })
        .unwrap();
        assert_eq!(p.state.status, "playing");
        assert_eq!(p.state.position, 0.25);
        let song = p.command(Command::CurrentSong).unwrap();
        let t = song["tracks"]
            .as_array()
            .unwrap()
            .iter()
            .find(|t| t["id"] == id)
            .unwrap();
        assert_eq!(t["name"], "旋律练习");
        assert_eq!(t["color"], "#f1ad67");
        assert!(t["sourceName"].as_str().is_some());
        let mut reopened = Player::new(data, std::path::PathBuf::new(), true);
        reopened.restore();
        assert_eq!(reopened.track_appearances()[&id], a);
        p.state.status = "paused".into();
        let package = p.export_piece_package(false).unwrap();
        let bytes: Vec<u8> = serde_json::from_value(package["bytes"].clone()).unwrap();
        let decoded = crate::piece_package::Package::decode(&bytes).unwrap();
        assert_eq!(decoded.manifest.track_appearances[&id], a);
        let before = p.track_appearances();
        assert!(
            p.command(Command::TrackAppearance {
                id,
                appearance: Some(TrackAppearance {
                    name: Some("bad\nname".into()),
                    color: None
                })
            })
            .is_err()
        );
        assert!(
            p.command(Command::TrackAppearance {
                id,
                appearance: Some(TrackAppearance {
                    name: None,
                    color: Some("red".into())
                })
            })
            .is_err()
        );
        assert_eq!(p.track_appearances(), before);
        p.command(Command::TrackSound {
            id,
            sound: Some(crate::track_sound::TrackSound {
                volume: 35,
                pan: None,
                program: None,
            }),
        })
        .unwrap();
        let backup = p.practice_backup_snapshot(&Default::default()).unwrap();
        let import:Command=serde_json::from_value(serde_json::json!({"type":"applyPracticeBackup","backup":backup,"selection":{"plans":false,"records":false,"presets":true,"days":0},"policy":"backup"})).unwrap();
        p.command(Command::TrackAppearance {
            id,
            appearance: None,
        })
        .unwrap();
        assert!(p.track_appearances().is_empty());
        p.command(import).unwrap();
        assert_eq!(p.track_appearances()[&id], a);
        assert_eq!(p.track_sound_settings()[&id].volume, 35);
    }
}
