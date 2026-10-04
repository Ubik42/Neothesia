use super::*;
pub(super) fn grid(file: &MidiFile) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for m in &file.musical_time.measures {
        for value in [
            m.start_tick,
            m.end_tick,
            u64::from(m.numerator),
            u64::from(m.denominator),
        ] {
            for b in value.to_le_bytes() {
                hash ^= u64::from(b);
                hash = hash.wrapping_mul(0x100000001b3);
            }
        }
    }
    format!("grid-v1-{hash:016x}")
}
pub(super) fn scope(
    file: Option<&MidiFile>,
    notes: &[PracticeNote],
    config: &SongConfig,
    assigned: &HashMap<(usize, usize), PracticePart>,
    wanted: &str,
) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    let mut feed = |value: u64| {
        for b in value.to_le_bytes() {
            hash ^= u64::from(b);
            hash = hash.wrapping_mul(0x100000001b3);
        }
    };
    if let Some(file) = file {
        for m in &file.musical_time.measures {
            feed(m.start_tick);
            feed(m.end_tick);
            feed(u64::from(m.numerator));
            feed(u64::from(m.denominator));
        }
    }
    for n in notes {
        let player = config
            .tracks
            .iter()
            .find(|t| t.track_id == n.track_id)
            .map_or(PlayerConfig::Mute, |t| t.player);
        let part = hands::part(config, assigned, n.track_id, n.index);
        if player == PlayerConfig::Human
            && match wanted {
                "left" => part == PracticePart::LeftHand,
                "right" => part == PracticePart::RightHand,
                _ => true,
            }
        {
            feed(n.track_id as u64);
            feed(n.index as u64);
            feed(match part {
                PracticePart::LeftHand => 1,
                PracticePart::RightHand => 2,
                _ => 0,
            });
        }
    }
    format!("practice-v1-{hash:016x}")
}
