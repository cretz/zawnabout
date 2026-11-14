// The heads-up display: flat triangles in clip space, colored and textured from the atlas
// (the map, the font and a white cell for plain shapes).

struct In {
    @location(0) pos: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
}

struct Out {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
}

@group(0) @binding(0) var atlas: texture_2d<f32>;
@group(0) @binding(1) var soft: sampler;

@vertex
fn vs_hud(v: In) -> Out {
    var o: Out;
    o.clip = vec4<f32>(v.pos, 0.0, 1.0);
    o.uv = v.uv;
    o.color = v.color;
    return o;
}

@fragment
fn fs_hud(o: Out) -> @location(0) vec4<f32> {
    return textureSample(atlas, soft, o.uv) * o.color;
}
