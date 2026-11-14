//! Outline and path shaping with plain arithmetic only, so every machine that builds the
//! world gets bit-identical results (no platform sin, cos or pow).

/// Stable hash of a string (FNV-1a).
pub fn hash_str(s: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}

fn hash(seed: u64, i: i64) -> f64 {
    let mut z = seed ^ (i as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;
    (z >> 11) as f64 / (1u64 << 53) as f64
}

/// Smooth 1D value noise in `[-1, 1]`.
fn noise1(seed: u64, x: f64) -> f64 {
    let i = x.floor();
    let t = x - i;
    let s = t * t * (3.0 - 2.0 * t);
    let (a, b) = (hash(seed, i as i64), hash(seed, i as i64 + 1));
    (a + (b - a) * s) * 2.0 - 1.0
}

/// Sine and cosine of an angle in degrees, by Taylor series on a reduced angle.
pub fn sincos_deg(deg: f64) -> (f64, f64) {
    let pi = std::f64::consts::PI;
    let mut d = deg % 360.0;
    if d > 180.0 {
        d -= 360.0;
    }
    if d < -180.0 {
        d += 360.0;
    }
    let x = d * pi / 180.0;
    let (mut s, mut c) = (0.0, 0.0);
    let (mut ts, mut tc) = (x, 1.0);
    for k in 0..20 {
        s += ts;
        c += tc;
        let k = k as f64;
        ts *= -x * x / ((2.0 * k + 2.0) * (2.0 * k + 3.0));
        tc *= -x * x / ((2.0 * k + 1.0) * (2.0 * k + 2.0));
    }
    (s, c)
}

fn add<const N: usize>(a: [f64; N], b: [f64; N], t: f64) -> [f64; N] {
    let mut o = a;
    for i in 0..N {
        o[i] += b[i] * t;
    }
    o
}

fn sub<const N: usize>(a: [f64; N], b: [f64; N]) -> [f64; N] {
    let mut o = a;
    for i in 0..N {
        o[i] -= b[i];
    }
    o
}

pub fn len2(a: [f64; 2]) -> f64 {
    (a[0] * a[0] + a[1] * a[1]).sqrt()
}

/// Catmull-Rom through the points, sampled about every `step` meters on x/z. Sharp points
/// get no tangent, so they stay corners. Returns the dense points and, for each dense
/// segment, the source segment it came from.
pub fn smooth<const N: usize>(
    pts: &[([f64; N], bool)],
    closed: bool,
    step: f64,
) -> (Vec<[f64; N]>, Vec<usize>) {
    let n = pts.len();
    if n < 2 {
        return (pts.iter().map(|p| p.0).collect(), Vec::new());
    }
    let at = |i: isize| -> [f64; N] {
        if closed {
            pts[i.rem_euclid(n as isize) as usize].0
        } else {
            pts[i.clamp(0, n as isize - 1) as usize].0
        }
    };
    let tangent = |i: usize| -> [f64; N] {
        if pts[i].1 {
            return [0.0; N];
        }
        let (a, b) = (at(i as isize - 1), at(i as isize + 1));
        let mut t = sub(b, a);
        for v in &mut t {
            *v *= 0.5;
        }
        t
    };
    let segs = if closed { n } else { n - 1 };
    let mut out = Vec::new();
    let mut from = Vec::new();
    for i in 0..segs {
        let (p0, p1) = (at(i as isize), at(i as isize + 1));
        let (m0, m1) = (tangent(i), tangent((i + 1) % n));
        // x and z are always the first two coordinates.
        let flat = len2([p1[0] - p0[0], p1[1] - p0[1]]);
        let k = ((flat / step).ceil() as usize).max(1);
        for j in 0..k {
            let t = j as f64 / k as f64;
            let (t2, t3) = (t * t, t * t * t);
            let h00 = 2.0 * t3 - 3.0 * t2 + 1.0;
            let h10 = t3 - 2.0 * t2 + t;
            let h01 = -2.0 * t3 + 3.0 * t2;
            let h11 = t3 - t2;
            let mut p = [0.0; N];
            p = add(p, p0, h00);
            p = add(p, m0, h10);
            p = add(p, p1, h01);
            p = add(p, m1, h11);
            out.push(p);
            from.push(i);
        }
    }
    if !closed {
        out.push(at(n as isize - 1));
    }
    (out, from)
}

/// Pushes outline points in or out along their normals by fixed fractal noise, so coasts
/// and edges are rough at scales below the hand-placed points.
pub fn jag(pts: &[[f64; 2]], closed: bool, amp: f64, seed: u64) -> Vec<[f64; 2]> {
    if amp <= 0.0 || pts.len() < 3 {
        return pts.to_vec();
    }
    let n = pts.len();
    let mut s = 0.0;
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let prev = if i == 0 {
            if closed { pts[n - 1] } else { pts[0] }
        } else {
            pts[i - 1]
        };
        let next = if i + 1 == n {
            if closed { pts[0] } else { pts[n - 1] }
        } else {
            pts[i + 1]
        };
        if i > 0 {
            s += len2(sub(pts[i], pts[i - 1]));
        }
        let d = sub(next, prev);
        let l = len2(d).max(1e-9);
        let normal = [-d[1] / l, d[0] / l];
        let mut off = noise1(seed, s / 160.0) * 0.55
            + noise1(seed ^ 0x51, s / 65.0) * 0.3
            + noise1(seed ^ 0xA2, s / 27.0) * 0.15;
        // Open paths keep their ends where they were placed.
        if !closed && (i == 0 || i + 1 == n) {
            off = 0.0;
        }
        out.push(add(pts[i], normal, off * amp));
    }
    out
}
