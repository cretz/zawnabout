//! Browser host. JavaScript owns the page, the frame loop and the devices, and calls in
//! here each frame. See `src/main.ts`.

#[cfg(target_arch = "wasm32")]
mod web {
    use wasm_bindgen::prelude::*;
    use zawnabout_game::input::Action;
    use zawnabout_game::{Game, Gfx, Output, Renderer, Script, wgpu};

    fn js_err(e: impl std::fmt::Display) -> JsValue {
        JsValue::from_str(&e.to_string())
    }

    #[wasm_bindgen]
    pub struct App {
        game: Game,
        gfx: Gfx,
        surface: wgpu::Surface<'static>,
        config: wgpu::SurfaceConfiguration,
        script: Option<Script>,
        logs: Vec<String>,
        /// The world's sound, filled in by the page's audio node once it starts.
        synth: zawnabout_game::audio::Synth,
        sound: bool,
    }

    #[wasm_bindgen]
    impl App {
        pub async fn create(canvas: web_sys::HtmlCanvasElement, seed: u32) -> Result<App, JsValue> {
            console_error_panic_hook::set_once();
            let size = (canvas.width().max(1), canvas.height().max(1));
            let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
                backends: wgpu::Backends::BROWSER_WEBGPU,
                ..wgpu::InstanceDescriptor::new_without_display_handle()
            });
            let surface = instance
                .create_surface(wgpu::SurfaceTarget::Canvas(canvas))
                .map_err(js_err)?;
            let adapter = instance
                .request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::HighPerformance,
                    compatible_surface: Some(&surface),
                    ..Default::default()
                })
                .await
                .map_err(js_err)?;
            let (device, queue) = adapter
                .request_device(&wgpu::DeviceDescriptor::default())
                .await
                .map_err(js_err)?;
            let caps = surface.get_capabilities(&adapter);
            let format = caps.formats[0];
            let view_format = zawnabout_game::srgb_view_format(format);
            let config = wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format,
                width: size.0,
                height: size.1,
                present_mode: wgpu::PresentMode::AutoVsync,
                desired_maximum_frame_latency: 2,
                alpha_mode: caps.alpha_modes[0],
                view_formats: vec![view_format],
                color_space: wgpu::SurfaceColorSpace::Auto,
            };
            surface.configure(&device, &config);
            let game = Game::new(seed as u64);
            let mut gfx = Gfx::new(Renderer::new(device, queue, view_format, size), &game);
            // One terrain chunk per frame keeps phones smooth while flying.
            gfx.chunk_budget = 1;
            Ok(App {
                game,
                gfx,
                surface,
                config,
                script: None,
                logs: Vec::new(),
                synth: zawnabout_game::audio::Synth::new(),
                sound: false,
            })
        }

        /// Advances and draws one frame. Returns host commands from a running script
        /// (such as `shot`), one per line, for the page to handle.
        pub fn frame(&mut self, dt: f32, width: u32, height: u32) -> String {
            if (width, height) != (self.config.width, self.config.height) && width > 0 && height > 0
            {
                self.config.width = width;
                self.config.height = height;
                self.surface
                    .configure(&self.gfx.renderer.device, &self.config);
                self.gfx.renderer.resize((width, height));
            }
            let mut host = Vec::new();
            if let Some(script) = &mut self.script {
                for o in script.update(&mut self.game, dt) {
                    match o {
                        Output::Log(msg) => self.logs.push(msg),
                        Output::Host(words) => host.push(words.join(" ")),
                    }
                }
                if script.done() {
                    self.script = None;
                }
            }
            self.game.advance(dt);
            let tex = match self.surface.get_current_texture() {
                wgpu::CurrentSurfaceTexture::Success(t)
                | wgpu::CurrentSurfaceTexture::Suboptimal(t) => t,
                wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                    self.surface
                        .configure(&self.gfx.renderer.device, &self.config);
                    return host.join("\n");
                }
                _ => return host.join("\n"),
            };
            let view = tex.texture.create_view(&wgpu::TextureViewDescriptor {
                format: Some(self.gfx.renderer.format),
                ..Default::default()
            });
            self.gfx.render(&self.game, &view);
            self.gfx.renderer.queue.present(tex);
            if self.sound {
                let mix = self.gfx.sound(&self.game);
                self.synth.set(mix);
            }
            host.join("\n")
        }

        /// Whether the sound is turned off, with U or the pause card.
        pub fn muted(&self) -> bool {
            self.game.muted
        }

        /// Turns the world's sound on or off. The page starts it on the first click or
        /// key, as browsers ask.
        pub fn set_sound(&mut self, on: bool) {
            self.sound = on;
        }

        /// Fills interleaved stereo samples at `rate` per second, for the page's audio.
        pub fn sound_fill(&mut self, out: &mut [f32], rate: f32) {
            if self.sound {
                self.synth.fill(out, rate);
            } else {
                out.fill(0.0);
            }
        }

        pub fn key(&mut self, code: &str, down: bool, repeat: bool) {
            self.game.key(code, down, repeat);
        }

        /// Releases all held input, such as when the page loses focus.
        pub fn blur(&mut self) {
            self.game.release_keys();
        }

        pub fn gamepad(&mut self, axes: &[f32], buttons: &[f32]) {
            self.game.input.gamepad(axes, buttons);
        }

        /// Looks around by a drag on the screen, in CSS pixels.
        pub fn look_drag(&mut self, dx: f32, dy: f32) {
            self.game.look_drag(dx, dy);
        }

        pub fn touch_stick(&mut self, x: f32, y: f32) {
            self.game.input.touch_stick(glam_vec2(x, y));
        }

        pub fn touch_action(&mut self, name: &str, down: bool) {
            if let Some(action) = Action::from_name(name) {
                self.game.touch_action(action, down);
            }
        }

        /// A tap or click at normalized coordinates (0..1, top left origin). Flies to and
        /// lands on whatever is there.
        pub fn tap(&mut self, nx: f32, ny: f32) -> bool {
            let aspect = self.config.width as f32 / self.config.height.max(1) as f32;
            self.game.tap(nx, ny, aspect)
        }

        /// Enters or leaves notes mode: everything frozen, a free camera, note pins.
        pub fn notes(&mut self, on: bool) {
            self.game.set_feedback(on);
        }

        pub fn notes_active(&self) -> bool {
            self.game.feedback.active
        }

        /// Turns the notes camera by a drag in CSS pixels.
        pub fn notes_look(&mut self, dx: f32, dy: f32) {
            self.game.feedback_look(dx, dy);
        }

        /// The spot under normalized coordinates in notes mode, as JSON with `spot`,
        /// `view` and `near`, or nothing when no surface is there.
        pub fn notes_pick(&mut self, nx: f32, ny: f32) -> Option<String> {
            let aspect = self.config.width as f32 / self.config.height.max(1) as f32;
            self.game.feedback_pick(nx, ny, aspect)
        }

        /// Pins to draw: x, y, z per note and a done flag per note.
        pub fn set_pins(&mut self, xyz: &[f32], done: &[u8]) {
            self.game.set_pins(xyz, done);
        }

        /// World square and soft edge width: min x, min z, max x, max z, edge.
        pub fn world_bounds(&self) -> Vec<f32> {
            let def = self.game.sim.world.terrain.def;
            let b = def.bounds;
            vec![b[0], b[1], b[2], b[3], def.edge]
        }

        /// The whole world as a shaded map, `size` square RGBA8, north up.
        pub fn map_image(&self, size: u32) -> Vec<u8> {
            zawnabout_game::map::render(&self.game.sim.world.terrain, size, false)
        }

        /// A square of the world for a close look on the big map: corner, width (m) and
        /// pixels across, RGBA8.
        pub fn map_area(&self, x0: f32, z0: f32, span: f32, size: u32) -> Vec<u8> {
            zawnabout_game::map::render_area(
                &self.game.sim.world.terrain,
                x0,
                z0,
                span,
                size,
                false,
            )
        }

        /// Places for the big map, one per line: kind, id, x, z, under (see
        /// `map::places`).
        pub fn map_places(&self) -> String {
            zawnabout_game::map::places(&self.game.sim.world.terrain)
        }

        /// What the map arrow shows: x, z and heading of the dragon, or of the camera in
        /// notes mode.
        pub fn map_marker(&self) -> Vec<f32> {
            if self.game.feedback.active {
                let cam = self.game.feedback.cam;
                vec![cam.pos.x, cam.pos.z, cam.yaw]
            } else {
                let d = self.game.player_dragon();
                vec![d.pos.x, d.pos.z, d.heading]
            }
        }

        /// The keys, one action a line: `flap Space KeyV`.
        pub fn bindings_text(&self) -> String {
            self.game.bindings.to_text()
        }

        /// Keys saved from [`App::bindings_text`].
        pub fn set_bindings_text(&mut self, text: &str) {
            self.game.bindings.set_text(text);
        }

        /// Makes `code` the one key for the action named `action` (`flap`, `up`...).
        pub fn bind_key(&mut self, action: &str, code: &str) {
            if let Some(a) = zawnabout_game::input::Action::from_name(action) {
                self.game.bindings.rebind(a, code);
            }
        }

        /// Back to the keys the game came with.
        pub fn reset_keys(&mut self) {
            self.game.bindings = zawnabout_game::input::Bindings::default();
        }

        /// Runs a debug command and returns its output, or `error: ...`.
        pub fn command(&mut self, line: &str) -> String {
            match self.game.command(line) {
                Ok(msg) => msg,
                Err(e) => format!("error: {e}"),
            }
        }

        /// Starts a debug script that runs in real time alongside play.
        pub fn run_script(&mut self, text: &str) {
            self.script = Some(Script::new(text));
        }

        pub fn script_running(&self) -> bool {
            self.script.is_some()
        }

        pub fn take_logs(&mut self) -> String {
            std::mem::take(&mut self.logs).join("\n")
        }

        pub fn params_json(&self) -> String {
            self.game.params_json()
        }

        pub fn dump(&self) -> String {
            self.game.dump()
        }

        pub fn state(&self) -> String {
            self.game.state()
        }

        pub fn take_trace(&mut self) -> Option<String> {
            self.game.take_trace()
        }
    }

    fn glam_vec2(x: f32, y: f32) -> zawnabout_game::sim::glam::Vec2 {
        zawnabout_game::sim::glam::Vec2::new(x, y)
    }
}
