//! The heads-up display drawn by the game itself, so every build has it: a minimap in the
//! corner, the big map on M, the help panel on H, which also changes the keys, and notes
//! mode on N with its list of notes and the note being written.
//! The web page has its own page chrome and leaves this off.

use bytemuck::{Pod, Zeroable};
use glam::Vec2;

use crate::input::{Action, Bindings};
use crate::notes::Note;

/// The atlas the HUD draws from: the world map in the top `MAP` square, then a row of
/// font cells under it, a white cell first for plain colored shapes.
pub const ATLAS: (u32, u32) = (1024, 1032);
pub const MAP: u32 = 1024;
const CELL: u32 = 8;

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct HudVertex {
    /// Clip space position.
    pub pos: [f32; 2],
    pub uv: [f32; 2],
    /// Linear color with alpha, multiplied with the atlas.
    pub color: [f32; 4],
}

/// 5 by 7 letters, top row first, the leftmost pixel the high bit. Anything else is a
/// space.
fn glyph(c: char) -> [u8; 7] {
    match c {
        'A' => [14, 17, 17, 31, 17, 17, 17],
        'B' => [30, 17, 17, 30, 17, 17, 30],
        'C' => [14, 17, 16, 16, 16, 17, 14],
        'D' => [30, 17, 17, 17, 17, 17, 30],
        'E' => [31, 16, 16, 30, 16, 16, 31],
        'F' => [31, 16, 16, 30, 16, 16, 16],
        'G' => [14, 17, 16, 23, 17, 17, 15],
        'H' => [17, 17, 17, 31, 17, 17, 17],
        'I' => [14, 4, 4, 4, 4, 4, 14],
        'J' => [7, 2, 2, 2, 2, 18, 12],
        'K' => [17, 18, 20, 24, 20, 18, 17],
        'L' => [16, 16, 16, 16, 16, 16, 31],
        'M' => [17, 27, 21, 21, 17, 17, 17],
        'N' => [17, 17, 25, 21, 19, 17, 17],
        'O' => [14, 17, 17, 17, 17, 17, 14],
        'P' => [30, 17, 17, 30, 16, 16, 16],
        'Q' => [14, 17, 17, 17, 21, 18, 13],
        'R' => [30, 17, 17, 30, 20, 18, 17],
        'S' => [15, 16, 16, 14, 1, 1, 30],
        'T' => [31, 4, 4, 4, 4, 4, 4],
        'U' => [17, 17, 17, 17, 17, 17, 14],
        'V' => [17, 17, 17, 17, 17, 10, 4],
        'W' => [17, 17, 17, 21, 21, 21, 10],
        'X' => [17, 17, 10, 4, 10, 17, 17],
        'Y' => [17, 17, 17, 10, 4, 4, 4],
        'Z' => [31, 1, 2, 4, 8, 16, 31],
        '0' => [14, 17, 19, 21, 25, 17, 14],
        '1' => [4, 12, 4, 4, 4, 4, 14],
        '2' => [14, 17, 1, 2, 4, 8, 31],
        '3' => [31, 2, 4, 2, 1, 17, 14],
        '4' => [2, 6, 10, 18, 31, 2, 2],
        '5' => [31, 16, 30, 1, 1, 17, 14],
        '6' => [6, 8, 16, 30, 17, 17, 14],
        '7' => [31, 1, 2, 4, 8, 8, 8],
        '8' => [14, 17, 17, 14, 17, 17, 14],
        '9' => [14, 17, 17, 15, 1, 2, 12],
        '!' => [4, 4, 4, 4, 4, 0, 4],
        '\'' => [4, 4, 8, 0, 0, 0, 0],
        '(' => [2, 4, 8, 8, 8, 4, 2],
        ')' => [8, 4, 2, 2, 2, 4, 8],
        '+' => [0, 4, 4, 31, 4, 4, 0],
        ',' => [0, 0, 0, 0, 12, 4, 8],
        '-' => [0, 0, 0, 31, 0, 0, 0],
        '.' => [0, 0, 0, 0, 0, 12, 12],
        '/' => [0, 1, 2, 4, 8, 16, 0],
        ':' => [0, 12, 12, 0, 12, 12, 0],
        '<' => [2, 4, 8, 16, 8, 4, 2],
        '=' => [0, 0, 31, 0, 31, 0, 0],
        '>' => [8, 4, 2, 1, 2, 4, 8],
        '?' => [14, 17, 1, 2, 4, 0, 4],
        '_' => [0, 0, 0, 0, 0, 0, 31],
        '"' => [10, 10, 0, 0, 0, 0, 0],
        '#' => [10, 10, 31, 10, 31, 10, 10],
        '%' => [24, 25, 2, 4, 8, 19, 3],
        '&' => [12, 18, 20, 8, 21, 18, 13],
        '*' => [0, 4, 21, 14, 21, 4, 0],
        ';' => [0, 12, 12, 0, 12, 4, 8],
        '@' => [14, 17, 23, 21, 23, 16, 15],
        '[' => [14, 8, 8, 8, 8, 8, 14],
        ']' => [14, 2, 2, 2, 2, 2, 14],
        'a' => [0, 0, 14, 1, 15, 17, 15],
        'b' => [16, 16, 22, 25, 17, 17, 30],
        'c' => [0, 0, 14, 16, 16, 17, 14],
        'd' => [1, 1, 13, 19, 17, 17, 15],
        'e' => [0, 0, 14, 17, 31, 16, 14],
        'f' => [6, 9, 8, 28, 8, 8, 8],
        'g' => [0, 15, 17, 17, 15, 1, 14],
        'h' => [16, 16, 22, 25, 17, 17, 17],
        'i' => [4, 0, 12, 4, 4, 4, 14],
        'j' => [2, 0, 6, 2, 2, 18, 12],
        'k' => [16, 16, 18, 20, 24, 20, 18],
        'l' => [12, 4, 4, 4, 4, 4, 14],
        'm' => [0, 0, 26, 21, 21, 17, 17],
        'n' => [0, 0, 22, 25, 17, 17, 17],
        'o' => [0, 0, 14, 17, 17, 17, 14],
        'p' => [0, 30, 17, 17, 30, 16, 16],
        'q' => [0, 15, 17, 17, 15, 1, 1],
        'r' => [0, 0, 22, 25, 16, 16, 16],
        's' => [0, 0, 15, 16, 14, 1, 30],
        't' => [8, 8, 28, 8, 8, 9, 6],
        'u' => [0, 0, 17, 17, 17, 19, 13],
        'v' => [0, 0, 17, 17, 17, 10, 4],
        'w' => [0, 0, 17, 17, 21, 21, 10],
        'x' => [0, 0, 17, 10, 4, 10, 17],
        'y' => [0, 17, 17, 15, 1, 17, 14],
        'z' => [0, 0, 31, 2, 4, 8, 31],
        _ => [0; 7],
    }
}

/// The characters with cells in the atlas, in order after the white cell.
const CHARS: &str =
    " !'()+,-./0123456789:<=>?ABCDEFGHIJKLMNOPQRSTUVWXYZ_\"#%&*;@[]abcdefghijklmnopqrstuvwxyz";

/// Draws the white cell and the font into the atlas rows under the map.
pub fn font_into(atlas: &mut [u8]) {
    let w = ATLAS.0 as usize;
    let mut put = |x: usize, y: usize| {
        let k = ((MAP as usize + y) * w + x) * 4;
        atlas[k..k + 4].copy_from_slice(&[255; 4]);
    };
    for y in 0..CELL as usize {
        for x in 0..CELL as usize {
            put(x, y);
        }
    }
    for (i, c) in CHARS.chars().enumerate() {
        let x0 = (i + 1) * CELL as usize;
        // Letters with tails hang a row lower, into the cell's spare bottom row.
        let drop = "gjpqy".contains(c) as usize;
        for (row, bits) in glyph(c).iter().enumerate() {
            for col in 0..5 {
                if bits & (16 >> col) != 0 {
                    put(x0 + col + 1, row + drop);
                }
            }
        }
    }
}

/// sRGB hex to linear color with alpha.
fn rgba(hex: u32, a: f32) -> [f32; 4] {
    let ch = |s: u32| ((hex >> s) & 255) as f32 / 255.0;
    let lin = |v: f32| v.powf(2.2);
    [lin(ch(16)), lin(ch(8)), lin(ch(0)), a]
}

/// Builds the HUD's triangles in pixels, top left origin, turned to clip space at the end.
struct Draw {
    out: Vec<HudVertex>,
    size: Vec2,
}

impl Draw {
    fn vert(&mut self, p: Vec2, uv: Vec2, color: [f32; 4]) {
        self.out.push(HudVertex {
            pos: [p.x / self.size.x * 2.0 - 1.0, 1.0 - p.y / self.size.y * 2.0],
            uv: [uv.x / ATLAS.0 as f32, uv.y / ATLAS.1 as f32],
            color,
        });
    }

    fn quad(&mut self, p0: Vec2, p1: Vec2, uv0: Vec2, uv1: Vec2, color: [f32; 4]) {
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
    fn rect(&mut self, p0: Vec2, p1: Vec2, color: [f32; 4]) {
        let white = Vec2::new(4.0, MAP as f32 + 4.0);
        self.quad(p0, p1, white, white, color);
    }

    fn tri(&mut self, a: Vec2, b: Vec2, c: Vec2, color: [f32; 4]) {
        let white = Vec2::new(4.0, MAP as f32 + 4.0);
        for p in [a, b, c] {
            self.vert(p, white, color);
        }
    }

    /// Text in the HUD font, `scale` pixels to a font pixel. Returns its width.
    fn text(&mut self, at: Vec2, scale: f32, s: &str, color: [f32; 4]) -> f32 {
        let mut x = at.x;
        for c in s.chars() {
            let i = CHARS
                .find(c)
                .or_else(|| CHARS.find(c.to_ascii_uppercase()))
                .unwrap_or(0) as f32;
            if i > 0.0 {
                let u0 = Vec2::new((i + 1.0) * CELL as f32, MAP as f32);
                self.quad(
                    Vec2::new(x, at.y),
                    Vec2::new(x + CELL as f32 * scale, at.y + CELL as f32 * scale),
                    u0,
                    u0 + Vec2::splat(CELL as f32),
                    color,
                );
            }
            x += 6.0 * scale;
        }
        x - at.x
    }

    /// An arrow pointing along `dir` (screen, y down), for where the dragon heads.
    fn arrow(&mut self, at: Vec2, dir: Vec2, size: f32) {
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
    /// Drawn at all. The web page has its own and leaves this off.
    pub on: bool,
    pub help: bool,
    pub map: bool,
    /// The action picked in the help panel, and whether the next key goes to it.
    pick: usize,
    waiting: bool,
    /// The keys changed, so the host should save them.
    pub changed: bool,
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
        "ArrowUp" => "UP ARROW".into(),
        "ArrowDown" => "DOWN ARROW".into(),
        "ArrowLeft" => "LEFT ARROW".into(),
        "ArrowRight" => "RIGHT ARROW".into(),
        "ShiftLeft" | "ShiftRight" => "SHIFT".into(),
        "ControlLeft" | "ControlRight" => "CTRL".into(),
        "AltLeft" | "AltRight" => "ALT".into(),
        _ => s.to_uppercase(),
    }
}

/// What each action does, for the help panel.
fn action_label(a: Action) -> &'static str {
    match a {
        Action::PitchUp => "NOSE UP",
        Action::PitchDown => "NOSE DOWN",
        Action::TurnLeft => "TURN LEFT",
        Action::TurnRight => "TURN RIGHT",
        Action::Flap => "FLAP",
        Action::Tuck => "DIVE",
        Action::Brake => "BRAKE, LAND, HOVER",
        Action::Breath => "BREATHE FIRE",
    }
}

impl Hud {
    /// Whether a panel covers the game, which pauses it.
    pub fn open(&self) -> bool {
        self.help || self.map
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

    /// Offers a key press to the HUD first.
    pub fn key(&mut self, code: &str, binds: &mut Bindings) -> HudKey {
        if !self.on {
            return HudKey::Pass;
        }
        if self.waiting {
            // The HUD's own keys and pause stay as they are.
            if matches!(code, "KeyH" | "KeyM" | "KeyP") {
                return HudKey::Used;
            }
            self.waiting = false;
            if code != "Escape" {
                binds.rebind(Action::ALL[self.pick], code);
                self.changed = true;
            }
            return HudKey::Used;
        }
        match code {
            "KeyH" => {
                self.help = !self.help;
                self.map = false;
            }
            "KeyM" => {
                self.map = !self.map;
                self.help = false;
            }
            "Escape" if self.open() => {
                self.help = false;
                self.map = false;
            }
            "ArrowUp" if self.help => {
                self.pick = (self.pick + Action::ALL.len() - 1) % Action::ALL.len();
            }
            "ArrowDown" if self.help => self.pick = (self.pick + 1) % Action::ALL.len(),
            "Enter" | "NumpadEnter" if self.help => self.waiting = true,
            "KeyR" if self.help => {
                *binds = Bindings::default();
                self.changed = true;
            }
            _ if self.open() => {}
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
        binds: &Bindings,
        map_ready: bool,
        notes: bool,
        paused: bool,
        muted: bool,
    ) -> Vec<HudVertex> {
        let mut d = Draw {
            out: Vec::new(),
            size: Vec2::new(size.0 as f32, size.1 as f32),
        };
        if !self.on {
            return d.out;
        }
        let (w, h) = (d.size.x, d.size.y);
        let ui = (h / 360.0).clamp(1.0, 3.0).round();
        let span = Vec2::new(bounds[2] - bounds[0], bounds[3] - bounds[1]);
        let to_map = |p: Vec2| (p - Vec2::new(bounds[0], bounds[1])) / span * MAP as f32;
        let dir = Vec2::new(heading.sin(), heading.cos());
        let white = rgba(0xFFFFFF, 1.0);
        let shade = rgba(0x1E1A16, 0.72);
        if notes {
            self.notes_panel(&mut d, ui);
            return d.out;
        }
        if map_ready && !self.map {
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
        if self.help {
            let line = 11.0 * ui * 1.4;
            let rows = Action::ALL.len() as f32 + 10.0;
            let bw = (w * 0.9).min(48.0 * 6.0 * ui * 1.4 + 36.0);
            let bh = rows * line + 30.0;
            let p0 = Vec2::new((w - bw) * 0.5, (h - bh) * 0.5);
            d.rect(p0, p0 + Vec2::new(bw, bh), shade);
            let s = ui * 1.4;
            let mut y = p0.y + 14.0;
            let x = p0.x + 18.0;
            d.text(Vec2::new(x, y), s * 1.3, "CONTROLS", rgba(0xF2C14E, 1.0));
            y += line * 1.6;
            for (i, a) in Action::ALL.into_iter().enumerate() {
                if i == self.pick {
                    d.rect(
                        Vec2::new(x - 8.0, y - 3.0),
                        Vec2::new(p0.x + bw - 10.0, y + line - 3.0),
                        rgba(0xE0564A, 0.6),
                    );
                }
                d.text(Vec2::new(x, y), s, action_label(a), white);
                let mut keys: Vec<String> = Vec::new();
                for (k, _) in binds.keys.iter().filter(|(_, b)| *b == a) {
                    let label = key_label(k);
                    if !keys.contains(&label) {
                        keys.push(label);
                    }
                }
                let shown = if self.waiting && i == self.pick {
                    "PRESS A KEY...".to_string()
                } else {
                    keys.join("  ")
                };
                d.text(
                    Vec2::new(x + 20.0 * 6.0 * s, y),
                    s,
                    &shown,
                    rgba(0xFFE7B0, 1.0),
                );
                y += line;
            }
            y += line * 0.5;
            for l in [
                "I J K L OR RIGHT DRAG  LOOK ROUND",
                "M  MAP      H  THIS HELP      N  NOTES",
                "ESC OR P  PAUSE      U  SOUND ON/OFF",
            ] {
                d.text(Vec2::new(x, y), s, l, rgba(0xDDDDDD, 1.0));
                y += line;
            }
            y += line * 0.5;
            let tip = if self.waiting {
                "PRESS THE NEW KEY, OR ESC TO KEEP THE OLD"
            } else {
                "UP DOWN PICK   ENTER CHANGE   R RESET ALL"
            };
            d.text(Vec2::new(x, y), s, tip, rgba(0xF2C14E, 1.0));
        }
        if paused && !self.open() {
            let s = ui * 1.5;
            let line = 11.0 * s;
            let (bw, bh) = (34.0 * 6.0 * s + 40.0, line * 4.0 + 28.0);
            let p0 = Vec2::new((w - bw) * 0.5, (h - bh) * 0.5);
            d.rect(p0, p0 + Vec2::new(bw, bh), shade);
            let x = p0.x + 20.0;
            let mut y = p0.y + 16.0;
            d.text(Vec2::new(x, y), s * 1.3, "PAUSED", rgba(0xF2C14E, 1.0));
            y += line * 1.6;
            let sound = if muted {
                "U  SOUND: OFF"
            } else {
                "U  SOUND: ON"
            };
            d.text(Vec2::new(x, y), s, sound, white);
            y += line;
            d.text(
                Vec2::new(x, y),
                s,
                "ESC OR P  FLY ON     H  HELP",
                rgba(0xDDDDDD, 1.0),
            );
        }
        if !self.open() {
            d.text(
                Vec2::new(16.0, h - 16.0 - 8.0 * ui),
                ui,
                "H HELP   M MAP   N NOTES",
                rgba(0xFFFFFF, 0.8),
            );
        }
        d.out
    }
}

impl Hud {
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
        assert_eq!(hud.key("KeyH", &mut c), HudKey::Used);
        assert!(hud.open());
        hud.key("Enter", &mut c);
        hud.key("KeyZ", &mut c);
        assert_eq!(c.key_action("KeyZ"), Some(Action::PitchUp));
        assert!(hud.changed);
        assert_eq!(hud.key("Escape", &mut c), HudKey::Used);
        assert_eq!(hud.key("KeyW", &mut c), HudKey::Pass);
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
