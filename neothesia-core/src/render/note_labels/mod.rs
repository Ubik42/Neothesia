use std::{
    collections::{HashMap, HashSet},
    time::Duration,
};

use crate::utils::Point;

use super::{KeyboardRenderer, TextRenderer, waterfall::NoteList};

type FingeringKey = (Duration, u8, u8, usize);
type FingeringSelection = (FingeringKey, Option<u8>);

#[derive(Default)]
struct LabelsCache {
    labels: Option<[glyphon::Buffer; 12]>,
    fingers: Option<[glyphon::Buffer; 12]>,
    note_name_width: f32,
    finger_neutral_width: f32,
    finger_sharp_width: f32,
}

impl LabelsCache {
    #[profiling::function]
    fn note_names(&mut self, keyboard: &KeyboardRenderer) -> &[glyphon::Buffer; 12] {
        let font_system = crate::font_system::font_system();
        let font_system = &mut font_system.borrow_mut();

        let sharp_width = keyboard.layout().sizing.sharp_width;
        let neutral_width = keyboard.layout().sizing.neutral_width;

        if self.labels.is_none() || self.note_name_width != neutral_width {
            let label_width = sharp_width;

            let labels = [
                ("C", neutral_width),
                ("C#", sharp_width),
                ("D", neutral_width),
                ("D#", sharp_width),
                ("E", neutral_width),
                ("F", neutral_width),
                ("F#", sharp_width),
                ("G", neutral_width),
                ("G#", sharp_width),
                ("A", neutral_width),
                ("A#", sharp_width),
                ("B", neutral_width),
            ]
            .map(|(label, note_width)| {
                let mut buffer = glyphon::Buffer::new(
                    font_system,
                    glyphon::Metrics::new(label_width, label_width),
                );
                buffer.set_size(Some(note_width), None);
                buffer.set_wrap(glyphon::Wrap::None);
                buffer.set_text(
                    label,
                    &glyphon::Attrs::new().family(glyphon::Family::SansSerif),
                    glyphon::Shaping::Basic,
                    Some(glyphon::cosmic_text::Align::Center),
                );
                buffer.shape_until_scroll(font_system, false);
                buffer
            });

            self.labels = Some(labels);
            self.note_name_width = neutral_width;
        }

        self.labels.as_ref().unwrap()
    }

    #[profiling::function]
    fn finger_numbers(&mut self, keyboard: &KeyboardRenderer) -> &[glyphon::Buffer; 12] {
        let font_system = crate::font_system::font_system();
        let font_system = &mut font_system.borrow_mut();
        let sharp_width = keyboard.layout().sizing.sharp_width;
        let neutral_width = keyboard.layout().sizing.neutral_width;
        if self.fingers.is_none()
            || self.finger_neutral_width != neutral_width
            || self.finger_sharp_width != sharp_width
        {
            self.fingers = Some(std::array::from_fn(|index| {
                let label = if index < 10 {
                    (index / 2 + 1).to_string()
                } else {
                    "•".to_owned()
                };
                let note_width = if index % 2 == 0 {
                    neutral_width
                } else {
                    sharp_width
                };
                let mut buffer = glyphon::Buffer::new(
                    font_system,
                    glyphon::Metrics::new(sharp_width, sharp_width),
                );
                buffer.set_size(Some(note_width), None);
                buffer.set_wrap(glyphon::Wrap::None);
                buffer.set_text(
                    &label,
                    &glyphon::Attrs::new().family(glyphon::Family::SansSerif),
                    glyphon::Shaping::Basic,
                    Some(glyphon::cosmic_text::Align::Center),
                );
                buffer.shape_until_scroll(font_system, false);
                buffer
            }));
            self.finger_neutral_width = neutral_width;
            self.finger_sharp_width = sharp_width;
        }
        self.fingers.as_ref().unwrap()
    }
}

pub struct NoteLabels {
    pos: Point<f32>,
    notes: NoteList,
    labels_cache: LabelsCache,
    text_renderer: TextRenderer,
    note_names_enabled: bool,
    fingerings: HashMap<FingeringKey, u8>,
    fingering_crossings: HashSet<FingeringKey>,
    fingerings_enabled: bool,
    fingering_selection: Option<FingeringSelection>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum FingeringGlyph {
    Finger(u8),
    Marker,
}

fn fingering_glyph(
    stored_finger: Option<u8>,
    selected: bool,
    preview_finger: Option<u8>,
) -> Option<FingeringGlyph> {
    preview_finger
        .filter(|finger| (1..=5).contains(finger))
        .or(stored_finger.filter(|finger| (1..=5).contains(finger)))
        .map(FingeringGlyph::Finger)
        .or_else(|| selected.then_some(FingeringGlyph::Marker))
}

impl NoteLabels {
    pub fn new(pos: Point<f32>, notes: &NoteList, text_renderer: TextRenderer) -> Self {
        Self::with_fingerings(pos, notes, text_renderer, true, HashMap::new(), false)
    }

    pub fn with_fingerings(
        pos: Point<f32>,
        notes: &NoteList,
        text_renderer: TextRenderer,
        note_names_enabled: bool,
        fingerings: HashMap<FingeringKey, u8>,
        fingerings_enabled: bool,
    ) -> Self {
        Self::with_fingering_guidance(
            pos,
            notes,
            text_renderer,
            note_names_enabled,
            fingerings,
            HashSet::new(),
            fingerings_enabled,
        )
    }

    pub fn with_fingering_guidance(
        pos: Point<f32>,
        notes: &NoteList,
        text_renderer: TextRenderer,
        note_names_enabled: bool,
        fingerings: HashMap<FingeringKey, u8>,
        fingering_crossings: HashSet<FingeringKey>,
        fingerings_enabled: bool,
    ) -> Self {
        let fingerings_enabled = !fingerings.is_empty() && fingerings_enabled;
        Self {
            pos,
            notes: notes.clone(),
            labels_cache: LabelsCache::default(),
            text_renderer,
            note_names_enabled,
            fingerings,
            fingering_crossings,
            fingerings_enabled,
            fingering_selection: None,
        }
    }

    pub fn has_fingerings(&self) -> bool {
        !self.fingerings.is_empty()
    }

    pub fn fingerings_enabled(&self) -> bool {
        self.has_fingerings() && self.fingerings_enabled
    }

    pub fn fingering_crossing_count(&self) -> usize {
        self.fingering_crossings.len()
    }

    pub fn toggle_fingerings(&mut self) -> bool {
        if !self.has_fingerings() {
            return false;
        }
        self.fingerings_enabled = !self.fingerings_enabled;
        true
    }

    pub fn set_fingering(&mut self, key: FingeringKey, finger: Option<u8>) {
        if let Some(finger) = finger.filter(|finger| (1..=5).contains(finger)) {
            self.fingerings.insert(key, finger);
            self.fingerings_enabled = true;
        } else {
            self.fingerings.remove(&key);
            self.fingering_crossings.remove(&key);
            if self.fingerings.is_empty() {
                self.fingerings_enabled = false;
            }
        }
    }

    pub fn set_fingering_selection(&mut self, key: FingeringKey, preview_finger: Option<u8>) {
        self.fingering_selection = Some((
            key,
            preview_finger.filter(|finger| (1..=5).contains(finger)),
        ));
        self.fingerings_enabled = true;
    }

    pub fn clear_fingering_selection(&mut self) {
        self.fingering_selection = None;
        if self.fingerings.is_empty() {
            self.fingerings_enabled = false;
        }
    }

    pub fn set_pos(&mut self, pos: Point<f32>) {
        self.pos = pos;
    }

    #[profiling::function]
    pub fn update(
        &mut self,
        physical_size: dpi::PhysicalSize<u32>,
        scale: f32,
        keyboard: &KeyboardRenderer,
        animation_speed: f32,
        time: f32,
    ) {
        let layout = keyboard.layout();
        let range_start = layout.range.start() as usize;
        let label_width = layout.sizing.sharp_width;

        let animation_speed = animation_speed / scale;

        if self.fingerings_enabled {
            let labels = self.labels_cache.finger_numbers(keyboard);
            let selection = self.fingering_selection;
            let iter = self
                .notes
                .inner
                .iter()
                .filter(|note| layout.range.contains(note.note) && note.channel != 9)
                .filter_map(|note| {
                    let note_key = (note.start, note.note, note.channel, note.track_id);
                    let selected = selection.is_some_and(|(key, _)| key == note_key);
                    let preview = selection
                        .filter(|(key, _)| *key == note_key)
                        .and_then(|(_, finger)| finger);
                    let glyph = fingering_glyph(
                        self.fingerings.get(&note_key).copied(),
                        selected,
                        preview,
                    )?;
                    let crossing = self.fingering_crossings.contains(&(
                        note.start,
                        note.note,
                        note.channel,
                        note.track_id,
                    ));
                    let key = &layout.keys[note.note as usize - range_start];
                    let kind_index = usize::from(key.kind().is_sharp());
                    let buffer = if let FingeringGlyph::Finger(finger) = glyph {
                        let finger_index = usize::from(finger.saturating_sub(1).min(4));
                        &labels[finger_index * 2 + kind_index]
                    } else {
                        &labels[10 + kind_index]
                    };
                    let x = key.x();
                    let y = self.pos.y
                        - (note.start.as_secs_f32() - time) * animation_speed
                        - label_width;
                    Some((buffer, x, y, crossing, selected))
                })
                .take_while(|(_buffer, _x, y, _crossing, _selected)| *y > 0.0)
                .skip_while(|(_buffer, _x, y, _crossing, _selected)| *y > keyboard.pos().y)
                .map(|(buffer, x, y, crossing, selected)| {
                    text_area((
                        buffer,
                        x,
                        y,
                        if selected {
                            glyphon::Color::rgb(80, 220, 255)
                        } else if crossing {
                            glyphon::Color::rgb(255, 196, 64)
                        } else {
                            glyphon::Color::rgb(255, 255, 255)
                        },
                    ))
                });
            self.text_renderer
                .update_from_iter(physical_size, scale, iter);
        } else if self.note_names_enabled {
            let labels = self.labels_cache.note_names(keyboard);
            let iter = self
                .notes
                .inner
                .iter()
                .filter(|note| layout.range.contains(note.note) && note.channel != 9)
                .map(|note| {
                    let buffer = &labels[(note.note % 12) as usize];
                    let x = layout.keys[note.note as usize - range_start].x();
                    let y = self.pos.y
                        - (note.start.as_secs_f32() - time) * animation_speed
                        - label_width;
                    (buffer, x, y, glyphon::Color::rgb(255, 255, 255))
                })
                .take_while(|(_buffer, _x, y, _color)| *y > 0.0)
                .skip_while(|(_buffer, _x, y, _color)| *y > keyboard.pos().y)
                .map(text_area);
            self.text_renderer
                .update_from_iter(physical_size, scale, iter);
        } else {
            self.text_renderer.update_from_iter(
                physical_size,
                scale,
                std::iter::empty::<glyphon::TextArea<'_>>(),
            );
        }
    }

    pub fn render<'rpass>(&'rpass mut self, render_pass: &mut wgpu_jumpstart::RenderPass<'rpass>) {
        self.text_renderer.render(render_pass);
    }
}

fn text_area(
    (buffer, left, top, color): (&glyphon::Buffer, f32, f32, glyphon::Color),
) -> glyphon::TextArea<'_> {
    glyphon::TextArea {
        buffer,
        left,
        top,
        scale: 1.0,
        bounds: glyphon::TextBounds {
            left: 0,
            top: 0,
            right: i32::MAX,
            bottom: i32::MAX,
        },
        default_color: color,
        custom_glyphs: &[],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selection_marker_and_preview_have_unambiguous_precedence() {
        assert_eq!(fingering_glyph(None, false, None), None);
        assert_eq!(
            fingering_glyph(None, true, None),
            Some(FingeringGlyph::Marker)
        );
        assert_eq!(
            fingering_glyph(Some(2), true, None),
            Some(FingeringGlyph::Finger(2))
        );
        assert_eq!(
            fingering_glyph(Some(2), true, Some(4)),
            Some(FingeringGlyph::Finger(4))
        );
        assert_eq!(
            fingering_glyph(Some(3), false, None),
            Some(FingeringGlyph::Finger(3))
        );
    }
}
