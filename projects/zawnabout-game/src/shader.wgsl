struct Globals {
    view_proj: mat4x4f,
    inv_view_proj: mat4x4f,
    cam_pos: vec4f,
    // Unit vector toward the sun.
    sun_dir: vec4f,
    sun_color: vec4f,
    sky_top: vec4f,
    sky_horizon: vec4f,
    ambient_sky: vec4f,
    ambient_ground: vec4f,
    // Fog start, fog end, light bands (0 for smooth), shadow strength.
    fog: vec4f,
    // Cave light (rgb) and how far underground the camera is (a).
    cave: vec4f,
    // Seconds (x), night (y), point light count (z), cloud shadow count (w).
    misc: vec4f,
    // Per point light: position and reach, then color.
    lights: array<vec4f, 16>,
    // Per big cloud: middle (x, z), radius, base height, for the shadows they cast.
    clouds: array<vec4f, 16>,
    // Where a story has put the lights out: middle (x, z) and reach, zero for none.
    dark: array<vec4f, 8>,
    // Toward the middle of the rainbow's circle.
    bow: vec4f,
};

// 1 where lights burn as usual, 0 where a story has put them out.
fn lights_on(p: vec2f) -> f32 {
    var on = 1.0;
    for (var k = 0u; k < 8u; k++) {
        let d = g.dark[k];
        if d.z > 0.0 && distance(p, d.xy) < d.z {
            on = 0.0;
        }
    }
    return on;
}

@group(0) @binding(0) var<uniform> g: Globals;

struct VertIn {
    @location(0) pos: vec3f,
    @location(1) normal: vec4f,
    @location(2) color: vec4f,
    @location(3) m0: vec4f,
    @location(4) m1: vec4f,
    @location(5) m2: vec4f,
    @location(6) m3: vec4f,
    @location(7) tint: vec4f,
};

struct VertOut {
    @builtin(position) clip: vec4f,
    @location(0) world: vec3f,
    @location(1) normal: vec3f,
    @location(2) color: vec4f,
    @location(3) thin: f32,
    // Position in the mesh's own frame, so glints stay put on a moving body.
    @location(4) local: vec3f,
};

fn to_linear(c: vec3f) -> vec3f {
    return pow(c, vec3f(2.2));
}

fn sky(dir: vec3f) -> vec3f {
    let up = clamp(dir.y, 0.0, 1.0);
    var c = mix(g.sky_horizon.rgb, g.sky_top.rgb, pow(up, 0.55));
    let s = max(dot(dir, g.sun_dir.xyz), 0.0);
    let night = g.misc.y;
    // The sun by day, a big bright moon with a soft halo by night.
    let disc = pow(s, mix(900.0, 350.0, night));
    c += g.sun_color.rgb * (disc * mix(3.0, 6.0, night) + pow(s, 10.0) * mix(0.1, 0.25, night));
    // A faint glow low over the horizon at night, so the sky's edge stands out from the
    // darker sea.
    c += vec3f(0.03, 0.05, 0.09) * night * pow(1.0 - up, 6.0);
    if night > 0.01 {
        // Stars: a few bright cells on a grid over the sky, twinkling slowly.
        let cell = floor(dir * 160.0);
        let h = fract(sin(dot(cell, vec3f(12.9898, 78.233, 37.719))) * 43758.5453);
        let star = step(0.9965, h) * smoothstep(0.02, 0.25, dir.y);
        let twinkle = 0.7 + 0.3 * sin(g.misc.x * (1.0 + h * 3.0) + h * 40.0);
        c += vec3f(0.9, 0.93, 1.0) * star * twinkle * night * (1.0 - disc);
    }
    return c;
}

// A rainbow opposite the sun: bands 40 to 42 degrees round the point right away from
// it, red outside and violet in. Only in the sky itself, not in reflections.
fn rainbow(dir: vec3f) -> vec3f {
    let bow = g.sky_horizon.w;
    let theta = acos(clamp(dot(dir, g.bow.xyz), -1.0, 1.0));
    let t = (theta - 0.7) / 0.045;
    let band = smoothstep(-0.15, 0.1, t) * (1.0 - smoothstep(0.9, 1.15, t));
    let hue = vec3f(smoothstep(0.35, 1.0, t), 1.0 - abs(t - 0.5) * 1.8, 1.0 - smoothstep(0.0, 0.55, t));
    return clamp(hue, vec3f(0.0), vec3f(1.0)) * band * bow * 0.3 * smoothstep(0.0, 0.08, dir.y);
}

@vertex
fn vs_lit(v: VertIn) -> VertOut {
    let m = mat4x4f(v.m0, v.m1, v.m2, v.m3);
    var world = m * vec4f(v.pos, 1.0);
    // Grass blades lean with the wind, more toward their tips (normal w of 100/127).
    if abs(v.normal.w - 0.787) < 0.02 {
        let t = g.misc.x;
        let gust = sin(t * 1.3 + world.x * 0.05 + world.z * 0.03) * 0.5 + 0.5;
        let wave = sin(t * 2.7 + world.x * 0.4 + world.z * 0.25);
        world = vec4f(world.xyz + vec3f(0.6, 0.0, 0.35) * v.pos.y * (0.12 + 0.18 * gust + 0.06 * wave), 1.0);
    }
    // Boats afloat rise and fall on a long swell that rocks them gently side to side.
    if abs(v.normal.w - 0.157) < 0.02 {
        let t = g.misc.x;
        let swell = sin(t * 1.1 + world.x * 0.04 + world.z * 0.05) * 0.18;
        let rock = sin(t * 1.7 + world.x * 0.13 - world.z * 0.09) * 0.07;
        world = vec4f(world.x, world.y + swell + rock, world.z, 1.0);
    }
    var o: VertOut;
    o.clip = g.view_proj * world;
    // Roads (flag 30) draw nearer the camera along the line of sight, more so far off,
    // so they stay put on screen but coarse distant ground never pokes through them.
    if abs(v.normal.w - 0.236) < 0.02 {
        let to = g.cam_pos.xyz - world.xyz;
        let d = length(to);
        let pull = to / max(d, 0.001) * min(d * 0.004, 3.0);
        o.clip = g.view_proj * vec4f(world.xyz + pull, 1.0);
    }
    o.world = world.xyz;
    o.normal = (m * vec4f(v.normal.xyz, 0.0)).xyz;
    o.color = vec4f(to_linear(v.color.rgb) * v.tint.rgb, v.color.a * v.tint.a);
    o.thin = v.normal.w;
    o.local = v.pos;
    return o;
}

// Warm light from fires, crystals and breath, soft and wrapping so it reads as a glow.
fn point_lights(n: vec3f, world: vec3f) -> vec3f {
    var sum = vec3f(0.0);
    let count = i32(g.misc.z);
    for (var i = 0; i < count; i++) {
        let l = g.lights[i * 2];
        let to = l.xyz - world;
        let d = length(to);
        let f = clamp(1.0 - d / l.w, 0.0, 1.0);
        let wrap = clamp(dot(n, to / max(d, 0.001)) * 0.6 + 0.4, 0.0, 1.0);
        sum += g.lights[i * 2 + 1].rgb * (f * f * wrap);
    }
    return sum;
}

// How much sunlight reaches `world` past the big clouds overhead: each casts a soft
// round shadow along the sun's way onto whatever is under it.
fn cloud_light(world: vec3f) -> f32 {
    let s = g.sun_dir.xyz;
    var lit = 1.0;
    if s.y > 0.05 {
        let count = i32(g.misc.w);
        for (var i = 0; i < count; i++) {
            let c = g.clouds[i];
            let lift = c.w - world.y;
            if lift > 0.0 {
                let q = world.xz + s.xz / s.y * lift;
                let d = length(q - c.xy) / c.z;
                lit *= 1.0 - 0.4 * (1.0 - smoothstep(0.55, 1.0, d));
            }
        }
    }
    return lit;
}

// Lights a surface color the way every solid thing is lit, then adds distance haze.
fn shade(base: vec3f, n: vec3f, world: vec3f, day: f32, thin: f32) -> vec3f {
    var ndl = dot(n, g.sun_dir.xyz);
    // Thin surfaces glow when the sun is behind them.
    ndl = max(ndl, -ndl * 0.7 * thin);
    var light = clamp(ndl * 0.75 + 0.25, 0.0, 1.0) * cloud_light(world);
    let bands = g.fog.z;
    if bands > 0.5 {
        light = min(floor(light * bands + 0.35) / bands, 1.0);
    }
    // Cave glow is brighter on floors than roofs, so shapes still read.
    let glow = g.cave.rgb * (0.55 + 0.45 * (n.y * 0.5 + 0.5));
    let hemi = mix(glow, mix(g.ambient_ground.rgb, g.ambient_sky.rgb, n.y * 0.5 + 0.5), day);
    let col = base * (hemi + g.sun_color.rgb * light * day + point_lights(n, world));
    return haze_lit(col, world, day);
}

fn haze(col: vec3f, world: vec3f) -> vec3f {
    return haze_lit(col, world, 0.0);
}

// Distance haze. Seen from a cave, its own walls fade into the cave's gloom, but the
// world outside a mouth (lit by day) still fades into the sky, so it stays bright.
fn haze_lit(col: vec3f, world: vec3f, day: f32) -> vec3f {
    let to_frag = world - g.cam_pos.xyz;
    let gloom = g.cave.a * (1.0 - day);
    // Underground the far walls keep their rock colors much further, dimming into a
    // dusky brown rather than washing out pale.
    let fog = smoothstep(g.fog.x, g.fog.y, length(to_frag) / (1.0 + gloom * 1.5));
    let far = mix(sky(normalize(to_frag)), g.cave.rgb * 0.3 + vec3f(0.05, 0.035, 0.025), gloom);
    // Morning mist lying low over the water and the low ground, thicker off in the
    // distance, never in caves.
    let low = (1.0 - smoothstep(2.0, 40.0, world.y)) * smoothstep(60.0, 900.0, length(to_frag));
    let bank = smoothstep(0.45, 0.75, vnoise(world.xz * 0.0025 + vec2f(g.misc.x * 0.004, 0.0)));
    let mist = g.sky_top.w * low * bank * 0.6 * (1.0 - g.cave.a);
    let veiled = mix(col, g.sky_horizon.rgb * 0.5 + vec3f(0.42), mist);
    return mix(veiled, far, fog * (1.0 - gloom * 0.4));
}

fn facing(normal: vec3f, front: bool) -> vec3f {
    var n = normalize(normal);
    if !front {
        n = -n;
    }
    return n;
}

@fragment
fn fs_lit(i: VertOut, @builtin(front_facing) front: bool) -> @location(0) vec4f {
    // Lamps and lit windows glow on their own.
    if i.thin < -0.75 {
        let glow = mix(1.0, mix(0.45, 1.5, lights_on(i.world.xz)), g.misc.y);
        return vec4f(haze(i.color.rgb * mix(1.5, glow, g.misc.y), i.world), 1.0);
    }
    // Windows that light up at night, most of them, in warm colors.
    if i.thin < -0.25 {
        let day = clamp(i.color.a, 0.0, 1.0);
        let col = shade(i.color.rgb, facing(i.normal, front), i.world, day, 0.0);
        let h = hash2(floor(i.world.xz * 0.16) + vec2f(floor(i.world.y * 0.3), 0.0));
        let warm = mix(vec3f(1.0, 0.78, 0.42), vec3f(1.0, 0.9, 0.65), h);
        let on = step(0.6, h) * g.misc.y * lights_on(i.world.xz);
        return vec4f(mix(col, haze(warm * (0.55 + 0.45 * h), i.world), on), 1.0);
    }
    // Vertex alpha is how much daylight reaches here. Deep in caves a soft glow takes over.
    let day = clamp(i.color.a, 0.0, 1.0);
    let n = facing(i.normal, front);
    // Dragon scales (flag 90) and gold (flag 70).
    if abs(i.thin - 0.709) < 0.03 || abs(i.thin - 0.551) < 0.03 {
        return vec4f(gleam(i.color.rgb, n, i.world, i.local, day, i.thin < 0.63), 1.0);
    }
    return vec4f(shade(i.color.rgb, n, i.world, day, i.thin), 1.0);
}

// Scales with a soft gloss and glints that twinkle as the facets turn, or gold that also
// mirrors the sky.
fn gleam(base: vec3f, n: vec3f, world: vec3f, local: vec3f, day: f32, metal: bool) -> vec3f {
    let m = select(0.0, 1.0, metal);
    var col = shade(base * mix(1.0, 0.75, m), n, world, day, 0.0);
    let v = normalize(g.cam_pos.xyz - world);
    let nh = max(dot(n, normalize(v + g.sun_dir.xyz)), 0.0);
    let tint = mix(vec3f(1.0), base * 1.6, m);
    let spec = pow(nh, mix(36.0, 60.0, m)) * mix(0.4, 1.3, m);
    let r = fract(sin(dot(floor(local * 24.0), vec3f(12.9898, 78.233, 37.719))) * 43758.5453);
    let twinkle = pow(max(sin(g.misc.x * (1.5 + r * 2.5) + r * 60.0 + dot(v, vec3f(9.0, 13.0, 7.0))), 0.0), 8.0);
    let glint = step(0.93, r) * pow(nh, 12.0) * twinkle * 3.0;
    col += g.sun_color.rgb * tint * (spec + glint) * day;
    let fres = pow(1.0 - max(dot(n, v), 0.0), 3.0);
    let env = sky(reflect(-v, n));
    col += mix(env * fres * 0.3, env * base * 0.55, m) * day;
    return col;
}

fn hash2(p: vec2f) -> f32 {
    return fract(sin(dot(p, vec2f(127.1, 311.7))) * 43758.5453);
}

// Smooth value noise in [0, 1].
fn vnoise(p: vec2f) -> f32 {
    let c = floor(p);
    let f = p - c;
    let u = f * f * (3.0 - 2.0 * f);
    let a = mix(hash2(c), hash2(c + vec2f(1.0, 0.0)), u.x);
    let b = mix(hash2(c + vec2f(0.0, 1.0)), hash2(c + vec2f(1.0, 1.0)), u.x);
    return mix(a, b, u.y);
}

// Ground detail without textures: grass in drifts of lush and dry with tufts up close,
// waves of wind and wildflowers, rock in layered strata, sand in faint ripples.
fn ground_detail(base: vec3f, world: vec3f) -> vec3f {
    let w = world.xz;
    let hi = max(base.r, max(base.g, base.b));
    let lo = min(base.r, min(base.g, base.b));
    let grass = smoothstep(0.02, 0.12, base.g - max(base.r, base.b));
    // Snow is pale too, but far brighter than any rock.
    let snow = smoothstep(0.62, 0.75, lo);
    let rock = (1.0 - smoothstep(0.28, 0.4, (hi - lo) / max(hi, 0.001))) * (1.0 - grass) * (1.0 - snow);
    let sand = smoothstep(0.1, 0.2, base.r - base.b) * (1.0 - grass);
    var c = base;
    // Drifts: big soft patches, then smaller ones inside them.
    let drift = vnoise(w * 0.011) * 0.6 + vnoise(w * 0.047 + 13.0) * 0.4;
    let lush = base * vec3f(0.72, 0.88, 0.78);
    let dry = base * vec3f(1.12, 1.04, 0.8);
    c = mix(c, mix(lush, dry, smoothstep(0.25, 0.75, drift)), grass * 0.85);
    // Tufts and clover up close, fading before they would shimmer.
    let tp = w * 0.9;
    let near = 1.0 - smoothstep(0.06, 0.2, max(fwidth(tp.x), fwidth(tp.y)));
    let tuft = hash2(floor(tp));
    c *= 1.0 + (tuft - 0.5) * 0.12 * near * grass;
    // Wind sweeping over the grass: gusts roll across in pale waves, as the blades bend
    // and show their lighter undersides.
    let t = g.misc.x;
    let gp = w * vec2f(0.016, 0.011) - vec2f(t * 0.09, t * 0.06);
    let gust = vnoise(gp) * 0.65 + vnoise(gp * 2.9 + 4.0) * 0.35;
    let sheen = smoothstep(0.52, 0.82, gust) * grass * (1.0 - snow);
    c = mix(c, c * vec3f(1.14, 1.12, 0.94) + vec3f(0.018, 0.022, 0.0), sheen * 0.75);
    // Wildflowers dotted through some meadows, one to a cell, only up close.
    let fp = w * 0.6;
    let fw = max(fwidth(fp.x), fwidth(fp.y));
    let fc = floor(fp);
    let fh = hash2(fc + 91.0);
    let spot = vec2f(hash2(fc + 3.0), hash2(fc + 5.0)) * 0.6 + 0.2;
    let petal_r = 0.09 + fw * 0.5;
    let dot_in = 1.0 - smoothstep(petal_r, petal_r + 0.05, length(fract(fp) - spot));
    let patchy = smoothstep(0.48, 0.72, vnoise(w * 0.021 + 40.0));
    let bloom = dot_in * step(0.5, fh) * patchy * grass * (1.0 - smoothstep(0.03, 0.08, fw));
    let kind = hash2(fc + 17.0);
    var petal = vec3f(0.92, 0.9, 0.82);
    if kind > 0.45 {
        petal = vec3f(0.95, 0.72, 0.08);
    }
    if kind > 0.75 {
        petal = vec3f(0.62, 0.36, 0.9);
    }
    if kind > 0.9 {
        petal = vec3f(0.95, 0.42, 0.55);
    }
    c = mix(c, petal, bloom * 0.9);
    // Rock in layers like sediment: each layer its own shade, from cool gray to warm
    // ochre, wavering with the noise, with a dark seam between layers.
    let y = world.y + (vnoise(w * 0.015) - 0.5) * 14.0;
    let layer = floor(y / 9.0);
    let lh = hash2(vec2f(layer, 7.0));
    let shade = mix(vec3f(0.86, 0.88, 0.94), vec3f(1.16, 1.04, 0.86), lh) * (0.9 + 0.2 * hash2(vec2f(layer, 3.0)));
    let seam = 1.0 - smoothstep(0.0, 0.06, fract(y / 9.0));
    c = mix(c, c * shade * (1.0 - seam * 0.14), rock);
    c = mix(c, c * vec3f(1.06, 1.0, 0.92), vnoise(w * 0.03 + 5.0) * rock * 0.4);
    // Sand: wind ripples.
    c *= 1.0 + sin(w.x * 0.9 + w.y * 0.4 + vnoise(w * 0.1) * 3.0) * 0.04 * sand * near;
    return c;
}

const DEEP_WATER: vec3f = vec3f(0.035, 0.2, 0.36);

// Terrain. Its normal's w holds the height above the nearest water in tenths of 12.7m,
// so the seabed fades into blue with depth and foam laps at the waterline.
@fragment
fn fs_ground(i: VertOut, @builtin(front_facing) front: bool) -> @location(0) vec4f {
    let day = clamp(i.color.a, 0.0, 1.0);
    let n = facing(i.normal, front);
    var base = ground_detail(i.color.rgb, i.world);
    let above = i.thin * 12.7;
    // Caustics' grid, measured out here before any branch for WebGPU.
    let cp = i.world.xz * 0.32;
    let cpx = max(fwidth(cp.x), fwidth(cp.y));
    if above < 12.6 {
        let t = g.misc.x;
        // Wet ground just above the water, darker.
        base *= 1.0 - 0.18 * (1.0 - smoothstep(0.0, 0.9, above)) * step(0.0, above);
        // Under water the ground sinks into blue.
        let under = clamp(-above / 9.0, 0.0, 1.0);
        base = mix(base, DEEP_WATER, sqrt(under) * 0.85);
        // Sunlight focused by the waves wanders over shallow beds in a bright net, where
        // two drifting noises cross.
        let a = vnoise(cp + vec2f(t * 0.31, t * 0.17));
        let b = vnoise(cp * 1.37 + vec2f(-t * 0.23, t * 0.29) + 7.0);
        let net = 1.0 - smoothstep(0.0, 0.07 + cpx * 0.5, abs(a - b));
        let shallow = step(above, 0.0) * (1.0 - smoothstep(0.5, 6.0, -above));
        let sharp = 1.0 - smoothstep(0.15, 0.5, cpx);
        base += vec3f(0.22, 0.3, 0.26) * net * shallow * sharp * day;
        // A band of foam that laps up and down the shore.
        let wob = hash2(floor(i.world.xz * 0.15)) * 6.28;
        let lap = 0.18 * sin(t * 1.1 + wob) + 0.12 * sin(t * 0.37 + i.world.x * 0.03);
        let foam = 1.0 - smoothstep(0.08, 0.3, abs(above - lap - 0.05));
        base = mix(base, vec3f(0.95, 0.97, 1.0), foam * 0.85);
        // Coral in warm sandy shallows: patches of pink, orange and violet under the
        // water, fading out deeper down.
        let sandy = smoothstep(0.1, 0.2, i.color.r - i.color.b) * (1.0 - smoothstep(0.02, 0.1, i.color.g - i.color.r));
        let reef = smoothstep(0.55, 0.75, vnoise(i.world.xz * 0.06 + 70.0)) * sandy;
        let shelf = smoothstep(-0.8, -1.5, above) * (1.0 - smoothstep(-6.0, -10.0, above));
        let tint = mix(vec3f(0.85, 0.35, 0.45), vec3f(0.95, 0.55, 0.2), vnoise(i.world.xz * 0.15));
        let coral = mix(tint, vec3f(0.55, 0.35, 0.8), smoothstep(0.62, 0.7, vnoise(i.world.xz * 0.35 + 30.0)));
        base = mix(base, coral * 0.75, reef * shelf * 0.7);
    }
    // Against sea cliffs the swell surges white up the rock every few seconds, each
    // stretch of cliff in its own time. Measured from the sea itself, as a tall cliff
    // face's own height over the water is too coarse.
    {
        let t = g.misc.x;
        let wob = hash2(floor(i.world.xz * 0.15)) * 6.28;
        let steep = 1.0 - smoothstep(0.35, 0.6, abs(n.y));
        let beat = pow(max(sin(t * 0.95 + wob * 1.7), 0.0), 3.0);
        let surge = 1.2 + 6.5 * beat * (0.6 + 0.4 * vnoise(i.world.xz * 0.05 + t * 0.2));
        let y = i.world.y;
        let froth = step(-0.3, y) * (1.0 - smoothstep(surge * 0.7, surge, y)) * step(0.6, day);
        let lace = smoothstep(0.35, 0.6, vnoise(i.world.xz * 0.4 + vec2f(0.0, y * 0.7) + t));
        base = mix(base, vec3f(0.96, 0.98, 1.0), froth * steep * mix(0.55, 0.95, lace));
    }
    // Deep in caves, walls get flowstone streaks and veins of glowing ore, except the
    // pale tiles of built tunnels and halls.
    let tiled = step(0.6, min(i.color.r, min(i.color.g, i.color.b)));
    let under = (1.0 - smoothstep(0.08, 0.35, day)) * (1.0 - tiled);
    var ore = vec3f(0.0);
    // Flat coordinates on whichever plane the wall faces most, wrapped so the noise keeps
    // its precision far from the origin. Screen derivatives must be taken out here, before
    // any branch, for WebGPU.
    let w = i.world;
    var uv = select(select(w.xz, vec2f(w.x, w.y), abs(n.y) < 0.7), vec2f(w.z, w.y), abs(n.x) > abs(n.z));
    uv = uv - floor(uv / 1024.0) * 1024.0;
    let vein_px = max(fwidth(uv.x), fwidth(uv.y)) * 0.03;
    let roof_p = w.xz * 0.45 + vec2f(w.y * 0.17, w.y * 0.11);
    let roof_px = max(fwidth(roof_p.x), fwidth(roof_p.y));
    if under > 0.01 && n.y < 0.6 {
        // Flowstone: pale streaks running down the walls.
        let streak = vnoise(vec2f(uv.x * 0.12, uv.y * 0.008 + uv.x * 0.003));
        base = mix(base, base * vec3f(1.18, 1.12, 1.02), smoothstep(0.65, 0.85, streak) * under * 0.6);
        // Veins: thin winding lines where ridged noise crosses its middle, fading out
        // with distance rather than growing fat.
        let q = uv * 0.03 + vec2f(vnoise(uv * 0.04) * 1.2, 0.0);
        let ridge = abs(vnoise(q) * 2.0 - 1.0);
        let near = 1.0 - smoothstep(0.03, 0.12, vein_px);
        let vein = (1.0 - smoothstep(0.012, 0.03, ridge)) * near;
        let patchy = smoothstep(0.45, 0.65, vnoise(uv * 0.008 + 31.0));
        let tint = normalize(g.cave.rgb + vec3f(0.02)) * 0.9 + vec3f(0.08);
        let pulse = 0.7 + 0.3 * sin(g.misc.x * 0.6 + vnoise(uv * 0.01) * 12.0);
        ore = tint * vein * patchy * under * pulse * 0.45;
        base = mix(base, base * 0.75, vein * patchy * under);
    }
    var col = shade(base, n, i.world, day, 0.0);
    let fog_ore = smoothstep(g.fog.x, g.fog.y, length(i.world - g.cam_pos.xyz));
    col += ore * (1.0 - fog_ore);
    // Glowworms speckle cave roofs like a sky full of tiny blue-green stars.
    let roof = under * smoothstep(0.25, 0.7, -n.y);
    if roof > 0.01 {
        let p = roof_p;
        let px = roof_px;
        let cell = floor(p);
        let h = hash2(cell);
        let spot = vec2f(hash2(cell + 17.0), hash2(cell + 31.0)) * 0.6 + 0.2;
        let d = length(fract(p) - spot);
        // They gather in colonies, with bare rock between.
        let colony = smoothstep(0.45, 0.7, vnoise(i.world.xz * 0.018 + vec2f(i.world.y * 0.01, 3.0)));
        let speck = (1.0 - smoothstep(0.04 + px * 0.5, 0.1 + px, d)) * step(0.62, h) * colony;
        let far = 1.0 - smoothstep(0.15, 0.5, px);
        let twinkle = 0.65 + 0.35 * sin(g.misc.x * (1.1 + h * 1.5) + h * 40.0);
        let fog = smoothstep(g.fog.x, g.fog.y, length(i.world - g.cam_pos.xyz));
        col += vec3f(0.35, 0.95, 0.9) * speck * twinkle * roof * far * (1.0 - fog) * 1.4;
    }
    return vec4f(col, 1.0);
}

// Faceted, gently rocking water. The surface is cut into a grid of triangles, each tilted
// by its own slow wobble, and the grid drifts with the flow.
@fragment
fn fs_water(i: VertOut) -> @location(0) vec4f {
    let t = g.misc.x;
    let flow = vec2f(i.normal.x, i.normal.z) * 20.0;
    let day = clamp(i.normal.y, 0.0, 1.0);
    let depth = clamp(i.color.a, 0.0, 1.0);
    let to_cam = g.cam_pos.xyz - i.world;
    let dist = length(to_cam);
    let v = to_cam / dist;
    let falls = i.thin > 0.5;
    // Facet grid, measured before branching so its screen size is known. Facets smaller
    // than a few pixels would only shimmer, so they flatten out.
    let size = 6.0;
    let p = (i.world.xz - flow * t) / size;
    let px = max(fwidth(p.x), fwidth(p.y));
    let calm = (1.0 - smoothstep(60.0, 380.0, dist)) * (1.0 - smoothstep(0.08, 0.3, px));

    var col: vec3f;
    var alpha: f32;
    if falls {
        // Falling water: long bright streaks racing down.
        let across = floor((i.world.x + i.world.z) * 0.7);
        let streak = fract(i.world.y * 0.02 + t * 0.9 + hash2(vec2f(across, 3.0)));
        let white = smoothstep(0.55, 0.95, streak) * 0.6 + 0.35;
        let base = mix(i.color.rgb * 1.3, vec3f(0.97, 0.99, 1.0), white);
        col = shade(base, normalize(vec3f(v.x, 0.4, v.z)), i.world, mix(0.6, 1.0, day), 0.0);
        alpha = 0.92;
    } else {
        let cell = floor(p);
        let f = p - cell;
        let upper = step(f.y, f.x);
        let id = cell * 2.0 + vec2f(upper, 0.0);
        let h = hash2(id);
        let k = hash2(id + 17.0);
        let wobble = vec2f(sin(t * (1.1 + h) + h * 6.28), cos(t * (0.9 + k) + k * 6.28));
        let tilt = wobble * (0.07 + 0.05 * length(flow) / 8.0) * calm;
        // A slow swell under the facets.
        let swell = vec2f(sin(i.world.x * 0.013 + t * 0.6), cos(i.world.z * 0.011 + t * 0.5)) * 0.03 * (0.3 + 0.7 * calm);
        let n = normalize(vec3f(tilt.x + swell.x, 1.0, tilt.y + swell.y));

        let shallow = vec3f(0.55, 0.92, 0.85);
        let base = mix(shallow, i.color.rgb * 0.75, smoothstep(0.0, 0.7, depth));
        var ndl = clamp(dot(n, g.sun_dir.xyz), 0.0, 1.0);
        // Cave water glows a little on its own, like lit crystal.
        let glow = base * 0.35 + vec3f(0.02, 0.12, 0.14);
        let sun = g.sun_color.rgb * (0.35 + 0.5 * ndl) * day * cloud_light(i.world);
        let lit = base * (mix(g.cave.rgb * 0.9, g.ambient_sky.rgb, day) + sun) + glow * (1.0 - day);
        let fres = pow(1.0 - clamp(dot(n, v), 0.0, 1.0), 4.0) * mix(0.35, 1.0, day);
        let refl = mix(g.cave.rgb * 0.5 + vec3f(0.0, 0.08, 0.1), sky(reflect(-v, n)), day);
        col = mix(lit, refl, fres * 0.75);
        // Sun glints, in hard little sparkles.
        let spec = pow(clamp(dot(reflect(-g.sun_dir.xyz, n), v), 0.0, 1.0), 120.0);
        col += g.sun_color.rgb * step(0.4, spec) * 1.2 * day * calm;
        // The moon lays a long shimmering path across the water at night.
        let night = g.misc.y;
        let moon = pow(clamp(dot(reflect(-g.sun_dir.xyz, n), v), 0.0, 1.0), 18.0);
        col += g.sun_color.rgb * moon * 0.7 * night * day;
        // Swell lines rolling in to the shore and breaking white in the shallows. Rivers
        // and pools are too small for surf, so only wide still water gets it.
        let still = 1.0 - smoothstep(0.5, 2.0, length(flow));
        let shore = (1.0 - smoothstep(0.04, 0.3, depth)) * still;
        let roll = fract(depth * 7.0 - t * 0.22 + vnoise(i.world.xz * 0.012) * 0.8);
        let crest = smoothstep(0.82, 0.96, roll) * (1.0 - smoothstep(0.96, 1.0, roll)) * shore;
        col = mix(col, vec3f(0.93, 0.97, 1.0), crest * 0.5 * mix(0.4, 1.0, day));
        // Whitecaps out on open water, little patches that come and go with the wind.
        let wind = i.world.xz * vec2f(0.16, 0.42) + vec2f(t * 0.5, t * 0.2);
        let gust = vnoise(wind) * smoothstep(0.45, 0.75, vnoise(i.world.xz * 0.02 + t * 0.03));
        let open = smoothstep(0.35, 0.8, depth) * still * (1.0 - smoothstep(80.0, 450.0, dist));
        col = mix(col, vec3f(0.95, 0.98, 1.0), smoothstep(0.8, 0.88, gust) * open * 0.5 * day);
        // Running water: streaks of light and foam drawn out along the flow, racing
        // downstream at its speed.
        let speed = length(flow);
        let along = flow / max(speed, 0.001);
        let q = vec2f(dot(i.world.xz, vec2f(-along.y, along.x)) * 0.3, dot(i.world.xz, along) * 0.035 - t * speed * 0.035);
        let streak = vnoise(q) * vnoise(q * vec2f(1.7, 1.3) + 9.0);
        let running = smoothstep(0.3, 1.5, speed) * (1.0 - smoothstep(150.0, 600.0, dist));
        col = mix(col, vec3f(0.9, 0.96, 1.0), smoothstep(0.3, 0.55, streak) * running * 0.5 * mix(0.4, 1.0, day));
        // Foam where the water thins out at its edges and where it rushes.
        let fizz = hash2(floor(i.world.xz * 0.5) + floor(t * 3.0));
        let edge = 1.0 - smoothstep(0.0, 0.08, depth);
        let rush = smoothstep(4.0, 9.0, length(flow)) * step(0.7, fizz);
        col = mix(col, vec3f(0.95, 0.98, 1.0), max(edge * 0.6, rush * 0.7));
        col = haze(col, i.world);
        // At night the open water stays a shade darker than the sky over it, out to the
        // horizon, so the two never run together.
        col *= 1.0 - 0.38 * night * day * smoothstep(0.3, 0.8, depth);
        alpha = mix(0.35, 0.95, smoothstep(0.0, 1.0, depth));
        alpha = max(alpha, fres * 0.9) * mix(0.0, 1.0, smoothstep(0.0, 0.04, depth) + 0.4);
        alpha = clamp(alpha, 0.0, 1.0);
    }
    return vec4f(col, alpha);
}

// The sign font, filled in from the sim's own when the shader is built: space to
// underscore, 3 by 5 bits each, the top left the highest.
var<private> GLYPHS: array<u32, 64> = array<u32, 64>(GLYPH_TABLE);

// Painted letters: flat quads that carry their corner (normal x, y) and character
// (normal z) instead of a normal, cut to the letter's cells. Normal w is 1 for letters
// that light up at night and 2 for ones that always glow.
@fragment
fn fs_letter(i: VertOut) -> @location(0) vec4f {
    // The face's normal from screen derivatives, taken before any branch for WebGPU.
    var n = normalize(cross(dpdx(i.world), dpdy(i.world)));
    if dot(n, g.cam_pos.xyz - i.world) < 0.0 {
        n = -n;
    }
    let u = clamp(i.normal.x, 0.0, 0.999);
    let v = clamp(i.normal.y, 0.0, 0.999);
    let code = min(u32(round(i.normal.z * 127.0)), 63u);
    let cell = u32((1.0 - v) * 5.0) * 3u + u32(u * 3.0);
    if ((GLYPHS[code] >> (14u - cell)) & 1u) == 0u {
        discard;
    }
    let day = clamp(i.color.a, 0.0, 1.0);
    let glow = haze(i.color.rgb * 1.5, i.world);
    if i.thin > 0.012 {
        return vec4f(glow, 1.0);
    }
    let col = shade(i.color.rgb, n, i.world, day, 0.0);
    if i.thin > 0.004 {
        return vec4f(mix(col, glow, g.misc.y), 1.0);
    }
    return vec4f(col, 1.0);
}

@fragment
fn fs_shadow(i: VertOut) -> @location(0) vec4f {
    // Red carries the fade from center to rim, alpha the per-instance strength.
    let fade = smoothstep(0.0, 0.6, i.color.r);
    return vec4f(0.16, 0.12, 0.3, fade * i.color.a * g.fog.w);
}

@fragment
fn fs_glow(i: VertOut) -> @location(0) vec4f {
    return vec4f(i.color.rgb * i.color.a, i.color.a);
}

struct SkyOut {
    @builtin(position) clip: vec4f,
    @location(0) ndc: vec2f,
};

@vertex
fn vs_sky(@builtin(vertex_index) idx: u32) -> SkyOut {
    let uv = vec2f(f32((idx << 1u) & 2u), f32(idx & 2u));
    let ndc = uv * 2.0 - 1.0;
    var o: SkyOut;
    o.clip = vec4f(ndc, 0.0, 1.0);
    o.ndc = ndc;
    return o;
}

@fragment
fn fs_sky(i: SkyOut) -> @location(0) vec4f {
    let far = g.inv_view_proj * vec4f(i.ndc, 1.0, 1.0);
    let dir = normalize(far.xyz / far.w - g.cam_pos.xyz);
    // Deep in a cave, past the walls drawn, is more rock in the gloom, not daylight.
    return vec4f(mix(sky(dir) + rainbow(dir), g.cave.rgb * 0.5, g.sun_dir.w), 1.0);
}
