//! The HUD's lettering: Nunito Bold, drawn into the HUD atlas once at a few sizes. Text
//! picks the size nearest above what it needs and scales down a touch, so it stays
//! crisp at any screen size.

use std::collections::HashMap;
use std::sync::OnceLock;

/// The sizes, in pixels to the em, drawn into the atlas.
const SIZES: [f32; 4] = [15.0, 22.0, 32.0, 46.0];
/// Gap between glyphs in the atlas, so scaled text never bleeds into its neighbors.
const PAD: usize = 2;
/// Characters past plain ASCII that stories and the journal use.
const EXTRA: &str = "’‘“”…·éèáàöüñ—–•★♪";

#[derive(Clone, Copy, Debug)]
pub struct Glyph {
    /// Top left of the glyph in the atlas, in pixels.
    pub uv: (f32, f32),
    pub size: (f32, f32),
    /// From the pen on the baseline to the glyph's top left, y down.
    pub offset: (f32, f32),
    pub advance: f32,
}

pub struct Face {
    pub em: f32,
    glyphs: HashMap<char, Glyph>,
}

pub struct Font {
    faces: Vec<Face>,
    /// The atlas rows the glyphs fill, starting at `top`, as alpha.
    pub rows: Vec<u8>,
    pub top: u32,
    pub width: u32,
}

static FONT: OnceLock<Font> = OnceLock::new();

/// The font, drawn the first time it is asked for. Glyph rows start at `top` in an atlas
/// `width` wide.
pub fn get() -> &'static Font {
    FONT.get_or_init(|| Font::new(crate::hud::ATLAS.0, crate::hud::FONT_TOP))
}

impl Font {
    fn new(width: u32, top: u32) -> Self {
        let font = fontdue::Font::from_bytes(
            include_bytes!("../assets/nunito-bold.ttf") as &[u8],
            fontdue::FontSettings::default(),
        )
        .expect("nunito loads");
        let chars: Vec<char> = (' '..='~').chain(EXTRA.chars()).collect();
        let w = width as usize;
        let mut rows: Vec<u8> = Vec::new();
        let (mut x, mut y, mut shelf) = (0usize, 0usize, 0usize);
        let mut faces = Vec::new();
        for em in SIZES {
            let mut glyphs = HashMap::new();
            for &c in &chars {
                let (m, bits) = font.rasterize(c, em);
                if x + m.width + PAD > w {
                    x = 0;
                    y += shelf + PAD;
                    shelf = 0;
                }
                rows.resize(rows.len().max((y + m.height + PAD) * w), 0);
                for r in 0..m.height {
                    let at = (y + r) * w + x;
                    rows[at..at + m.width].copy_from_slice(&bits[r * m.width..(r + 1) * m.width]);
                }
                glyphs.insert(
                    c,
                    Glyph {
                        uv: (x as f32, (top as usize + y) as f32),
                        size: (m.width as f32, m.height as f32),
                        offset: (m.xmin as f32, -(m.ymin as f32 + m.height as f32)),
                        advance: m.advance_width,
                    },
                );
                x += m.width + PAD;
                shelf = shelf.max(m.height);
            }
            faces.push(Face { em, glyphs });
        }
        Self {
            faces,
            rows,
            top,
            width,
        }
    }

    /// The drawn face to use for text `px` to the em, and how much to scale it.
    pub fn face(&self, px: f32) -> (&Face, f32) {
        let f = self
            .faces
            .iter()
            .find(|f| f.em >= px * 0.95)
            .unwrap_or_else(|| self.faces.last().expect("a face"));
        (f, px / f.em)
    }

    /// How wide `s` is at `px` to the em.
    pub fn measure(&self, px: f32, s: &str) -> f32 {
        let (f, k) = self.face(px);
        s.chars().map(|c| f.glyph(c).advance).sum::<f32>() * k
    }

    /// `s` broken into lines no wider than `width` at `px`, at spaces where it can.
    pub fn wrap(&self, px: f32, s: &str, width: f32) -> Vec<String> {
        let mut out = Vec::new();
        for para in s.split('\n') {
            let mut line = String::new();
            for word in para.split(' ') {
                let joined = if line.is_empty() {
                    word.to_string()
                } else {
                    format!("{line} {word}")
                };
                if !line.is_empty() && self.measure(px, &joined) > width {
                    out.push(std::mem::replace(&mut line, word.to_string()));
                } else {
                    line = joined;
                }
            }
            out.push(line);
        }
        out
    }
}

impl Face {
    /// The glyph for `c`, a question mark for anything the font lacks.
    pub fn glyph(&self, c: char) -> Glyph {
        self.glyphs
            .get(&c)
            .or_else(|| self.glyphs.get(&'?'))
            .copied()
            .expect("question mark glyph")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every size fits in the atlas rows set aside for it, and wrapping keeps to the width.
    #[test]
    fn fits_and_wraps() {
        let f = get();
        let (aw, ah) = crate::hud::ATLAS;
        assert!(
            f.rows.len() <= (aw * (ah - f.top)) as usize,
            "{}",
            f.rows.len()
        );
        let lines = f.wrap(22.0, "a little brass bell, green with the sea", 120.0);
        assert!(lines.len() > 1);
        assert!(lines.iter().all(|l| f.measure(22.0, l) <= 120.0));
        assert!(f.measure(22.0, "Hello") > f.measure(15.0, "Hello"));
    }
}
