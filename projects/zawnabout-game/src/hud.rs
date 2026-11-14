//! The heads-up display drawn by the game itself: the journal (see `book`) everywhere,
//! and where the host has no page chrome of its own, a minimap in the corner, the big
//! map on M, and notes mode on N with its list of notes and the note being written.
//! The web page has its own map and notes and leaves those off.

use bytemuck::{Pod, Zeroable};
use glam::Vec2;

use crate::input::{Action, Bindings};
use crate::notes::Note;

/// The atlas the HUD draws from: the world map in the top `MAP` square, a white cell
/// under it for plain colored shapes, the font from `FONT_TOP`, then postcard pictures
/// from `CARDS_TOP`, `CARDS_ROW` to a row, then treasure pictures from `THUMBS_TOP`.
pub const ATLAS: (u32, u32) = (1024, PORTRAIT_TOP + PORTRAIT);
pub const MAP: u32 = 1024;
const CELL: u32 = 8;
pub const FONT_TOP: u32 = MAP + CELL;
pub const CARDS_TOP: u32 = 1640;
pub const CARDS_ROW: u32 = 6;
/// A postcard picture's cell, with a little room round it.
pub const CARD_CELL: (u32, u32) = (164, 94);
/// Rows of postcard pictures.
pub const CARD_ROWS: u32 = 16;
pub const THUMBS_TOP: u32 = CARDS_TOP + CARD_ROWS * CARD_CELL.1;
/// The player's dragon's portrait for the journal, square, at the left under the
/// treasure pictures.
pub const PORTRAIT: u32 = 256;
pub const PORTRAIT_TOP: u32 = THUMBS_TOP + 3 * crate::thumbs::THUMB;

/// Where treasure kind `i`'s picture sits in the atlas: its top left corner.
pub fn thumb_slot(i: usize) -> (u32, u32) {
    let (i, t) = (i as u32, crate::thumbs::THUMB);
    let row = ATLAS.0 / t;
    ((i % row) * t, THUMBS_TOP + (i / row) * t)
}

/// Draws every treasure's picture into the atlas.
pub fn thumbs_into(atlas: &mut [u8]) {
    let (w, t) = (ATLAS.0 as usize, crate::thumbs::THUMB as usize);
    for (i, kind) in crate::treasure::KINDS.iter().enumerate() {
        let px = crate::thumbs::draw(&crate::treasure::treasure(kind));
        let (x0, y0) = thumb_slot(i);
        for y in 0..t {
            let k = ((y0 as usize + y) * w + x0 as usize) * 4;
            atlas[k..k + t * 4].copy_from_slice(&px[y * t * 4..(y + 1) * t * 4]);
        }
    }
}

/// Where postcard `i`'s picture sits in the atlas: its top left corner.
pub fn card_slot(i: usize) -> (u32, u32) {
    let i = i as u32;
    (
        (i % CARDS_ROW) * CARD_CELL.0 + 2,
        CARDS_TOP + (i / CARDS_ROW) * CARD_CELL.1 + 2,
    )
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct HudVertex {
    /// Clip space position.
    pub pos: [f32; 2],
    pub uv: [f32; 2],
    /// Linear color with alpha, multiplied with the atlas.
    pub color: [f32; 4],
}

/// Draws the white cell and the font into the atlas under the map.
pub fn font_into(atlas: &mut [u8]) {
    let w = ATLAS.0 as usize;
    for y in 0..CELL as usize {
        for x in 0..CELL as usize {
            let k = ((MAP as usize + y) * w + x) * 4;
            atlas[k..k + 4].copy_from_slice(&[255; 4]);
        }
    }
    let f = crate::font::get();
    let base = f.top as usize * w;
    for (i, a) in f.rows.iter().enumerate() {
        atlas[(base + i) * 4..(base + i) * 4 + 4].copy_from_slice(&[255, 255, 255, *a]);
    }
}

/// sRGB hex to linear color with alpha.
pub(crate) fn rgba(hex: u32, a: f32) -> [f32; 4] {
    let ch = |s: u32| ((hex >> s) & 255) as f32 / 255.0;
    let lin = |v: f32| v.powf(2.2);
    [lin(ch(16)), lin(ch(8)), lin(ch(0)), a]
}

/// Builds the HUD's triangles in pixels, top left origin, turned to clip space at the end.
pub(crate) struct Draw {
    pub out: Vec<HudVertex>,
    pub size: Vec2,
}

impl Draw {
    pub(crate) fn vert(&mut self, p: Vec2, uv: Vec2, color: [f32; 4]) {
        self.out.push(HudVertex {
            pos: [p.x / self.size.x * 2.0 - 1.0, 1.0 - p.y / self.size.y * 2.0],
            uv: [uv.x / ATLAS.0 as f32, uv.y / ATLAS.1 as f32],
            color,
        });
    }

    pub(crate) fn quad(&mut self, p0: Vec2, p1: Vec2, uv0: Vec2, uv1: Vec2, color: [f32; 4]) {
        let (a, b, c, d) = (
            (p0, uv0),
            (Vec2::new(p1.x, p0.y), Vec2::new(uv1.x, uv0.y)),
            (p1, uv1),
            (Vec2::new(p0.x, p1.y), Vec2::new(uv0.x, uv1.y)),
        );
        for (p, uv) in [a, b, c, a, c, d] {
            self.vert(p, uv, color);
        }
    }

    /// A plain colored box.
    pub(crate) fn rect(&mut self, p0: Vec2, p1: Vec2, color: [f32; 4]) {
        let white = Vec2::new(4.0, MAP as f32 + 4.0);
        self.quad(p0, p1, white, white, color);
    }

    pub(crate) fn tri(&mut self, a: Vec2, b: Vec2, c: Vec2, color: [f32; 4]) {
        let white = Vec2::new(4.0, MAP as f32 + 4.0);
        for p in [a, b, c] {
            self.vert(p, white, color);
        }
    }

    /// Text with its top at `at`, `scale` tenths of a pixel to the em. Returns its width.
    pub(crate) fn text(&mut self, at: Vec2, scale: f32, s: &str, color: [f32; 4]) -> f32 {
        self.write(at, scale * 10.0, s, color)
    }

    /// Text with its top at `at`, `px` to the em. Returns its width.
    pub(crate) fn write(&mut self, at: Vec2, px: f32, s: &str, color: [f32; 4]) -> f32 {
        let (face, k) = crate::font::get().face(px);
        let base = at.y + px * 0.9;
        let mut x = at.x;
        for c in s.chars() {
            let g = face.glyph(c);
            if g.size.0 > 0.0 {
                let p0 = Vec2::new(x + g.offset.0 * k, base + g.offset.1 * k);
                let uv0 = Vec2::new(g.uv.0, g.uv.1);
                let size = Vec2::new(g.size.0, g.size.1);
                self.quad(p0, p0 + size * k, uv0, uv0 + size, color);
            }
            x += g.advance * k;
        }
        x - at.x
    }

    /// An arrow pointing along `dir` (screen, y down), for where the dragon heads.
    pub(crate) fn arrow(&mut self, at: Vec2, dir: Vec2, size: f32) {
        let side = Vec2::new(-dir.y, dir.x);
        let (tip, back) = (at + dir * size, at - dir * size * 0.7);
        let (l, r) = (back + side * size * 0.7, back - side * size * 0.7);
        let notch = at - dir * size * 0.25;
        let dark = rgba(0x2A1E14, 1.0);
        let grow = |p: Vec2| at + (p - at) * 1.35;
        self.tri(grow(tip), grow(l), grow(notch), dark);
        self.tri(grow(tip), grow(notch), grow(r), dark);
        let red = rgba(0xE0564A, 1.0);
        self.tri(tip, l, notch, red);
        self.tri(tip, notch, r, red);
    }
}

/// What the HUD shows and is in the middle of.
#[derive(Clone, Debug, Default)]
pub struct Hud {
    /// The map, minimap and notes are drawn. The web page has its own and leaves this off.
    pub on: bool,
    pub map: bool,
    /// The journal, drawn whether or not `on` is.
    pub book: crate::book::Book,
    /// A line shown at the bottom, who or what said it, and how many seconds it stays.
    pub toast: Option<(String, String, f32)>,
    /// What the passenger just said, in a bubble beside them, and how long it stays.
    pub say: Option<(String, f32)>,
    /// Where the passenger's head is on screen, in pixels, while one rides.
    pub rider_at: Option<Vec2>,
    /// Something the passenger points at, on screen in pixels, and whether it is in view.
    pub point: Option<(Vec2, bool)>,
    /// Where what the dragon carries is going: on screen in pixels, whether it is in
    /// view, and its name with how far it is. Shown the whole way there.
    pub goal: Option<(Vec2, bool, String)>,
    /// Notes mode's list, oldest first, and where the host keeps them.
    pub notes: Vec<Note>,
    pub notes_where: String,
    /// The picked note, counted from the newest.
    note_pick: usize,
    /// Delete was pressed once, and a second press deletes.
    deleting: bool,
    /// The note being written: its spot, view and text so far.
    pub writing: Option<Note>,
    /// Notes to save or delete, for the host to take.
    pub outbox: Vec<NoteChange>,
}

/// A change to the notes the host keeps.
#[derive(Clone, Debug, PartialEq)]
pub enum NoteChange {
    /// A new note, with no id yet.
    Save(Note),
    /// Removes the note with this id.
    Delete(String),
}

/// What a key did in notes mode.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum NotesKey {
    /// Not the HUD's: it moves the free camera.
    Pass,
    Used,
    /// Leave notes mode.
    Leave,
    /// Fly the free camera to this view: x, y, z, yaw, pitch.
    Go([f32; 5]),
}

/// Text broken into lines of at most `cols` letters, at spaces where it can.
fn wrap(s: &str, cols: usize) -> Vec<String> {
    let cols = cols.max(4);
    let mut out = Vec::new();
    for para in s.split('\n') {
        let mut line = String::new();
        for word in para.split(' ') {
            if !line.is_empty() && line.chars().count() + 1 + word.chars().count() > cols {
                out.push(std::mem::take(&mut line));
            } else if !line.is_empty() {
                line.push(' ');
            }
            line.push_str(word);
            while line.chars().count() > cols {
                let rest: String = line.chars().skip(cols).collect();
                line = line.chars().take(cols).collect();
                out.push(std::mem::replace(&mut line, rest));
            }
        }
        out.push(line);
    }
    out
}

/// What a key did in the HUD.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HudKey {
    /// Not the HUD's: the game takes it.
    Pass,
    /// The HUD used it.
    Used,
}

/// A key's name as people know it.
pub fn key_label(code: &str) -> String {
    let s = code
        .strip_prefix("Key")
        .or_else(|| code.strip_prefix("Digit"))
        .unwrap_or(code);
    match s {
        "ArrowUp" => "Up arrow".into(),
        "ArrowDown" => "Down arrow".into(),
        "ArrowLeft" => "Left arrow".into(),
        "ArrowRight" => "Right arrow".into(),
        "ShiftLeft" | "ShiftRight" => "Shift".into(),
        "ControlLeft" | "ControlRight" => "Ctrl".into(),
        "AltLeft" | "AltRight" => "Alt".into(),
        _ if s.chars().count() == 1 => s.to_uppercase(),
        _ => s.to_string(),
    }
}

impl Hud {
    /// Whether the big map covers the game, which pauses it.
    pub fn open(&self) -> bool {
        self.map
    }

    /// Replaces the listed notes, keeping the same one picked where it can.
    pub fn set_notes(&mut self, notes: Vec<Note>) {
        let picked = self.picked().map(|n| n.id.clone());
        self.notes = notes;
        self.note_pick = picked
            .and_then(|id| self.notes.iter().rev().position(|n| n.id == id))
            .unwrap_or(0);
        self.deleting = false;
    }

    fn picked(&self) -> Option<&Note> {
        self.notes.iter().rev().nth(self.note_pick)
    }

    /// Offers a key press, and the text it types, to notes mode.
    pub fn notes_key(&mut self, code: &str, text: Option<&str>) -> NotesKey {
        if let Some(note) = &mut self.writing {
            match code {
                "Escape" => self.writing = None,
                "Enter" | "NumpadEnter" => {
                    let mut note = self.writing.take().expect("writing");
                    note.text = note.text.trim().to_string();
                    if !note.text.is_empty() {
                        self.outbox.push(NoteChange::Save(note.clone()));
                        self.notes.push(note);
                        self.note_pick = 0;
                    }
                }
                "Backspace" => {
                    note.text.pop();
                }
                _ => {
                    let typed = text.unwrap_or("").chars().filter(|c| !c.is_control());
                    note.text.extend(typed);
                }
            }
            return NotesKey::Used;
        }
        let deleting = std::mem::take(&mut self.deleting);
        let count = self.notes.len().max(1);
        match code {
            "KeyN" | "Escape" => return NotesKey::Leave,
            "BracketLeft" | "PageUp" => self.note_pick = (self.note_pick + count - 1) % count,
            "BracketRight" | "PageDown" => self.note_pick = (self.note_pick + 1) % count,
            "KeyG" | "Enter" | "NumpadEnter" => {
                if let Some(n) = self.picked() {
                    return NotesKey::Go(n.view);
                }
            }
            "Delete" => {
                if !deleting {
                    self.deleting = self.picked().is_some();
                } else if let Some(n) = self.picked() {
                    let id = n.id.clone();
                    self.notes.retain(|n| n.id != id);
                    self.outbox.push(NoteChange::Delete(id));
                    self.note_pick = self.note_pick.min(self.notes.len().saturating_sub(1));
                }
            }
            _ => return NotesKey::Pass,
        }
        NotesKey::Used
    }

    /// Offers a key press to the map.
    pub fn key(&mut self, code: &str) -> HudKey {
        if !self.on {
            return HudKey::Pass;
        }
        match code {
            "KeyM" => self.map = !self.map,
            "Escape" if self.map => self.map = false,
            _ if self.map => {}
            _ => return HudKey::Pass,
        }
        HudKey::Used
    }

    /// The HUD's triangles for a `size` pixel screen. `at` and `heading` place the
    /// dragon, `bounds` is the world the map covers, and `map_ready` says whether the
    /// map is in the atlas yet.
    pub fn build(
        &self,
        size: (u32, u32),
        at: Vec2,
        heading: f32,
        bounds: [f32; 4],
        map_ready: bool,
        notes: bool,
        view: &crate::book::View,
    ) -> Vec<HudVertex> {
        let mut d = Draw {
            out: Vec::new(),
            size: Vec2::new(size.0 as f32, size.1 as f32),
        };
        if !self.on {
            if !notes {
                if !self.book.showing() {
                    self.rider_onto(&mut d);
                }
                self.toast_onto(&mut d);
                self.book.draw(&mut d, view);
            }
            return d.out;
        }
        let (w, h) = (d.size.x, d.size.y);
        let ui = (h / 360.0).clamp(1.0, 3.0).round();
        let span = Vec2::new(bounds[2] - bounds[0], bounds[3] - bounds[1]);
        let to_map = |p: Vec2| (p - Vec2::new(bounds[0], bounds[1])) / span * MAP as f32;
        let dir = Vec2::new(heading.sin(), heading.cos());
        let white = rgba(0xFFFFFF, 1.0);
        if notes {
            self.notes_panel(&mut d, ui);
            return d.out;
        }
        if map_ready && !self.map && !self.book.showing() {
            // The minimap: 2.4 km round the dragon, north up.
            let side = (h * 0.3).clamp(120.0, 300.0);
            let p0 = Vec2::new(w - side - 16.0, h - side - 16.0);
            d.rect(p0 - 4.0, p0 + side + 4.0, rgba(0xFFF6E6, 0.95));
            let view = 2400.0 / span.x * MAP as f32;
            let c = to_map(at);
            d.quad(
                p0,
                p0 + side,
                c - view * 0.5,
                c + view * 0.5,
                rgba(0xFFFFFF, 1.0),
            );
            d.arrow(p0 + side * 0.5, dir, 7.0 * ui);
            d.text(
                Vec2::new(p0.x + side * 0.5 - 3.0 * ui, p0.y + 4.0),
                ui,
                "N",
                white,
            );
        }
        if self.map && map_ready {
            let side = (w.min(h) - 60.0).max(100.0);
            let p0 = Vec2::new((w - side) * 0.5, (h - side) * 0.5 + 10.0);
            d.rect(Vec2::ZERO, d.size, rgba(0x14202C, 0.85));
            d.rect(p0 - 4.0, p0 + side + 4.0, rgba(0xFFF6E6, 1.0));
            d.quad(
                p0,
                p0 + side,
                Vec2::ZERO,
                Vec2::splat(MAP as f32),
                rgba(0xFFFFFF, 1.0),
            );
            d.arrow(p0 + to_map(at) / MAP as f32 * side, dir, 8.0 * ui);
            d.text(
                Vec2::new(p0.x, p0.y - 14.0 * ui),
                ui * 1.5,
                "MAP   M OR ESC TO CLOSE",
                white,
            );
        }
        if !self.open() && !self.book.showing() {
            d.text(
                Vec2::new(16.0, h - 16.0 - 8.0 * ui),
                ui,
                "Esc journal   M map   N notes",
                rgba(0xFFFFFF, 0.85),
            );
        }
        if !self.open() {
            if !self.book.showing() {
                self.rider_onto(&mut d);
            }
            self.toast_onto(&mut d);
            self.book.draw(&mut d, view);
        }
        d.out
    }
}

impl Hud {
    /// The passenger's little round button, as a middle and radius in pixels.
    pub fn rider_button(&self, size: Vec2) -> Option<(Vec2, f32)> {
        let at = self.rider_at?;
        let px = (size.y / 34.0).min(size.x / 34.0).clamp(11.0, 26.0);
        let r = px * 1.1;
        let c = at + Vec2::new(px * 2.6, px * 0.4);
        let c = c.clamp(Vec2::splat(r + 4.0), size - (r + 4.0));
        Some((c, r))
    }

    /// The passenger's own bits: their button, what they say in a bubble beside them,
    /// and a marker on whatever they point at.
    fn rider_onto(&self, d: &mut Draw) {
        let size = d.size;
        let px = (size.y / 34.0).min(size.x / 34.0).clamp(11.0, 26.0);
        if let Some((c, r)) = self.rider_button(size) {
            for (rad, col) in [(r + 2.0, rgba(0x4A3426, 0.6)), (r, rgba(0xFFF6E6, 0.95))] {
                for k in 0..16 {
                    let a0 = k as f32 / 16.0 * std::f32::consts::TAU;
                    let a1 = (k + 1) as f32 / 16.0 * std::f32::consts::TAU;
                    d.tri(
                        c,
                        c + Vec2::new(a0.cos(), a0.sin()) * rad,
                        c + Vec2::new(a1.cos(), a1.sin()) * rad,
                        col,
                    );
                }
            }
            for k in -1..=1 {
                let p = c + Vec2::new(k as f32 * r * 0.42, 0.0);
                d.rect(p - r * 0.12, p + r * 0.12, rgba(0xE0564A, 1.0));
            }
        }
        if let (Some((text, left)), Some(at)) = (&self.say, self.rider_at) {
            let fade = left.clamp(0.0, 1.0);
            let f = crate::font::get();
            let small = px * 0.85;
            let most = (size.x * 0.42).clamp(px * 9.0, px * 16.0);
            let lines: Vec<String> = f.wrap(small, text, most).into_iter().take(3).collect();
            let tw = lines
                .iter()
                .map(|l| f.measure(small, l))
                .fold(0.0, f32::max);
            let bs = Vec2::new(tw + px * 1.2, lines.len() as f32 * small * 1.25 + px * 0.7);
            // Above and to the left of their head, kept on screen.
            let p0 = (at + Vec2::new(-bs.x - px * 0.6, -bs.y - px * 2.2))
                .clamp(Vec2::splat(8.0), size - bs - 8.0);
            let tip = at - Vec2::new(0.0, px * 0.6);
            let foot = Vec2::new(
                (tip.x - px * 0.3).clamp(p0.x + px, p0.x + bs.x - px * 1.6),
                p0.y + bs.y,
            );
            d.tri(
                foot,
                foot + Vec2::new(px * 1.1, 0.0),
                tip,
                rgba(0xFFFDF6, 0.95 * fade),
            );
            d.rect(p0 - 1.5, p0 + bs + 1.5, rgba(0x4A3426, 0.35 * fade));
            d.rect(p0, p0 + bs, rgba(0xFFFDF6, 0.95 * fade));
            let mut y = p0.y + px * 0.35;
            for l in &lines {
                d.write(
                    Vec2::new(p0.x + px * 0.6, y),
                    small,
                    l,
                    rgba(0x4A3426, fade),
                );
                y += small * 1.25;
            }
        }
        if let Some((p, seen, label)) = &self.goal {
            let f = crate::font::get();
            let small = px * 0.8;
            let tw = f.measure(small, label);
            let pad = Vec2::new(px * 0.45, px * 0.25);
            let bs = Vec2::new(tw, small * 1.15) + pad * 2.0;
            let coral = rgba(0xE0564A, 1.0);
            let dark = rgba(0x2A1E14, 0.8);
            let tag = |d: &mut Draw, p0: Vec2| {
                let p0 = p0.clamp(Vec2::splat(6.0), size - bs - 6.0);
                d.rect(p0 - 1.5, p0 + bs + 1.5, rgba(0x4A3426, 0.5));
                d.rect(p0, p0 + bs, rgba(0xFFF6E6, 0.95));
                d.write(p0 + pad, small, label, rgba(0x4A3426, 1.0));
            };
            if *seen {
                // A pin standing over the spot, with its name and distance on top.
                let tip = *p;
                let head = tip - Vec2::new(0.0, px * 1.6);
                for (grow, col) in [(1.3, dark), (1.0, coral)] {
                    d.tri(
                        tip + Vec2::new(0.0, (grow - 1.0) * px * 0.4),
                        head + Vec2::new(-px * 0.55 * grow, 0.0),
                        head + Vec2::new(px * 0.55 * grow, 0.0),
                        col,
                    );
                    d.rect(head - px * 0.55 * grow, head + px * 0.55 * grow, col);
                }
                d.rect(head - px * 0.2, head + px * 0.2, rgba(0xFFF6E6, 1.0));
                tag(d, head - Vec2::new(bs.x * 0.5, px * 0.8 + bs.y));
            } else {
                // Off screen: a big arrow at the edge the whole way, with the name by it.
                let mid = size * 0.5;
                let dir = (*p - mid).normalize_or(Vec2::Y);
                let reach = (mid - px * 3.0) / dir.abs().max(Vec2::splat(1e-3));
                let at = mid + dir * reach.x.min(reach.y);
                d.arrow(at, dir, px * 1.3);
                tag(d, at - dir * px * 2.6 - bs * 0.5);
            }
        }
        if let Some((p, seen)) = self.point {
            let gold = rgba(0xFFD54A, 1.0);
            let dark = rgba(0x2A1E14, 0.8);
            if seen {
                // A little arrow bobbing over it.
                let tip = p - Vec2::new(0.0, px * 1.2);
                let top = tip - Vec2::new(0.0, px * 1.6);
                for (grow, col) in [(1.35, dark), (1.0, gold)] {
                    d.tri(
                        tip + Vec2::new(0.0, (grow - 1.0) * px * 0.5),
                        top + Vec2::new(-px * 0.8 * grow, 0.0),
                        top + Vec2::new(px * 0.8 * grow, 0.0),
                        col,
                    );
                }
            } else {
                // Off screen: an arrow at the edge, pointing the way.
                let mid = size * 0.5;
                let dir = (p - mid).normalize_or(Vec2::Y);
                let reach = (mid - px * 2.5) / dir.abs().max(Vec2::splat(1e-3));
                let at = mid + dir * reach.x.min(reach.y);
                d.arrow(at, dir, px * 0.9);
            }
        }
    }

    fn toast_onto(&self, d: &mut Draw) {
        if let Some((who, text, left)) = &self.toast
            && !self.book.showing()
        {
            crate::book::draw_toast(d, who, text, left.clamp(0.0, 1.0));
        }
    }

    /// Notes mode: help along the top, the list down the right, and the note being
    /// written in the middle.
    fn notes_panel(&self, d: &mut Draw, ui: f32) {
        let (w, h) = (d.size.x, d.size.y);
        let s = ui;
        let (cw, line) = (6.0 * s, 10.0 * s);
        let shade = rgba(0x1E1A16, 0.78);
        let (gold, cream, grey) = (
            rgba(0xF2C14E, 1.0),
            rgba(0xFFE7B0, 1.0),
            rgba(0xBBBBBB, 1.0),
        );
        let white = rgba(0xFFFFFF, 1.0);
        let top = line * 2.0 + 8.0;
        d.rect(Vec2::ZERO, Vec2::new(w, top), shade);
        d.text(Vec2::new(12.0, 8.0), s, "NOTES", gold);
        d.text(
            Vec2::new(12.0 + 7.0 * cw, 8.0),
            s,
            "WASD MOVE   ARROWS OR RIGHT DRAG LOOK   SPACE UP   SHIFT DOWN",
            white,
        );
        d.text(
            Vec2::new(12.0 + 7.0 * cw, 8.0 + line),
            s,
            "CLICK A SPOT TO LEAVE A NOTE   N OR ESC BACK TO FLYING",
            white,
        );

        // The list, newest first, scrolled so the picked note shows.
        let pw = (w * 0.34).clamp(160.0, 52.0 * cw + 24.0);
        let (x0, y0, y1) = (w - pw - 12.0, top + 12.0, h - 12.0);
        let cols = ((pw - 24.0) / cw) as usize;
        d.rect(Vec2::new(x0, y0), Vec2::new(w - 12.0, y1), shade);
        let x = x0 + 12.0;
        let mut y = y0 + 10.0;
        d.text(
            Vec2::new(x, y),
            s,
            &format!("NOTES ({})", self.notes.len()),
            gold,
        );
        y += line;
        for l in wrap(&self.notes_where, cols) {
            d.text(Vec2::new(x, y), s, &l, grey);
            y += line;
        }
        y += line * 0.5;
        let foot = y1 - line * 2.0 - 8.0;
        if self.notes.is_empty() {
            for l in wrap(
                "No notes yet. Click anywhere in the world to leave one.",
                cols,
            ) {
                d.text(Vec2::new(x, y), s, &l, white);
                y += line;
            }
        }
        for (i, n) in self
            .notes
            .iter()
            .rev()
            .enumerate()
            .skip(self.note_pick.saturating_sub(3))
        {
            let picked = i == self.note_pick;
            let mut lines = wrap(&n.text, cols);
            if !picked && lines.len() > 3 {
                lines.truncate(3);
                lines[2].push_str("..");
            }
            let place = n
                .near
                .iter()
                .rev()
                .find(|s| s.starts_with("in "))
                .or(n.near.first())
                .cloned()
                .unwrap_or_default();
            let meta = format!("{} - {place}", if n.done { "DONE" } else { "OPEN" });
            let mut replies = Vec::new();
            if picked {
                for (from, text) in &n.replies {
                    replies.extend(wrap(&format!("{from}: {text}"), cols - 2));
                }
            }
            let tall = (lines.len() + 1 + replies.len()) as f32 * line + 6.0;
            if y + tall > foot {
                break;
            }
            if picked {
                d.rect(
                    Vec2::new(x0 + 4.0, y - 4.0),
                    Vec2::new(w - 16.0, y + tall - 4.0),
                    rgba(0xE0564A, 0.45),
                );
            }
            let color = if n.done { grey } else { white };
            for l in &lines {
                d.text(Vec2::new(x, y), s, l, color);
                y += line;
            }
            d.text(Vec2::new(x, y), s, &meta, if n.done { grey } else { gold });
            y += line;
            for l in &replies {
                d.text(Vec2::new(x + 2.0 * cw, y), s, l, cream);
                y += line;
            }
            y += 6.0;
        }
        let tip = if self.deleting {
            "PRESS DELETE AGAIN TO DELETE IT"
        } else {
            "[ ] PICK   G GO THERE   DEL DELETE"
        };
        d.text(Vec2::new(x, y1 - line - 6.0), s, tip, gold);

        // The note being written.
        if let Some(n) = &self.writing {
            let bw = (w - pw - 48.0).min(60.0 * cw * 1.25 + 32.0);
            let s2 = s * 1.25;
            let (cw2, line2) = (6.0 * s2, 10.0 * s2);
            let cols2 = ((bw - 32.0) / cw2) as usize;
            let mut text = wrap(&format!("{}_", n.text), cols2);
            if text.len() > 8 {
                text.drain(..text.len() - 8);
            }
            let near = wrap(
                &n.near
                    .iter()
                    .take(3)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(" - "),
                cols2,
            );
            let bh = (text.len() + near.len() + 4) as f32 * line2 + 24.0;
            let p0 = Vec2::new(12.0 + (w - pw - 24.0 - bw) * 0.5, (h - bh) * 0.5);
            d.rect(p0, p0 + Vec2::new(bw, bh), rgba(0x1E1A16, 0.9));
            let mut y = p0.y + 14.0;
            let x = p0.x + 16.0;
            d.text(Vec2::new(x, y), s2, "NEW NOTE", gold);
            y += line2;
            for l in &near {
                d.text(Vec2::new(x, y), s2, l, grey);
                y += line2;
            }
            y += line2 * 0.5;
            for l in &text {
                d.text(Vec2::new(x, y), s2, l, white);
                y += line2;
            }
            y += line2 * 0.5;
            d.text(Vec2::new(x, y), s2, "ENTER SAVES   ESC CANCELS", gold);
        }
    }
}

impl Bindings {
    /// The keys as text, one action a line: `flap Space KeyV`.
    pub fn to_text(&self) -> String {
        let mut out = String::new();
        for a in Action::ALL {
            out += a.name();
            for (k, _) in self.keys.iter().filter(|(_, b)| *b == a) {
                out += " ";
                out += k;
            }
            out += "\n";
        }
        out
    }

    /// Makes `code` the one key for `action`, taking it from whatever had it before.
    pub fn rebind(&mut self, action: Action, code: &str) {
        self.keys.retain(|(k, a)| k != code && *a != action);
        self.keys.push((code.to_string(), action));
    }

    /// Keys from [`Bindings::to_text`], the rest left as they are. Gamepad buttons stay.
    pub fn set_text(&mut self, text: &str) {
        let mut keys = Vec::new();
        for line in text.lines() {
            let mut words = line.split_whitespace();
            let Some(a) = words.next().and_then(Action::from_name) else {
                continue;
            };
            keys.extend(words.map(|k| (k.to_string(), a)));
        }
        if !keys.is_empty() {
            self.keys = keys;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Keys saved as text come back the same, and a new key moves off its old action.
    #[test]
    fn keys_round_trip_and_rebind() {
        let mut b = Bindings::default();
        b.rebind(Action::Flap, "KeyF");
        assert_eq!(b.key_action("KeyF"), Some(Action::Flap));
        assert_eq!(b.key_action("Space"), None);
        let mut c = Bindings::default();
        c.set_text(&b.to_text());
        let sorted = |b: &Bindings| {
            let mut k: Vec<String> = b.keys.iter().map(|(k, a)| format!("{k} {a:?}")).collect();
            k.sort();
            k
        };
        assert_eq!(sorted(&c), sorted(&b));
        let mut hud = Hud {
            on: true,
            ..Default::default()
        };
        assert_eq!(hud.key("KeyM"), HudKey::Used);
        assert!(hud.open());
        assert_eq!(hud.key("Escape"), HudKey::Used);
        assert_eq!(hud.key("KeyW"), HudKey::Pass);
    }

    /// Writing a note queues it for saving, and deleting takes two presses.
    #[test]
    fn notes_write_and_delete() {
        let mut hud = Hud::default();
        assert_eq!(hud.notes_key("KeyW", None), NotesKey::Pass);
        hud.writing = Some(Note::default());
        hud.notes_key("KeyH", Some("Hi"));
        hud.notes_key("Space", Some(" there!"));
        hud.notes_key("Backspace", None);
        assert_eq!(hud.notes_key("Enter", None), NotesKey::Used);
        assert_eq!(hud.notes.len(), 1);
        assert!(matches!(&hud.outbox[0], NoteChange::Save(n) if n.text == "Hi there"));
        hud.notes[0].id = "a".into();
        hud.notes_key("Delete", None);
        assert_eq!(hud.notes.len(), 1);
        hud.notes_key("Delete", None);
        assert!(hud.notes.is_empty());
        assert_eq!(hud.outbox[1], NoteChange::Delete("a".into()));
        assert_eq!(hud.notes_key("Escape", None), NotesKey::Leave);
        assert_eq!(wrap("one two three", 7), vec!["one two", "three"]);
    }
}
