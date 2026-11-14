//! Postcard pictures: a frame of the game shrunk to a small card, softened and painted
//! down to fewer colors, and kept as short text so it travels in the save.

/// Size of a postcard picture in pixels.
pub const PIC: (usize, usize) = (160, 90);

/// Paints a `w` by `h` RGBA frame into a postcard picture: the middle of the frame,
/// cropped to the card's shape, averaged down, its colors laid on in flat strokes.
pub fn paint(rgba: &[u8], w: usize, h: usize) -> Vec<u8> {
    let (pw, ph) = PIC;
    let mut out = vec![255u8; pw * ph * 4];
    if w == 0 || h == 0 || rgba.len() < w * h * 4 {
        return out;
    }
    // The largest box of the card's shape in the middle of the frame.
    let (cw, ch) = if w * ph > h * pw {
        (h * pw / ph, h)
    } else {
        (w, w * ph / pw)
    };
    let (x0, y0) = ((w - cw) / 2, (h - ch) / 2);
    for py in 0..ph {
        for px in 0..pw {
            let (sx0, sx1) = (
                x0 + px * cw / pw,
                x0 + ((px + 1) * cw / pw).max(px * cw / pw + 1),
            );
            let (sy0, sy1) = (
                y0 + py * ch / ph,
                y0 + ((py + 1) * ch / ph).max(py * ch / ph + 1),
            );
            let mut sum = [0u32; 3];
            let mut n = 0;
            for sy in sy0..sy1.min(h) {
                for sx in sx0..sx1.min(w) {
                    let k = (sy * w + sx) * 4;
                    for c in 0..3 {
                        sum[c] += rgba[k + c] as u32;
                    }
                    n += 1;
                }
            }
            let k = (py * pw + px) * 4;
            for c in 0..3 {
                let v = sum[c] / n.max(1);
                // A little warmer and brighter, like paint on card, then sixteen levels.
                let v = (v as f32 * if c == 2 { 1.0 } else { 1.04 } + 6.0).min(255.0) as u32;
                out[k + c] = level(v as u8) * 17;
            }
        }
    }
    out
}

fn level(v: u8) -> u8 {
    ((v as u32 * 15 + 127) / 255) as u8
}

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// A painted picture as text: runs of up to 16 same colored pixels, two bytes a run
/// (red and green levels, then blue level and the run length), in base64.
pub fn encode(pic: &[u8]) -> String {
    let px: Vec<[u8; 3]> = pic
        .chunks(4)
        .map(|p| [level(p[0]), level(p[1]), level(p[2])])
        .collect();
    let mut bytes = Vec::new();
    let mut i = 0;
    while i < px.len() {
        let mut run = 1;
        while run < 16 && i + run < px.len() && px[i + run] == px[i] {
            run += 1;
        }
        let [r, g, b] = px[i];
        bytes.push((r << 4) | g);
        bytes.push((b << 4) | (run as u8 - 1));
        i += run;
    }
    let mut s = String::with_capacity(bytes.len() * 4 / 3 + 4);
    for c in bytes.chunks(3) {
        let n = (c[0] as u32) << 16
            | (*c.get(1).unwrap_or(&0) as u32) << 8
            | *c.get(2).unwrap_or(&0) as u32;
        for k in 0..=c.len() {
            s.push(B64[(n >> (18 - 6 * k)) as usize & 63] as char);
        }
    }
    s
}

/// A picture from [`encode`], or none if the text is not one.
pub fn decode(s: &str) -> Option<Vec<u8>> {
    let mut bytes = Vec::with_capacity(s.len() * 3 / 4);
    let vals: Option<Vec<u32>> = s
        .bytes()
        .map(|c| B64.iter().position(|b| *b == c).map(|v| v as u32))
        .collect();
    for c in vals?.chunks(4) {
        let mut n = 0;
        for (k, v) in c.iter().enumerate() {
            n |= v << (18 - 6 * k);
        }
        for k in 0..c.len().saturating_sub(1) {
            bytes.push((n >> (16 - 8 * k)) as u8);
        }
    }
    let (pw, ph) = PIC;
    let mut out = Vec::with_capacity(pw * ph * 4);
    for run in bytes.as_chunks::<2>().0 {
        let (r, g, b) = (run[0] >> 4, run[0] & 15, run[1] >> 4);
        for _ in 0..=(run[1] & 15) {
            out.extend_from_slice(&[r * 17, g * 17, b * 17, 255]);
        }
    }
    (out.len() == pw * ph * 4).then_some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A painted frame comes back from its text exactly, and stays small.
    #[test]
    fn paints_and_round_trips() {
        let (w, h) = (320, 200);
        let frame: Vec<u8> = (0..w * h)
            .flat_map(|i| {
                let (x, y) = (i % w, i / w);
                [
                    (x / 3) as u8,
                    (y + 40) as u8,
                    if x > 160 { 200 } else { 90 },
                    255,
                ]
            })
            .collect();
        let pic = paint(&frame, w, h);
        assert_eq!(pic.len(), PIC.0 * PIC.1 * 4);
        let text = encode(&pic);
        assert!(text.len() < 30_000, "{}", text.len());
        assert_eq!(decode(&text).as_deref(), Some(&pic[..]));
        assert_eq!(decode("not a picture!"), None);
    }
}
