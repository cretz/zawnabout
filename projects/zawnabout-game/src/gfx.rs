//! Turns game state into frames: registers every mesh once, then builds each frame's
//! draw list from the camera and dragon poses.

use std::collections::HashMap;

use glam::{Mat4, Quat, Vec3, Vec4};
use zawnabout_sim::PropKind;

use crate::game::Game;
use crate::mesh::Rgb;
use crate::models;
use crate::pose::Part;
use crate::render::{Cull, Frame, Instance, Lighting, MeshId, Renderer};
use crate::terrain_mesh::TerrainMeshes;

struct DragonIds {
    body: MeshId,
    neck: MeshId,
    head: MeshId,
    eye: MeshId,
    tail: Vec<MeshId>,
    leg: MeshId,
    arm: [MeshId; 2],
    hand: [MeshId; 2],
    star: MeshId,
}

pub struct Gfx {
    pub renderer: Renderer,
    dragon: DragonIds,
    terrain: TerrainMeshes,
}

/// Props are batched per square of this size (m), so far squares can be skipped.
const PROP_TILE: f32 = 512.0;
const CLOUD_TILE: f32 = 3000.0;

fn linear(c: Rgb, scale: f32) -> Vec3 {
    let f = |s: u32| ((s & 0xFF) as f32 / 255.0).powf(2.2) * scale;
    Vec3::new(f(c.0 >> 16), f(c.0 >> 8), f(c.0))
}

impl Gfx {
    pub fn new(mut renderer: Renderer, game: &Game) -> Self {
        let world = &game.sim.world;
        let sea = renderer.add_mesh(&models::sea(&world.terrain));
        renderer.add_static(sea, &[Instance::new(Mat4::IDENTITY)]);
        let lakes = renderer.add_mesh(&models::lakes(&world.terrain));
        renderer.add_static(lakes, &[Instance::new(Mat4::IDENTITY)]);

        // Props are batched by tile and culled by distance. Clouds use bigger tiles and
        // reach farther.
        let tile = |p: Vec3, kind: PropKind| {
            let size = if kind == PropKind::Cloud {
                CLOUD_TILE
            } else {
                PROP_TILE
            };
            ((p.x / size).floor() as i32, (p.z / size).floor() as i32)
        };
        let mut batches: HashMap<(PropKind, u8, (i32, i32)), Vec<Instance>> = HashMap::new();
        let variants: HashMap<PropKind, usize> = PropKind::ALL
            .iter()
            .map(|k| (*k, models::prop_variants(*k).len()))
            .collect();
        for p in &world.props {
            let v = p.variant % variants[&p.kind] as u8;
            let m = Mat4::from_scale_rotation_translation(
                Vec3::splat(p.scale),
                Quat::from_rotation_y(p.yaw),
                p.pos,
            );
            // Clouds glow a little so they stay white from below.
            let tint = if p.kind == PropKind::Cloud {
                Vec4::new(1.5, 1.5, 1.5, 1.0)
            } else {
                Vec4::ONE
            };
            batches
                .entry((p.kind, v, tile(p.pos, p.kind)))
                .or_default()
                .push(Instance::tinted(m, tint));
        }
        let mut keys: Vec<_> = batches.keys().copied().collect();
        keys.sort_by_key(|(k, v, t)| (*k as u8, *v, *t));
        let mut ids: HashMap<(PropKind, u8), MeshId> = HashMap::new();
        let mut far_ids: HashMap<(PropKind, u8), Option<MeshId>> = HashMap::new();
        for key in keys {
            let (kind, v, (tx, tz)) = key;
            let id = *ids
                .entry((kind, v))
                .or_insert_with(|| renderer.add_mesh(&models::prop_variants(kind)[v as usize]));
            let far = *far_ids
                .entry((kind, v))
                .or_insert_with(|| models::prop_far(kind, v as u32).map(|m| renderer.add_mesh(&m)));
            let instances = &batches[&key];
            // Clouds reach farther and keep detail longer, flowers only show up close.
            let (size, y, reach, detail) = match kind {
                PropKind::Cloud => (CLOUD_TILE, 700.0, 3.0, 3.0),
                PropKind::Flower | PropKind::Bush => (PROP_TILE, 300.0, 0.25, 1.0),
                _ => (PROP_TILE, 300.0, 1.0, 1.0),
            };
            let center = Vec3::new((tx as f32 + 0.5) * size, y, (tz as f32 + 0.5) * size);
            // Half the tile diagonal plus room for tall things.
            let radius = size * 0.71 + 700.0;
            renderer.add_static_culled(
                id,
                instances,
                Cull {
                    center,
                    radius,
                    reach,
                    far,
                    detail,
                },
            );
        }

        let m = models::dragon();
        let dragon = DragonIds {
            body: renderer.add_mesh(&m.body),
            neck: renderer.add_mesh(&m.neck),
            head: renderer.add_mesh(&m.head),
            eye: renderer.add_mesh(&m.eye),
            tail: m.tail.iter().map(|t| renderer.add_mesh(t)).collect(),
            leg: renderer.add_mesh(&m.leg),
            arm: [renderer.add_mesh(&m.arm[0]), renderer.add_mesh(&m.arm[1])],
            hand: [renderer.add_mesh(&m.hand[0]), renderer.add_mesh(&m.hand[1])],
            star: renderer.add_mesh(&m.star),
        };
        let mut gfx = Self {
            renderer,
            dragon,
            terrain: TerrainMeshes::default(),
        };
        gfx.update_terrain(game, usize::MAX);
        gfx
    }

    /// Streams terrain chunks around the camera, building at most `budget` new ones.
    pub fn update_terrain(&mut self, game: &Game, budget: usize) {
        self.terrain.update(
            &mut self.renderer,
            &game.sim.world.terrain,
            game.camera.pos,
            budget,
        );
    }

    fn mesh(&self, part: Part) -> MeshId {
        let d = &self.dragon;
        match part {
            Part::Body => d.body,
            Part::Neck(_) => d.neck,
            Part::Head => d.head,
            Part::Eye => d.eye,
            Part::Tail(i) => d.tail[i],
            Part::Leg => d.leg,
            Part::Arm(i) => d.arm[i],
            Part::Hand(i) => d.hand[i],
            Part::Star => d.star,
        }
    }

    pub fn frame(&self, game: &Game) -> Frame {
        let (w, h) = self.renderer.size();
        let look = &game.look;
        let (yaw, pitch) = (look.sun_yaw.to_radians(), look.sun_height.to_radians());
        let lighting = Lighting {
            sun_dir: Vec3::new(
                yaw.sin() * pitch.cos(),
                pitch.sin(),
                yaw.cos() * pitch.cos(),
            ),
            sun_color: linear(Rgb(0xFFF1DC), 1.05),
            sky_top: linear(Rgb(0x5FAEEB), 1.0),
            sky_horizon: linear(Rgb(0xFFE6CF), 1.0),
            ambient_sky: linear(Rgb(0xA9CBF0), 0.62),
            ambient_ground: linear(Rgb(0xC9E0A6), 0.42),
            fog_start: look.fog_start,
            fog_end: look.fog_end,
            bands: look.bands,
            shadow: look.shadow,
            cave_ambient: linear(Rgb(0xB8B4D8), 0.95),
            underground: 1.0 - game.sim.world.terrain.daylight(game.camera.pos),
        };

        let alpha = game.alpha();
        let mut parts = Vec::new();
        let mut draws = Vec::new();
        let mut shadows = Vec::new();
        let world = &game.sim.world;
        for (_, visual) in &game.visuals {
            parts.clear();
            visual.parts(alpha, &mut parts);
            // Instance alpha carries daylight, so the dragon darkens in caves too.
            let (pos, _) = visual.interpolated(alpha);
            let day = world.terrain.daylight(pos);
            draws.extend(parts.iter().map(|(part, m, tint)| {
                (
                    self.mesh(*part),
                    Instance::tinted(*m, tint.truncate().extend(tint.w * day)),
                )
            }));
            let ground = world.support(pos.x, pos.z, pos.y);
            let height = (pos.y - ground).max(0.0);
            let normal = world.ground_normal(pos.x, pos.z);
            let radius = 2.6 + height * 0.015;
            let strength = 1.0 - (height / 160.0).clamp(0.0, 1.0);
            let m = Mat4::from_scale_rotation_translation(
                Vec3::new(radius, 1.0, radius * 1.3),
                Quat::from_rotation_arc(Vec3::Y, normal)
                    * Quat::from_rotation_y(game.player_dragon().heading),
                Vec3::new(pos.x, ground + 0.12, pos.z),
            );
            shadows.push(Instance::tinted(m, Vec4::new(1.0, 1.0, 1.0, strength)));
        }

        // Landing rings: two that breathe in and out, warmer for the destination.
        let mut rings = Vec::new();
        let t = game.time as f32;
        let player = game.player_dragon().pos;
        for m in game.markers() {
            let fade = if game.feedback.active {
                1.0
            } else {
                1.0 - (m.pos.distance(player) / game.look.ring_range).clamp(0.0, 1.0)
            };
            let (color, strength) = if m.destination {
                (linear(Rgb(0xFFB089), 1.0), 0.95)
            } else {
                (linear(Rgb(0xFFE38A), 1.0), 0.95 * fade.sqrt())
            };
            for k in 0..2 {
                let phase = (t * 0.8 + k as f32 * 0.5).fract();
                // Halos face the camera and keep a minimum on-screen size, so spots stay
                // easy to see and tap from any angle and distance.
                let center = m.pos + Vec3::Y * 0.6;
                let to_cam = game.camera.pos - center;
                let size = m.radius.max(to_cam.length() * 0.035);
                let scale = size * (0.55 + 0.6 * phase);
                let alpha = strength * (1.0 - phase) * (phase * 4.0).min(1.0);
                let face = Quat::from_rotation_arc(Vec3::Y, to_cam.normalize_or(Vec3::Y));
                let model = Mat4::from_scale_rotation_translation(
                    Vec3::new(scale, 1.0, scale),
                    face,
                    center,
                );
                rings.push(Instance::tinted(model, color.extend(alpha)));
            }
        }

        Frame {
            view_proj: game.camera.view_proj(w as f32 / h.max(1) as f32),
            cam_pos: game.camera.pos,
            lighting,
            draws,
            terrain: self.terrain.draw_list(
                game.camera.pos,
                // Cave insides are hidden unless underground or close enough to see in.
                lighting.underground > 0.05 || world.terrain.near_voids(game.camera.pos, 150.0),
                lighting.underground,
            ),
            prop_range: game.look.prop_range,
            detail_range: game.look.detail_range,
            deep: lighting.underground > 0.85,
            shadows,
            rings,
        }
    }

    pub fn render(&mut self, game: &Game, target: &wgpu::TextureView) {
        self.update_terrain(game, 3);
        let frame = self.frame(game);
        self.renderer.render(target, &frame);
    }

    /// Renders one frame offscreen and reads it back as tightly packed RGBA8. Blocks,
    /// so native only.
    pub fn capture(&mut self, game: &Game) -> Vec<u8> {
        // Shots always show finished terrain.
        while !self
            .terrain
            .settled(&game.sim.world.terrain, game.camera.pos)
        {
            self.update_terrain(game, usize::MAX);
        }
        let (w, h) = self.renderer.size();
        let device = &self.renderer.device;
        let tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("capture"),
            size: wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.renderer.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = tex.create_view(&Default::default());
        self.render(game, &view);

        let device = &self.renderer.device;
        let row = (w * 4).div_ceil(256) * 256;
        let buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("readback"),
            size: (row * h) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut enc = device.create_command_encoder(&Default::default());
        enc.copy_texture_to_buffer(
            tex.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &buf,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(row),
                    rows_per_image: Some(h),
                },
            },
            wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
        );
        self.renderer.queue.submit([enc.finish()]);
        buf.slice(..)
            .map_async(wgpu::MapMode::Read, |r| r.expect("map readback"));
        device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("poll device");
        let data = buf.slice(..).get_mapped_range().expect("mapped range");
        let bgra = matches!(
            self.renderer.format,
            wgpu::TextureFormat::Bgra8UnormSrgb | wgpu::TextureFormat::Bgra8Unorm
        );
        let mut out = Vec::with_capacity((w * h * 4) as usize);
        for y in 0..h {
            let line = &data[(y * row) as usize..(y * row + w * 4) as usize];
            for px in line.as_chunks::<4>().0 {
                if bgra {
                    out.extend_from_slice(&[px[2], px[1], px[0], 255]);
                } else {
                    out.extend_from_slice(&[px[0], px[1], px[2], 255]);
                }
            }
        }
        out
    }
}
