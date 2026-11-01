//! Desktop host. Runs windowed for play, or `--headless` to run a debug script and write
//! screenshots without a window.
//!
//! zawnabout [--headless] [--script FILE] [--out DIR] [--size WxH] [--seed N] [--debug]
//!
//! `--debug` reads commands from stdin while windowed. F12 saves a screenshot to `--out`.

use std::path::{Path, PathBuf};
use std::sync::{Arc, mpsc};
use std::time::Instant;

use gilrs::{Axis, Button, Gilrs};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::PhysicalKey;
use winit::window::{Window, WindowId};
use zawnabout_game::{Game, Gfx, Output, Renderer, Script, wgpu};

const FRAME: f32 = 1.0 / 60.0;

struct Args {
    headless: bool,
    debug: bool,
    script: Option<PathBuf>,
    out: PathBuf,
    size: (u32, u32),
    seed: u64,
}

fn parse_args() -> Args {
    let mut a = Args {
        headless: false,
        debug: false,
        script: None,
        out: PathBuf::from("."),
        size: (1280, 720),
        seed: 1,
    };
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        let mut val = || it.next().unwrap_or_else(|| panic!("{arg} needs a value"));
        match arg.as_str() {
            "--headless" => a.headless = true,
            "--debug" => a.debug = true,
            "--script" => a.script = Some(val().into()),
            "--out" => a.out = val().into(),
            "--size" => {
                let v = val();
                let (w, h) = v.split_once('x').expect("--size WxH");
                a.size = (w.parse().expect("width"), h.parse().expect("height"));
            }
            "--seed" => a.seed = val().parse().expect("seed"),
            _ => panic!("unknown argument {arg}"),
        }
    }
    a
}

fn main() {
    let args = parse_args();
    if args.headless {
        headless(args);
    } else {
        let event_loop = EventLoop::new().expect("event loop");
        let mut app = App::new(args);
        event_loop.run_app(&mut app).expect("run app");
    }
}

fn device(
    instance: &wgpu::Instance,
    surface: Option<&wgpu::Surface>,
) -> (wgpu::Adapter, wgpu::Device, wgpu::Queue) {
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: surface,
        force_fallback_adapter: false,
        ..Default::default()
    }))
    .expect("no GPU adapter");
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("no GPU device");
    (adapter, device, queue)
}

fn save_png(path: &Path, rgba: &[u8], (w, h): (u32, u32)) {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).expect("create output dir");
    }
    let file = std::fs::File::create(path).expect("create png");
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), w, h);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header()
        .expect("png header")
        .write_image_data(rgba)
        .expect("png data");
    println!("shot {}", path.display());
}

/// Handles a host command from a script. Returns false to quit.
fn host_command(
    words: &[String],
    game: &mut Game,
    gfx: &mut Gfx,
    out: &Path,
    shots: &mut u32,
) -> bool {
    match words[0].as_str() {
        "shot" => {
            let name = words
                .get(1)
                .cloned()
                .unwrap_or_else(|| format!("shot-{:03}.png", *shots));
            *shots += 1;
            let rgba = gfx.capture(game);
            save_png(&out.join(name), &rgba, gfx.renderer.size());
        }
        "size" => {
            let (w, h) = (words.get(1), words.get(2));
            if let (Some(w), Some(h)) = (
                w.and_then(|w| w.parse().ok()),
                h.and_then(|h| h.parse().ok()),
            ) {
                gfx.renderer.resize((w, h));
            }
        }
        "save-trace" => {
            let path = out.join(words.get(1).map(String::as_str).unwrap_or("trace.csv"));
            let csv = game.take_trace().unwrap_or_default();
            std::fs::create_dir_all(out).expect("create output dir");
            std::fs::write(&path, csv).expect("write trace");
            println!("trace {}", path.display());
        }
        "quit" => return false,
        _ => {}
    }
    true
}

fn headless(args: Args) {
    let script_path = args.script.expect("--headless needs --script");
    let text = std::fs::read_to_string(&script_path).expect("read script");
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let (_, device, queue) = device(&instance, None);
    let mut game = Game::new(args.seed);
    let renderer = Renderer::new(
        device,
        queue,
        wgpu::TextureFormat::Rgba8UnormSrgb,
        args.size,
    );
    let mut gfx = Gfx::new(renderer, &game);
    let mut script = Script::new(&text);
    let mut shots = 0;
    while !script.done() {
        for o in script.update(&mut game, FRAME) {
            match o {
                Output::Log(msg) => println!("{msg}"),
                Output::Host(words) => {
                    if !host_command(&words, &mut game, &mut gfx, &args.out, &mut shots) {
                        return;
                    }
                }
            }
        }
        game.advance(FRAME);
    }
}

struct Window_ {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    gfx: Gfx,
}

struct App {
    args: Args,
    game: Game,
    win: Option<Window_>,
    gilrs: Option<Gilrs>,
    last: Instant,
    script: Option<Script>,
    stdin: Option<mpsc::Receiver<String>>,
    shots: u32,
    title_at: f32,
}

impl App {
    fn new(args: Args) -> Self {
        let script = args
            .script
            .as_ref()
            .map(|p| Script::new(&std::fs::read_to_string(p).expect("read script")));
        let stdin = args.debug.then(|| {
            let (tx, rx) = mpsc::channel();
            std::thread::spawn(move || {
                for line in std::io::stdin().lines().map_while(Result::ok) {
                    if tx.send(line).is_err() {
                        break;
                    }
                }
            });
            rx
        });
        Self {
            game: Game::new(args.seed),
            args,
            win: None,
            gilrs: Gilrs::new().ok(),
            last: Instant::now(),
            script,
            stdin,
            shots: 0,
            title_at: 0.0,
        }
    }

    fn poll_gamepad(&mut self) {
        let Some(gilrs) = &mut self.gilrs else { return };
        while gilrs.next_event().is_some() {}
        let Some((_, pad)) = gilrs.gamepads().next() else {
            self.game.input.gamepad(&[], &[]);
            return;
        };
        let axes = [
            pad.value(Axis::LeftStickX),
            -pad.value(Axis::LeftStickY),
            pad.value(Axis::RightStickX),
            -pad.value(Axis::RightStickY),
        ];
        // W3C standard gamepad button order.
        let order = [
            Button::South,
            Button::East,
            Button::West,
            Button::North,
            Button::LeftTrigger,
            Button::RightTrigger,
            Button::LeftTrigger2,
            Button::RightTrigger2,
            Button::Select,
            Button::Start,
            Button::LeftThumb,
            Button::RightThumb,
            Button::DPadUp,
            Button::DPadDown,
            Button::DPadLeft,
            Button::DPadRight,
        ];
        let buttons: Vec<f32> = order
            .iter()
            .map(|b| pad.button_data(*b).map(|d| d.value()).unwrap_or(0.0))
            .collect();
        self.game.input.gamepad(&axes, &buttons);
    }

    fn frame(&mut self) {
        let dt = self.last.elapsed().as_secs_f32();
        self.last = Instant::now();
        self.poll_gamepad();
        let Some(win) = &mut self.win else { return };

        let mut host = Vec::new();
        if let Some(script) = &mut self.script {
            for o in script.update(&mut self.game, dt) {
                match o {
                    Output::Log(msg) => println!("{msg}"),
                    Output::Host(words) => host.push(words),
                }
            }
        }
        if let Some(rx) = &self.stdin {
            for line in rx.try_iter() {
                let words: Vec<String> = line.split_whitespace().map(String::from).collect();
                if words.is_empty() {
                    continue;
                }
                if ["shot", "size", "save-trace", "quit"].contains(&words[0].as_str()) {
                    host.push(words);
                } else {
                    match self.game.command(&line) {
                        Ok(msg) => println!("{msg}"),
                        Err(e) => println!("error: {e}"),
                    }
                }
            }
        }
        for words in host {
            if !host_command(
                &words,
                &mut self.game,
                &mut win.gfx,
                &self.args.out,
                &mut self.shots,
            ) {
                std::process::exit(0);
            }
        }

        self.game.advance(dt);
        if self.args.debug {
            self.title_at -= dt;
            if self.title_at <= 0.0 {
                self.title_at = 0.25;
                win.window
                    .set_title(&format!("zawnabout  {}", self.game.state()));
            }
        }

        let tex = match win.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(t)
            | wgpu::CurrentSurfaceTexture::Suboptimal(t) => t,
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                win.surface.configure(&win.gfx.renderer.device, &win.config);
                return;
            }
            _ => return,
        };
        let view = tex.texture.create_view(&wgpu::TextureViewDescriptor {
            format: Some(win.gfx.renderer.format),
            ..Default::default()
        });
        win.gfx.render(&self.game, &view);
        win.window.pre_present_notify();
        win.gfx.renderer.queue.present(tex);
    }

    fn resize(&mut self, w: u32, h: u32) {
        let Some(win) = &mut self.win else { return };
        if w == 0 || h == 0 {
            return;
        }
        win.config.width = w;
        win.config.height = h;
        win.surface.configure(&win.gfx.renderer.device, &win.config);
        win.gfx.renderer.resize((w, h));
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, el: &ActiveEventLoop) {
        if self.win.is_some() {
            return;
        }
        let attrs = Window::default_attributes()
            .with_title("zawnabout")
            .with_inner_size(LogicalSize::new(self.args.size.0, self.args.size.1));
        let window = Arc::new(el.create_window(attrs).expect("create window"));
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let surface = instance
            .create_surface(window.clone())
            .expect("create surface");
        let (adapter, device, queue) = device(&instance, Some(&surface));
        let caps = surface.get_capabilities(&adapter);
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| {
                f.remove_srgb_suffix() == wgpu::TextureFormat::Bgra8Unorm
                    || f.remove_srgb_suffix() == wgpu::TextureFormat::Rgba8Unorm
            })
            .unwrap_or(caps.formats[0]);
        let view_format = zawnabout_game::srgb_view_format(format);
        let size = window.inner_size();
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::AutoVsync,
            desired_maximum_frame_latency: 2,
            alpha_mode: caps.alpha_modes[0],
            view_formats: if view_format != format {
                vec![view_format]
            } else {
                vec![]
            },
            color_space: wgpu::SurfaceColorSpace::Auto,
        };
        surface.configure(&device, &config);
        let renderer = Renderer::new(device, queue, view_format, (config.width, config.height));
        let gfx = Gfx::new(renderer, &self.game);
        self.win = Some(Window_ {
            window,
            surface,
            config,
            gfx,
        });
        self.last = Instant::now();
    }

    fn window_event(&mut self, el: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => el.exit(),
            WindowEvent::Resized(s) => self.resize(s.width, s.height),
            WindowEvent::Focused(false) => self.game.input.clear(),
            WindowEvent::KeyboardInput { event, .. } => {
                let PhysicalKey::Code(code) = event.physical_key else {
                    return;
                };
                let name = format!("{code:?}");
                if name == "F12" && event.state.is_pressed() {
                    if let Some(win) = &mut self.win {
                        let words = vec!["shot".to_string()];
                        host_command(
                            &words,
                            &mut self.game,
                            &mut win.gfx,
                            &self.args.out,
                            &mut self.shots,
                        );
                    }
                    return;
                }
                self.game.key(&name, event.state.is_pressed(), event.repeat);
            }
            WindowEvent::RedrawRequested => self.frame(),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _: &ActiveEventLoop) {
        if let Some(win) = &self.win {
            win.window.request_redraw();
        }
    }
}
