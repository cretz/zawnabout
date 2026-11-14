//! The forward renderer: a sky pass, flat-shaded lit meshes and soft blob shadows, all
//! drawn into a 4x MSAA target.

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3, Vec4};
use wgpu::util::DeviceExt;

use crate::mesh::{MeshBuilder, Vertex};

pub const SAMPLES: u32 = 4;

/// Codes in `Renderer::hide` that leave out a whole part of the picture.
pub const PART_SKY: u32 = 9_000_001;
pub const PART_STATICS: u32 = 9_000_002;
pub const PART_DYNAMIC: u32 = 9_000_003;
pub const PART_FX: u32 = 9_000_004;
pub const PART_SHADOWS: u32 = 9_000_005;
pub const PART_HUD: u32 = 9_000_006;
/// Point lights per frame.
pub const MAX_LIGHTS: usize = 8;
const DEPTH: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MeshId(pub u32);

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Instance {
    pub model: [[f32; 4]; 4],
    pub tint: [f32; 4],
}

impl Instance {
    pub fn new(model: Mat4) -> Self {
        Self::tinted(model, Vec4::ONE)
    }

    pub fn tinted(model: Mat4, tint: Vec4) -> Self {
        Self {
            model: model.to_cols_array_2d(),
            tint: tint.to_array(),
        }
    }
}

/// Lighting and atmosphere, all in linear color.
#[derive(Clone, Copy, Debug)]
pub struct Lighting {
    pub sun_dir: Vec3,
    pub sun_color: Vec3,
    pub sky_top: Vec3,
    pub sky_horizon: Vec3,
    pub ambient_sky: Vec3,
    pub ambient_ground: Vec3,
    pub fog_start: f32,
    pub fog_end: f32,
    pub bands: f32,
    pub shadow: f32,
    /// Light deep in caves, where no sun reaches.
    pub cave_ambient: Vec3,
    /// How far underground the camera is, 0 to 1. Turns the far haze dark.
    pub underground: f32,
    /// How much the sky gives way to cave gloom, 0 to 1: deep in a cave, past the walls
    /// it draws, it is dark rock, not daylight. Off when looking out of a mouth.
    pub sky_gloom: f32,
    /// Day 0 to night 1: stars and a moon in the sky, lit windows.
    pub night: f32,
    /// Morning mist lying over low ground and water, 0 to 1.
    pub valley_fog: f32,
    /// A rainbow in the sky opposite the sun, 0 to 1.
    pub rainbow: f32,
    /// Toward the middle of the rainbow's circle, which sits so one end touches down on
    /// the rainbow's end in the far corner of the sea.
    pub bow: Vec3,
}

/// Everything one frame draws.
#[derive(Clone, Debug)]
pub struct Frame {
    pub view_proj: Mat4,
    pub cam_pos: Vec3,
    pub lighting: Lighting,
    pub draws: Vec<(MeshId, Instance)>,
    /// Terrain chunks, drawn where they are, with bounding spheres for culling.
    pub terrain: Vec<(MeshId, Vec3, f32)>,
    /// Static batches farther than this (times their reach) are skipped.
    pub prop_range: f32,
    /// Static batches nearer than this use their full mesh, farther ones their far mesh.
    pub detail_range: f32,
    /// Deep underground nothing outside can be seen except through openings, where sky
    /// alone looks right, so all statics (sea, lakes, props, clouds) are skipped.
    pub deep: bool,
    /// Cave insides are drawn this frame.
    pub caves: bool,
    pub shadows: Vec<Instance>,
    /// Soft glowing rings, such as landing markers.
    pub rings: Vec<Instance>,
    /// Glowing see-through things in the world, such as fire and light shafts. Hidden
    /// behind solid things but never hiding anything.
    pub fx: Vec<(MeshId, Instance)>,
    /// Point lights: position, reach and color. At most [`MAX_LIGHTS`] are used.
    pub lights: Vec<(Vec3, f32, Vec3)>,
    /// See-through water, drawn after everything solid, with bounding spheres.
    pub water: Vec<(MeshId, Vec3, f32)>,
    /// Seconds, for moving water.
    pub time: f32,
    /// Big clouds that shade the ground: middle (x, z), radius and base height. At most
    /// [`crate::weather::SHADOWS`] are used.
    pub clouds: Vec<Vec4>,
    /// Where a story has put the lights out: middle (x, z) and reach. At most [`DARK`].
    pub dark: Vec<Vec4>,
    /// The heads-up display, drawn over everything.
    pub hud: Vec<crate::hud::HudVertex>,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Globals {
    view_proj: [[f32; 4]; 4],
    inv_view_proj: [[f32; 4]; 4],
    cam_pos: [f32; 4],
    sun_dir: [f32; 4],
    sun_color: [f32; 4],
    sky_top: [f32; 4],
    sky_horizon: [f32; 4],
    ambient_sky: [f32; 4],
    ambient_ground: [f32; 4],
    fog: [f32; 4],
    cave: [f32; 4],
    misc: [f32; 4],
    /// Per light: position and reach, then color.
    lights: [[f32; 4]; MAX_LIGHTS * 2],
    /// Cloud shadows: middle (x, z), radius, base height.
    clouds: [[f32; 4]; crate::weather::SHADOWS],
    /// Dark regions: middle (x, z) and reach, zero reach for none.
    dark: [[f32; 4]; DARK],
    /// Toward the middle of the rainbow's circle.
    bow: [f32; 4],
}

/// Most dark regions a frame passes to the shaders.
pub const DARK: usize = 8;

/// A mesh on its way to the GPU a slice at a time.
pub struct Sending {
    buf: wgpu::Buffer,
    bytes: Vec<u8>,
    sent: usize,
    count: u32,
    lo: Vec3,
    hi: Vec3,
}

impl Sending {
    /// Bytes still to send.
    pub fn left(&self) -> usize {
        self.bytes.len() - self.sent
    }

    /// Its middle and how far it reaches from there.
    pub fn sphere(&self) -> (Vec3, f32) {
        (
            (self.lo + self.hi) * 0.5,
            (self.hi - self.lo).length() * 0.5,
        )
    }
}

struct GpuMesh {
    buf: wgpu::Buffer,
    count: u32,
    /// Its bounding box.
    lo: Vec3,
    hi: Vec3,
}

struct StaticBatch {
    mesh: MeshId,
    buf: wgpu::Buffer,
    count: u32,
    cull: Cull,
    /// The box round every instance, which distances are measured to.
    lo: Vec3,
    hi: Vec3,
}

/// How a static batch is culled and simplified with distance.
#[derive(Clone, Copy, Debug)]
pub struct Cull {
    /// Bounding sphere of every instance.
    pub center: Vec3,
    pub radius: f32,
    /// Drawn out to this many times the frame's prop range.
    pub reach: f32,
    /// A cheaper mesh for beyond the frame's detail range times `detail`.
    pub far: Option<MeshId>,
    pub detail: f32,
    /// Under ground: drawn only with the cave insides, even deep down.
    pub cave: bool,
    /// Part of a group of tiles that draws as one merged mesh when far away.
    pub group: Option<Group>,
}

/// A group of tiles: within `range` times the frame's detail range its tiles draw one by
/// one, beyond it the group's merged mesh draws instead.
#[derive(Clone, Copy, Debug)]
pub struct Group {
    pub center: Vec3,
    pub radius: f32,
    pub range: f32,
    /// This is the merged mesh, not one of the tiles.
    pub merged: bool,
}

impl Cull {
    pub const ALWAYS: Cull = Cull {
        center: Vec3::ZERO,
        radius: f32::INFINITY,
        reach: 1.0,
        far: None,
        detail: 1.0,
        cave: false,
        group: None,
    };
}

/// The sign font for the shader: each printable character from space to underscore as
/// 15 bits, 3 across and 5 down, the top left the highest bit.
fn glyph_table() -> String {
    (32u8..96)
        .map(|c| {
            let rows = zawnabout_sim::landmark::glyph(c as char);
            let bits = rows
                .iter()
                .fold(0u32, |acc, r| (acc << 3) | (*r as u32 & 7));
            format!("{bits}u")
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// What the last frame drew, for judging cost on slow devices.
#[derive(Clone, Debug, Default)]
pub struct Stats {
    pub draws: u32,
    /// Draws by group: static batches, terrain chunks and water.
    pub draws_static: u32,
    pub draws_terrain: u32,
    pub draws_water: u32,
    pub triangles: u64,
    pub terrain: u64,
    /// Instanced static props, clouds included.
    pub props: u64,
    pub water: u64,
    pub meshes: u32,
    /// The costliest instanced draws this frame: mesh, instances and triangles.
    pub heaviest: Vec<(u32, u32, u64)>,
}

pub struct Renderer {
    pub stats: Stats,
    /// Meshes left out of the lit draws, for finding what something on screen is. The
    /// `PART_*` codes leave out a whole part, for timing it.
    pub hide: Vec<u32>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    /// Always an sRGB format. Render targets are viewed with it.
    pub format: wgpu::TextureFormat,
    size: (u32, u32),
    msaa: wgpu::TextureView,
    depth: wgpu::TextureView,
    globals: wgpu::Buffer,
    bind: wgpu::BindGroup,
    sky: wgpu::RenderPipeline,
    lit: wgpu::RenderPipeline,
    /// Painted letters, cut out of flat quads by the font in the shader.
    letters: wgpu::RenderPipeline,
    ground: wgpu::RenderPipeline,
    water: wgpu::RenderPipeline,
    shadow: wgpu::RenderPipeline,
    glow: wgpu::RenderPipeline,
    fx: wgpu::RenderPipeline,
    /// Bytes of mesh handed to the GPU so far, for seeing what a frame uploaded, and the
    /// biggest single mesh since `largest` was last cleared.
    pub uploaded: u64,
    pub largest: u64,
    meshes: Vec<Option<GpuMesh>>,
    free: Vec<u32>,
    statics: Vec<StaticBatch>,
    letter_statics: Vec<StaticBatch>,
    dynamic: wgpu::Buffer,
    identity: wgpu::Buffer,
    shadow_mesh: Option<MeshId>,
    ring_mesh: Option<MeshId>,
    hud: wgpu::RenderPipeline,
    hud_atlas: wgpu::Texture,
    hud_bind: wgpu::BindGroup,
    hud_buf: wgpu::Buffer,
}

fn vec4(v: Vec3, w: f32) -> [f32; 4] {
    [v.x, v.y, v.z, w]
}

/// The six clip planes of a view projection, normalized, facing inward.
fn frustum(m: Mat4) -> [Vec4; 6] {
    let r = |i: usize| m.row(i);
    [
        r(3) + r(0),
        r(3) - r(0),
        r(3) + r(1),
        r(3) - r(1),
        r(2),
        r(3) - r(2),
    ]
    .map(|p| p / p.truncate().length())
}

fn in_view(planes: &[Vec4; 6], center: Vec3, radius: f32) -> bool {
    !radius.is_finite() || planes.iter().all(|p| p.dot(center.extend(1.0)) >= -radius)
}

impl Renderer {
    pub fn new(
        device: wgpu::Device,
        queue: wgpu::Queue,
        format: wgpu::TextureFormat,
        size: (u32, u32),
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("shader"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("shader.wgsl")
                    .replace("GLYPH_TABLE", &glyph_table())
                    .into(),
            ),
        });
        let globals = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("globals"),
            size: size_of::<Globals>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("globals"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("globals"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: globals.as_entire_binding(),
            }],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("main"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });

        let vertex_layout = wgpu::VertexBufferLayout {
            array_stride: size_of::<Vertex>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Snorm8x4, 2 => Unorm8x4],
        };
        let instance_layout = wgpu::VertexBufferLayout {
            array_stride: size_of::<Instance>() as u64,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &wgpu::vertex_attr_array![
                3 => Float32x4, 4 => Float32x4, 5 => Float32x4, 6 => Float32x4, 7 => Float32x4
            ],
        };
        let pipeline = |label,
                        vs,
                        fs,
                        buffers: &[Option<wgpu::VertexBufferLayout>],
                        depth_write,
                        depth_test,
                        blend| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some(vs),
                    compilation_options: Default::default(),
                    buffers,
                },
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: DEPTH,
                    depth_write_enabled: Some(depth_write),
                    // Reversed depth: nearer is larger.
                    depth_compare: Some(if depth_test {
                        wgpu::CompareFunction::GreaterEqual
                    } else {
                        wgpu::CompareFunction::Always
                    }),
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: wgpu::MultisampleState {
                    count: SAMPLES,
                    ..Default::default()
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some(fs),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            })
        };
        let mesh_buffers = [Some(vertex_layout), Some(instance_layout)];
        // Drawn last of the solid things, only where nothing else is.
        let sky = pipeline("sky", "vs_sky", "fs_sky", &[], false, true, None);
        let lit = pipeline("lit", "vs_lit", "fs_lit", &mesh_buffers, true, true, None);
        let letters = pipeline(
            "letters",
            "vs_lit",
            "fs_letter",
            &mesh_buffers,
            true,
            true,
            None,
        );
        let ground = pipeline(
            "ground",
            "vs_lit",
            "fs_ground",
            &mesh_buffers,
            true,
            true,
            None,
        );
        let water = pipeline(
            "water",
            "vs_lit",
            "fs_water",
            &mesh_buffers,
            false,
            true,
            Some(wgpu::BlendState::ALPHA_BLENDING),
        );
        let shadow = pipeline(
            "shadow",
            "vs_lit",
            "fs_shadow",
            &mesh_buffers,
            false,
            true,
            Some(wgpu::BlendState::ALPHA_BLENDING),
        );
        // Premultiplied, so rings brighten what is under them without washing it out.
        let premultiplied = wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent::OVER,
        };
        // Markers are interface, so they draw over the world.
        let glow = pipeline(
            "glow",
            "vs_lit",
            "fs_glow",
            &mesh_buffers,
            false,
            false,
            Some(premultiplied),
        );
        let fx = pipeline(
            "fx",
            "vs_lit",
            "fs_glow",
            &mesh_buffers,
            false,
            true,
            Some(premultiplied),
        );
        // The HUD: flat triangles over everything, from its atlas.
        let hud_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("hud"),
            source: wgpu::ShaderSource::Wgsl(include_str!("hud.wgsl").into()),
        });
        let hud_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("hud"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let (aw, ah) = crate::hud::ATLAS;
        let hud_atlas = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("hud atlas"),
            size: wgpu::Extent3d {
                width: aw,
                height: ah,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        // Smooth, so text drawn a little smaller than its atlas size stays clean.
        let hud_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("hud"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let hud_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("hud"),
            layout: &hud_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(
                        &hud_atlas.create_view(&Default::default()),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&hud_sampler),
                },
            ],
        });
        let hud = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("hud"),
            layout: Some(
                &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("hud"),
                    bind_group_layouts: &[Some(&hud_layout)],
                    immediate_size: 0,
                }),
            ),
            vertex: wgpu::VertexState {
                module: &hud_shader,
                entry_point: Some("vs_hud"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: size_of::<crate::hud::HudVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![
                        0 => Float32x2, 1 => Float32x2, 2 => Float32x4
                    ],
                })],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH,
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::Always),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: SAMPLES,
                ..Default::default()
            },
            fragment: Some(wgpu::FragmentState {
                module: &hud_shader,
                entry_point: Some("fs_hud"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        let hud_buf = Self::hud_buffer(&device, 4096);
        let dynamic = Self::instance_buffer(&device, 256);
        let identity = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("identity instance"),
            contents: bytemuck::bytes_of(&Instance::new(Mat4::IDENTITY)),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let (msaa, depth) = Self::targets(&device, format, size);
        let mut r = Self {
            stats: Stats::default(),
            hide: Vec::new(),
            device,
            queue,
            format,
            size,
            msaa,
            depth,
            globals,
            bind,
            sky,
            lit,
            letters,
            ground,
            water,
            shadow,
            glow,
            fx,
            uploaded: 0,
            largest: 0,
            meshes: Vec::new(),
            free: Vec::new(),
            statics: Vec::new(),
            letter_statics: Vec::new(),
            dynamic,
            identity,
            shadow_mesh: None,
            ring_mesh: None,
            hud,
            hud_atlas,
            hud_bind,
            hud_buf,
        };
        // The font goes in now. The map follows once it is drawn.
        let mut atlas = vec![0u8; (aw * ah * 4) as usize];
        crate::hud::font_into(&mut atlas);
        crate::hud::thumbs_into(&mut atlas);
        r.write_hud_atlas(&atlas, 0, ah);
        r.shadow_mesh = Some(r.add_mesh(&crate::models::shadow_disc()));
        r.ring_mesh = Some(r.add_mesh(&crate::models::ring()));
        r
    }

    fn hud_buffer(device: &wgpu::Device, count: usize) -> wgpu::Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("hud"),
            size: (count * size_of::<crate::hud::HudVertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    /// Writes rows `y0..y0 + rows` of the HUD atlas from full-width RGBA rows.
    pub fn write_hud_atlas(&self, rgba: &[u8], y0: u32, rows: u32) {
        let w = crate::hud::ATLAS.0;
        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.hud_atlas,
                mip_level: 0,
                origin: wgpu::Origin3d { x: 0, y: y0, z: 0 },
                aspect: wgpu::TextureAspect::All,
            },
            &rgba[(y0 * w * 4) as usize..((y0 + rows) * w * 4) as usize],
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(w * 4),
                rows_per_image: Some(rows),
            },
            wgpu::Extent3d {
                width: w,
                height: rows,
                depth_or_array_layers: 1,
            },
        );
    }

    /// Puts a `w` by `h` RGBA picture into the HUD atlas at `x`, `y`.
    pub fn write_hud_rect(&self, x: u32, y: u32, w: u32, h: u32, rgba: &[u8]) {
        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.hud_atlas,
                mip_level: 0,
                origin: wgpu::Origin3d { x, y, z: 0 },
                aspect: wgpu::TextureAspect::All,
            },
            rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(w * 4),
                rows_per_image: Some(h),
            },
            wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
        );
    }

    /// Puts the world map, `MAP` square RGBA, into the HUD atlas.
    pub fn set_hud_map(&self, rgba: &[u8]) {
        self.write_hud_atlas(rgba, 0, crate::hud::MAP);
    }

    fn instance_buffer(device: &wgpu::Device, count: usize) -> wgpu::Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("instances"),
            size: (count * size_of::<Instance>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    fn targets(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        (w, h): (u32, u32),
    ) -> (wgpu::TextureView, wgpu::TextureView) {
        let make = |label, format| {
            device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some(label),
                    size: wgpu::Extent3d {
                        width: w.max(1),
                        height: h.max(1),
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: SAMPLES,
                    dimension: wgpu::TextureDimension::D2,
                    format,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                    view_formats: &[],
                })
                .create_view(&Default::default())
        };
        (make("msaa", format), make("depth", DEPTH))
    }

    pub fn size(&self) -> (u32, u32) {
        self.size
    }

    pub fn resize(&mut self, size: (u32, u32)) {
        if size != self.size {
            self.size = size;
            (self.msaa, self.depth) = Self::targets(&self.device, self.format, size);
        }
    }

    pub fn add_mesh(&mut self, mesh: &MeshBuilder) -> MeshId {
        let bytes = (mesh.verts.len() * std::mem::size_of::<crate::mesh::Vertex>()) as u64;
        self.uploaded += bytes;
        self.largest = self.largest.max(bytes);
        let buf = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("mesh"),
                contents: bytemuck::cast_slice(&mesh.verts),
                usage: wgpu::BufferUsages::VERTEX,
            });
        let (lo, hi) = mesh.verts.iter().fold(
            (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN)),
            |(lo, hi), v| (lo.min(Vec3::from(v.pos)), hi.max(Vec3::from(v.pos))),
        );
        let gpu = Some(GpuMesh {
            buf,
            count: mesh.verts.len() as u32,
            lo,
            hi,
        });
        match self.free.pop() {
            Some(i) => {
                self.meshes[i as usize] = gpu;
                MeshId(i)
            }
            None => {
                self.meshes.push(gpu);
                MeshId(self.meshes.len() as u32 - 1)
            }
        }
    }

    /// Starts a mesh on its way to the GPU without sending any of it yet, so a big one can
    /// go up a slice a frame (see [`Renderer::send_mesh`]) instead of stalling a slow GPU
    /// in one go. None for an empty mesh.
    pub fn start_mesh(&mut self, mesh: &MeshBuilder) -> Option<Sending> {
        if mesh.verts.is_empty() {
            return None;
        }
        let bytes: Vec<u8> = bytemuck::cast_slice(&mesh.verts).to_vec();
        let buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("mesh"),
            size: bytes.len() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let (lo, hi) = mesh.verts.iter().fold(
            (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN)),
            |(lo, hi), v| (lo.min(Vec3::from(v.pos)), hi.max(Vec3::from(v.pos))),
        );
        Some(Sending {
            buf,
            bytes,
            sent: 0,
            count: mesh.verts.len() as u32,
            lo,
            hi,
        })
    }

    /// Sends up to `max` more bytes of a mesh. Says whether all of it is there now.
    pub fn send_mesh(&mut self, s: &mut Sending, max: usize) -> bool {
        // Whole vertices, which keeps every copy a multiple of four bytes.
        let vertex = std::mem::size_of::<crate::mesh::Vertex>();
        let n = (max.max(vertex) / vertex * vertex).min(s.bytes.len() - s.sent);
        self.queue
            .write_buffer(&s.buf, s.sent as u64, &s.bytes[s.sent..s.sent + n]);
        s.sent += n;
        self.uploaded += n as u64;
        self.largest = self.largest.max(n as u64);
        s.sent == s.bytes.len()
    }

    /// A mesh that is all there, ready to draw.
    pub fn finish_mesh(&mut self, s: Sending) -> MeshId {
        let gpu = Some(GpuMesh {
            buf: s.buf,
            count: s.count,
            lo: s.lo,
            hi: s.hi,
        });
        match self.free.pop() {
            Some(i) => {
                self.meshes[i as usize] = gpu;
                MeshId(i)
            }
            None => {
                self.meshes.push(gpu);
                MeshId(self.meshes.len() as u32 - 1)
            }
        }
    }

    /// Releases a mesh. Its id may be handed out again.
    pub fn free_mesh(&mut self, id: MeshId) {
        if self.meshes[id.0 as usize].take().is_some() {
            self.free.push(id.0);
        }
    }

    /// Adds a mesh of painted letters that never moves, culled like a static batch.
    pub fn add_letters(&mut self, mesh: MeshId, cull: Cull) {
        let n = self.statics.len();
        self.add_static_culled(mesh, &[Instance::new(Mat4::IDENTITY)], cull);
        if self.statics.len() > n {
            let b = self.statics.pop().expect("just added");
            self.letter_statics.push(b);
        }
    }

    /// Adds instances of a mesh that never move, always drawn.
    pub fn add_static(&mut self, mesh: MeshId, instances: &[Instance]) {
        self.add_static_culled(mesh, instances, Cull::ALWAYS);
    }

    /// Adds instances of a mesh that never move, culled and simplified with distance.
    pub fn add_static_culled(&mut self, mesh: MeshId, instances: &[Instance], cull: Cull) {
        if instances.is_empty() {
            return;
        }
        let buf = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("static instances"),
                contents: bytemuck::cast_slice(instances),
                usage: wgpu::BufferUsages::VERTEX,
            });
        // The box round every instance's box, or the cull sphere's if the mesh is gone.
        let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
        match &self.meshes[mesh.0 as usize] {
            Some(m) if m.count > 0 && cull.radius.is_finite() => {
                for i in instances {
                    let model = Mat4::from_cols_array_2d(&i.model);
                    for k in 0..8 {
                        let corner = Vec3::new(
                            if k & 1 == 0 { m.lo.x } else { m.hi.x },
                            if k & 2 == 0 { m.lo.y } else { m.hi.y },
                            if k & 4 == 0 { m.lo.z } else { m.hi.z },
                        );
                        let p = model.transform_point3(corner);
                        lo = lo.min(p);
                        hi = hi.max(p);
                    }
                }
            }
            _ => {
                lo = cull.center - Vec3::splat(cull.radius);
                hi = cull.center + Vec3::splat(cull.radius);
            }
        }
        self.statics.push(StaticBatch {
            mesh,
            buf,
            count: instances.len() as u32,
            cull,
            lo,
            hi,
        });
    }

    /// Renders `frame` into `target`, which must be viewed with [`Renderer::format`] at
    /// [`Renderer::size`].
    /// Draws the static batches in `list` that are in view and in reach.
    fn draw_statics(
        &self,
        pass: &mut wgpu::RenderPass,
        list: &[StaticBatch],
        frame: &Frame,
        planes: &[Vec4; 6],
        shown: impl Fn(&Cull) -> bool,
        stats: &mut Stats,
    ) {
        for s in list.iter().filter(|s| shown(&s.cull)) {
            let c = &s.cull;
            let dist = frame.cam_pos.clamp(s.lo, s.hi).distance(frame.cam_pos);
            if dist > frame.prop_range * c.reach || !in_view(planes, c.center, c.radius) {
                continue;
            }
            if let Some(g) = c.group {
                let away = (g.center - frame.cam_pos).length() - g.radius;
                if (away >= g.range * frame.detail_range) != g.merged {
                    continue;
                }
            }
            let id = match c.far {
                Some(far) if dist > frame.detail_range * c.detail => far,
                _ => s.mesh,
            };
            let Some(mesh) = &self.meshes[id.0 as usize] else {
                continue;
            };
            if self.hide.contains(&(id.0 + 1_000_000)) {
                continue;
            }
            pass.set_vertex_buffer(0, mesh.buf.slice(..));
            pass.set_vertex_buffer(1, s.buf.slice(..));
            pass.draw(0..mesh.count, 0..s.count);
            stats.draws += 1;
            stats.draws_static += 1;
            let tris = (mesh.count / 3) as u64 * s.count as u64;
            stats.props += tris;
            // Static batches show as their mesh number plus a million.
            stats.heaviest.push((id.0 + 1_000_000, s.count, tris));
        }
    }

    pub fn render(&mut self, target: &wgpu::TextureView, frame: &Frame) {
        let l = &frame.lighting;
        let globals = Globals {
            view_proj: frame.view_proj.to_cols_array_2d(),
            inv_view_proj: frame.view_proj.inverse().to_cols_array_2d(),
            cam_pos: vec4(frame.cam_pos, 1.0),
            sun_dir: vec4(l.sun_dir.normalize(), l.sky_gloom),
            sun_color: vec4(l.sun_color, 0.0),
            sky_top: vec4(l.sky_top, l.valley_fog),
            sky_horizon: vec4(l.sky_horizon, l.rainbow),
            ambient_sky: vec4(l.ambient_sky, 0.0),
            ambient_ground: vec4(l.ambient_ground, 0.0),
            fog: [l.fog_start, l.fog_end, l.bands, l.shadow],
            cave: vec4(l.cave_ambient, l.underground),
            misc: [
                frame.time,
                l.night,
                frame.lights.len().min(MAX_LIGHTS) as f32,
                frame.clouds.len().min(crate::weather::SHADOWS) as f32,
            ],
            lights: {
                let mut out = [[0.0; 4]; MAX_LIGHTS * 2];
                for (i, (p, r, c)) in frame.lights.iter().take(MAX_LIGHTS).enumerate() {
                    out[i * 2] = vec4(*p, *r);
                    out[i * 2 + 1] = vec4(*c, 0.0);
                }
                out
            },
            clouds: {
                let mut out = [[0.0; 4]; crate::weather::SHADOWS];
                for (o, c) in out.iter_mut().zip(&frame.clouds) {
                    *o = c.to_array();
                }
                out
            },
            dark: {
                let mut out = [[0.0; 4]; DARK];
                for (o, c) in out.iter_mut().zip(&frame.dark) {
                    *o = c.to_array();
                }
                out
            },
            bow: vec4(l.bow, 0.0),
        };
        self.queue
            .write_buffer(&self.globals, 0, bytemuck::bytes_of(&globals));

        // Dynamic instances grouped by mesh, then shadows last.
        let mut draws = frame.draws.clone();
        draws.sort_by_key(|(m, _)| m.0);
        let mut instances: Vec<Instance> = draws.iter().map(|(_, i)| *i).collect();
        let shadow_start = instances.len() as u32;
        instances.extend_from_slice(&frame.shadows);
        let ring_start = instances.len() as u32;
        instances.extend_from_slice(&frame.rings);
        let mut fx = frame.fx.clone();
        fx.sort_by_key(|(m, _)| m.0);
        let fx_start = instances.len();
        instances.extend(fx.iter().map(|(_, i)| *i));
        if (frame.hud.len() * size_of::<crate::hud::HudVertex>()) as u64 > self.hud_buf.size() {
            self.hud_buf = Self::hud_buffer(&self.device, frame.hud.len().next_power_of_two());
        }
        if !frame.hud.is_empty() {
            self.queue
                .write_buffer(&self.hud_buf, 0, bytemuck::cast_slice(&frame.hud));
        }
        let needed = (instances.len() * size_of::<Instance>()) as u64;
        if needed > self.dynamic.size() {
            self.dynamic = Self::instance_buffer(&self.device, instances.len().next_power_of_two());
        }
        if !instances.is_empty() {
            self.queue
                .write_buffer(&self.dynamic, 0, bytemuck::cast_slice(&instances));
        }

        let mut stats = Stats {
            meshes: self.meshes.iter().filter(|m| m.is_some()).count() as u32,
            ..Default::default()
        };
        let mut enc = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("main"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.msaa,
                    depth_slice: None,
                    resolve_target: Some(target),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Discard,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(0.0),
                        store: wgpu::StoreOp::Discard,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_bind_group(0, &self.bind, &[]);
            let planes = frustum(frame.view_proj);
            // Peeking in from outside shows cave walls but not everything inside.
            let inside = frame.lighting.underground > 0.1;
            let shown = |c: &Cull| {
                if c.cave {
                    frame.caves && inside
                } else {
                    !frame.deep
                }
            };
            let statics = !self.hide.contains(&PART_STATICS);
            if statics {
                pass.set_pipeline(&self.lit);
                self.draw_statics(&mut pass, &self.statics, frame, &planes, shown, &mut stats);
            }
            pass.set_pipeline(&self.ground);
            for (id, center, radius) in &frame.terrain {
                if !in_view(&planes, *center, *radius)
                    || self.hide.contains(&4_000_000)
                    || self.hide.contains(&(id.0 + 4_000_000))
                {
                    continue;
                }
                let Some(mesh) = &self.meshes[id.0 as usize] else {
                    continue;
                };
                pass.set_vertex_buffer(0, mesh.buf.slice(..));
                pass.set_vertex_buffer(1, self.identity.slice(..));
                pass.draw(0..mesh.count, 0..1);
                stats.draws += 1;
                stats.draws_terrain += 1;
                stats.terrain += (mesh.count / 3) as u64;
                // Terrain chunks show as their mesh number plus four million.
                stats
                    .heaviest
                    .push((id.0 + 4_000_000, 1, (mesh.count / 3) as u64));
            }
            pass.set_pipeline(&self.lit);
            let mut i = if self.hide.contains(&PART_DYNAMIC) {
                draws.len()
            } else {
                0
            };
            while i < draws.len() {
                let id = draws[i].0;
                let end = i + draws[i..].iter().take_while(|(m, _)| *m == id).count();
                if let Some(mesh) = &self.meshes[id.0 as usize]
                    && !self.hide.contains(&id.0)
                {
                    pass.set_vertex_buffer(0, mesh.buf.slice(..));
                    pass.set_vertex_buffer(1, self.dynamic.slice(..));
                    pass.draw(0..mesh.count, i as u32..end as u32);
                    stats.draws += 1;
                    let tris = (mesh.count / 3) as u64 * (end - i) as u64;
                    stats.triangles += tris;
                    stats.heaviest.push((id.0, (end - i) as u32, tris));
                }
                i = end;
            }

            // Letters cut themselves out of their quads, and the sky fills whatever is
            // left, so both come after everything solid. Tiled GPUs (iPads) skip the hidden
            // parts of solid things best that way.
            if statics {
                pass.set_pipeline(&self.letters);
                self.draw_statics(
                    &mut pass,
                    &self.letter_statics,
                    frame,
                    &planes,
                    shown,
                    &mut stats,
                );
            }
            if !self.hide.contains(&PART_SKY) {
                pass.set_pipeline(&self.sky);
                pass.draw(0..3, 0..1);
            }

            pass.set_pipeline(&self.water);
            for (id, center, radius) in &frame.water {
                if !in_view(&planes, *center, *radius) || self.hide.contains(&3_000_000) {
                    continue;
                }
                let Some(mesh) = &self.meshes[id.0 as usize] else {
                    continue;
                };
                pass.set_vertex_buffer(0, mesh.buf.slice(..));
                pass.set_vertex_buffer(1, self.identity.slice(..));
                pass.draw(0..mesh.count, 0..1);
                stats.draws += 1;
                stats.draws_water += 1;
                stats.water += (mesh.count / 3) as u64;
            }

            pass.set_pipeline(&self.fx);
            let mut i = if self.hide.contains(&PART_FX) {
                fx.len()
            } else {
                0
            };
            while i < fx.len() {
                let id = fx[i].0;
                let end = i + fx[i..].iter().take_while(|(m, _)| *m == id).count();
                if let Some(mesh) = &self.meshes[id.0 as usize]
                    && !self.hide.contains(&(id.0 + 2_000_000))
                {
                    pass.set_vertex_buffer(0, mesh.buf.slice(..));
                    pass.set_vertex_buffer(1, self.dynamic.slice(..));
                    pass.draw(
                        0..mesh.count,
                        (fx_start + i) as u32..(fx_start + end) as u32,
                    );
                    stats.draws += 1;
                    let tris = (mesh.count / 3) as u64 * (end - i) as u64;
                    stats.triangles += tris;
                    // Glow and effects show as their mesh number plus two million.
                    stats
                        .heaviest
                        .push((id.0 + 2_000_000, (end - i) as u32, tris));
                }
                i = end;
            }

            let shadows = !self.hide.contains(&PART_SHADOWS);
            if let (Some(disc), false, true) = (self.shadow_mesh, frame.shadows.is_empty(), shadows)
                && let Some(mesh) = &self.meshes[disc.0 as usize]
            {
                pass.set_pipeline(&self.shadow);
                pass.set_vertex_buffer(0, mesh.buf.slice(..));
                pass.set_vertex_buffer(1, self.dynamic.slice(..));
                pass.draw(
                    0..mesh.count,
                    shadow_start..shadow_start + frame.shadows.len() as u32,
                );
            }
            if let (Some(ring), false, true) = (self.ring_mesh, frame.rings.is_empty(), shadows)
                && let Some(mesh) = &self.meshes[ring.0 as usize]
            {
                pass.set_pipeline(&self.glow);
                pass.set_vertex_buffer(0, mesh.buf.slice(..));
                pass.set_vertex_buffer(1, self.dynamic.slice(..));
                pass.draw(
                    0..mesh.count,
                    ring_start..ring_start + frame.rings.len() as u32,
                );
            }
            if !frame.hud.is_empty() && !self.hide.contains(&PART_HUD) {
                pass.set_pipeline(&self.hud);
                pass.set_bind_group(0, &self.hud_bind, &[]);
                pass.set_vertex_buffer(0, self.hud_buf.slice(..));
                pass.draw(0..frame.hud.len() as u32, 0..1);
                stats.draws += 1;
            }
        }
        self.queue.submit([enc.finish()]);
        stats.triangles += stats.terrain + stats.props + stats.water;
        self.stats = stats;
    }
}
