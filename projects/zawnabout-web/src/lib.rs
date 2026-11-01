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
            let gfx = Gfx::new(Renderer::new(device, queue, view_format, size), &game);
            Ok(App {
                game,
                gfx,
                surface,
                config,
                script: None,
                logs: Vec::new(),
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
            host.join("\n")
        }

        pub fn key(&mut self, code: &str, down: bool, repeat: bool) {
            self.game.key(code, down, repeat);
        }

        /// Releases all held input, such as when the page loses focus.
        pub fn blur(&mut self) {
            self.game.input.clear();
        }

        pub fn gamepad(&mut self, axes: &[f32], buttons: &[f32]) {
            self.game.input.gamepad(axes, buttons);
        }

        pub fn touch_stick(&mut self, x: f32, y: f32) {
            self.game.input.touch_stick(glam_vec2(x, y));
        }

        pub fn touch_action(&mut self, name: &str, down: bool) {
            if let Some(action) = Action::from_name(name) {
                self.game.touch_action(action, down);
            }
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
