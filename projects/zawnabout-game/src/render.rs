//! The forward renderer: a sky pass, flat-shaded lit meshes and soft blob shadows, all
//! drawn into a 4x MSAA target.

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3, Vec4};
use wgpu::util::DeviceExt;

use crate::mesh::{MeshBuilder, Vertex};

pub const SAMPLES: u32 = 4;
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
}

/// Everything one frame draws.
#[derive(Clone, Debug)]
pub struct Frame {
    pub view_proj: Mat4,
    pub cam_pos: Vec3,
    pub lighting: Lighting,
    pub draws: Vec<(MeshId, Instance)>,
    pub shadows: Vec<Instance>,
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
}

struct GpuMesh {
    buf: wgpu::Buffer,
    count: u32,
}

struct StaticBatch {
    mesh: MeshId,
    buf: wgpu::Buffer,
    count: u32,
}

pub struct Renderer {
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
    shadow: wgpu::RenderPipeline,
    meshes: Vec<GpuMesh>,
    statics: Vec<StaticBatch>,
    dynamic: wgpu::Buffer,
    shadow_mesh: Option<MeshId>,
}

fn vec4(v: Vec3, w: f32) -> [f32; 4] {
    [v.x, v.y, v.z, w]
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
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
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
                    depth_compare: Some(if depth_test {
                        wgpu::CompareFunction::LessEqual
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
        let sky = pipeline("sky", "vs_sky", "fs_sky", &[], false, false, None);
        let lit = pipeline("lit", "vs_lit", "fs_lit", &mesh_buffers, true, true, None);
        let shadow = pipeline(
            "shadow",
            "vs_lit",
            "fs_shadow",
            &mesh_buffers,
            false,
            true,
            Some(wgpu::BlendState::ALPHA_BLENDING),
        );
        let dynamic = Self::instance_buffer(&device, 256);
        let (msaa, depth) = Self::targets(&device, format, size);
        let mut r = Self {
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
            shadow,
            meshes: Vec::new(),
            statics: Vec::new(),
            dynamic,
            shadow_mesh: None,
        };
        r.shadow_mesh = Some(r.add_mesh(&crate::models::shadow_disc()));
        r
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
        let buf = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("mesh"),
                contents: bytemuck::cast_slice(&mesh.verts),
                usage: wgpu::BufferUsages::VERTEX,
            });
        self.meshes.push(GpuMesh {
            buf,
            count: mesh.verts.len() as u32,
        });
        MeshId(self.meshes.len() as u32 - 1)
    }

    /// Adds instances of a mesh that never move.
    pub fn add_static(&mut self, mesh: MeshId, instances: &[Instance]) {
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
        self.statics.push(StaticBatch {
            mesh,
            buf,
            count: instances.len() as u32,
        });
    }

    /// Renders `frame` into `target`, which must be viewed with [`Renderer::format`] at
    /// [`Renderer::size`].
    pub fn render(&mut self, target: &wgpu::TextureView, frame: &Frame) {
        let l = &frame.lighting;
        let globals = Globals {
            view_proj: frame.view_proj.to_cols_array_2d(),
            inv_view_proj: frame.view_proj.inverse().to_cols_array_2d(),
            cam_pos: vec4(frame.cam_pos, 1.0),
            sun_dir: vec4(l.sun_dir.normalize(), 0.0),
            sun_color: vec4(l.sun_color, 0.0),
            sky_top: vec4(l.sky_top, 0.0),
            sky_horizon: vec4(l.sky_horizon, 0.0),
            ambient_sky: vec4(l.ambient_sky, 0.0),
            ambient_ground: vec4(l.ambient_ground, 0.0),
            fog: [l.fog_start, l.fog_end, l.bands, l.shadow],
        };
        self.queue
            .write_buffer(&self.globals, 0, bytemuck::bytes_of(&globals));

        // Dynamic instances grouped by mesh, then shadows last.
        let mut draws = frame.draws.clone();
        draws.sort_by_key(|(m, _)| m.0);
        let mut instances: Vec<Instance> = draws.iter().map(|(_, i)| *i).collect();
        let shadow_start = instances.len() as u32;
        instances.extend_from_slice(&frame.shadows);
        let needed = (instances.len() * size_of::<Instance>()) as u64;
        if needed > self.dynamic.size() {
            self.dynamic = Self::instance_buffer(&self.device, instances.len().next_power_of_two());
        }
        if !instances.is_empty() {
            self.queue
                .write_buffer(&self.dynamic, 0, bytemuck::cast_slice(&instances));
        }

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
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Discard,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_bind_group(0, &self.bind, &[]);
            pass.set_pipeline(&self.sky);
            pass.draw(0..3, 0..1);

            pass.set_pipeline(&self.lit);
            for s in &self.statics {
                let mesh = &self.meshes[s.mesh.0 as usize];
                pass.set_vertex_buffer(0, mesh.buf.slice(..));
                pass.set_vertex_buffer(1, s.buf.slice(..));
                pass.draw(0..mesh.count, 0..s.count);
            }
            let mut i = 0;
            while i < draws.len() {
                let id = draws[i].0;
                let end = i + draws[i..].iter().take_while(|(m, _)| *m == id).count();
                let mesh = &self.meshes[id.0 as usize];
                pass.set_vertex_buffer(0, mesh.buf.slice(..));
                pass.set_vertex_buffer(1, self.dynamic.slice(..));
                pass.draw(0..mesh.count, i as u32..end as u32);
                i = end;
            }

            if let (Some(disc), false) = (self.shadow_mesh, frame.shadows.is_empty()) {
                let mesh = &self.meshes[disc.0 as usize];
                pass.set_pipeline(&self.shadow);
                pass.set_vertex_buffer(0, mesh.buf.slice(..));
                pass.set_vertex_buffer(1, self.dynamic.slice(..));
                pass.draw(
                    0..mesh.count,
                    shadow_start..shadow_start + frame.shadows.len() as u32,
                );
            }
        }
        self.queue.submit([enc.finish()]);
    }
}
