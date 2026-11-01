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
};

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
};

fn to_linear(c: vec3f) -> vec3f {
    return pow(c, vec3f(2.2));
}

fn sky(dir: vec3f) -> vec3f {
    let up = clamp(dir.y, 0.0, 1.0);
    var c = mix(g.sky_horizon.rgb, g.sky_top.rgb, pow(up, 0.55));
    let s = max(dot(dir, g.sun_dir.xyz), 0.0);
    c += g.sun_color.rgb * (pow(s, 900.0) * 3.0 + pow(s, 10.0) * 0.1);
    return c;
}

@vertex
fn vs_lit(v: VertIn) -> VertOut {
    let m = mat4x4f(v.m0, v.m1, v.m2, v.m3);
    let world = m * vec4f(v.pos, 1.0);
    var o: VertOut;
    o.clip = g.view_proj * world;
    o.world = world.xyz;
    o.normal = (m * vec4f(v.normal.xyz, 0.0)).xyz;
    o.color = vec4f(to_linear(v.color.rgb) * v.tint.rgb, v.color.a * v.tint.a);
    o.thin = v.normal.w;
    return o;
}

@fragment
fn fs_lit(i: VertOut, @builtin(front_facing) front: bool) -> @location(0) vec4f {
    var n = normalize(i.normal);
    if !front {
        n = -n;
    }
    var ndl = dot(n, g.sun_dir.xyz);
    // Thin surfaces glow when the sun is behind them.
    ndl = max(ndl, -ndl * 0.7 * i.thin);
    var light = clamp(ndl * 0.75 + 0.25, 0.0, 1.0);
    let bands = g.fog.z;
    if bands > 0.5 {
        light = min(floor(light * bands + 0.35) / bands, 1.0);
    }
    // Vertex alpha is how much daylight reaches here. Deep in caves a soft glow takes over.
    let day = clamp(i.color.a, 0.0, 1.0);
    // Cave glow is brighter on floors than roofs, so shapes still read.
    let glow = g.cave.rgb * (0.55 + 0.45 * (n.y * 0.5 + 0.5));
    let hemi = mix(glow, mix(g.ambient_ground.rgb, g.ambient_sky.rgb, n.y * 0.5 + 0.5), day);
    var col = i.color.rgb * (hemi + g.sun_color.rgb * light * day);
    let to_frag = i.world - g.cam_pos.xyz;
    let fog = smoothstep(g.fog.x, g.fog.y, length(to_frag));
    let haze = mix(sky(normalize(to_frag)), g.cave.rgb * 0.5, g.cave.a);
    col = mix(col, haze, fog * (1.0 - g.cave.a * 0.4));
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
    return vec4f(sky(dir), 1.0);
}
