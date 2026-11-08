//! Desktop host. Runs windowed for play, or `--headless` to run a debug script and write
//! screenshots without a window.
//!
//! zawnabout [--headless] [--script FILE] [--out DIR] [--size WxH] [--seed N] [--debug] [--mute] [--software]
//!           [--no-play]
//! zawnabout --map FILE [--size WxH]
//!
//! `--debug` reads commands from stdin while windowed. F12 saves a screenshot to `--out`.
//! `--no-play` turns gameplay off, leaving the islands alone.
//! `--map` writes a top-down shaded map of the world, `--size` width wide, and exits.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, mpsc};
use std::time::Instant;

use gilrs::{Axis, Button, Gilrs};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::PhysicalKey;
use winit::window::{Window, WindowId};
use zawnabout_game::audio::Synth;
use zawnabout_game::game::Ask;
use zawnabout_game::hud::{HudKey, NoteChange};
use zawnabout_game::notes::{Note, note_stamp};
use zawnabout_game::{Game, Gfx, Output, Renderer, Script, wgpu};

const FRAME: f32 = 1.0 / 60.0;

/// Where the player's things are kept between runs: keys, settings and saves.
fn data_file(name: &str) -> Option<PathBuf> {
    let base = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
    Some(base.join("zawnabout").join(name))
}

fn keys_file() -> Option<PathBuf> {
    data_file("keys.txt")
}

/// The save slot. One for now, named so more can come.
const SLOT: &str = "save-a.txt";

fn write_data(name: &str, text: &str) {
    if let Some(f) = data_file(name) {
        let _ = std::fs::create_dir_all(f.parent().unwrap_or(Path::new(".")));
        let _ = std::fs::write(f, text);
    }
}

/// Does what the game asked: keeps its save, settings and keys, and offers files for
/// export and import.
fn answer_asks(game: &mut Game) {
    for ask in std::mem::take(&mut game.asks) {
        match ask {
            Ask::Save => {
                if let Some(text) = game.save_text() {
                    write_data(SLOT, &text);
                }
            }
            Ask::Prefs => write_data("prefs.txt", &game.prefs_text()),
            Ask::Keys => write_data("keys.txt", &game.bindings.to_text()),
            Ask::Export => {
                let Some(text) = game.save_text() else {
                    continue;
                };
                if let Some(path) = rfd::FileDialog::new()
                    .set_title("Save your game")
                    .set_file_name("zawnabout-save.txt")
                    .add_filter("zawnabout save", &["txt"])
                    .save_file()
                {
                    let _ = std::fs::write(path, text);
                }
            }
            Ask::Source => {
                let url = zawnabout_game::book::SOURCE;
                #[cfg(windows)]
                let open = std::process::Command::new("explorer").arg(url).spawn();
                #[cfg(target_os = "macos")]
                let open = std::process::Command::new("open").arg(url).spawn();
                #[cfg(not(any(windows, target_os = "macos")))]
                let open = std::process::Command::new("xdg-open").arg(url).spawn();
                if let Err(e) = open {
                    eprintln!("open {url}: {e}");
                }
            }
            Ask::Import => {
                if let Some(path) = rfd::FileDialog::new()
                    .set_title("Load a game")
                    .add_filter("zawnabout save", &["txt"])
                    .pick_file()
                {
                    match std::fs::read_to_string(path)
                        .map_err(|e| e.to_string())
                        .and_then(|t| game.load_save(&t))
                    {
                        Ok(()) => game.asks.push(Ask::Save),
                        Err(e) => eprintln!("load: {e}"),
                    }
                }
            }
        }
    }
}

/// Where design notes go: `working-area/feedback` beside the repo when the game runs from
/// it, as the dev server does, or else a folder next to the saved keys.
fn notes_dir() -> PathBuf {
    let starts = [std::env::current_exe().ok(), std::env::current_dir().ok()];
    for start in starts.into_iter().flatten() {
        for dir in start.ancestors() {
            let f = dir.join("working-area").join("feedback");
            if f.is_dir() {
                return f;
            }
        }
    }
    keys_file()
        .and_then(|f| Some(f.parent()?.join("notes")))
        .unwrap_or_else(|| PathBuf::from("notes"))
}

/// Every note in the folder, oldest first.
fn load_notes(dir: &Path) -> Vec<Note> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .map(|d| d.flatten().map(|e| e.path()).collect())
        .unwrap_or_default();
    files.retain(|f| f.extension().is_some_and(|e| e == "json"));
    files.sort();
    files
        .iter()
        .filter_map(|f| Note::from_json(&std::fs::read_to_string(f).ok()?))
        .collect()
}

/// Saves new notes and deletes old ones as the HUD asks, then lists them again.
fn sync_notes(game: &mut Game, dir: &Path) {
    let changes = std::mem::take(&mut game.hud.outbox);
    if changes.is_empty() {
        return;
    }
    let _ = std::fs::create_dir_all(dir);
    for change in changes {
        match change {
            NoteChange::Save(mut note) => {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default();
                (note.id, note.created) = note_stamp(now.as_secs(), now.subsec_nanos() as u64);
                let _ = std::fs::write(dir.join(format!("{}.json", note.id)), note.to_json());
            }
            NoteChange::Delete(id) => {
                // Only names the game made, so nothing outside the folder goes.
                if !id.is_empty() && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
                    let _ = std::fs::remove_file(dir.join(format!("{id}.json")));
                }
            }
        }
    }
    game.set_notes(load_notes(dir));
}

struct Args {
    headless: bool,
    debug: bool,
    script: Option<PathBuf>,
    out: PathBuf,
    size: (u32, u32),
    seed: u64,
    map: Option<PathBuf>,
    mute: bool,
    no_play: bool,
    /// The software GPU even where there is a real one, to stand in for a slow device.
    software: bool,
}

fn parse_args() -> Args {
    let mut a = Args {
        headless: false,
        debug: false,
        script: None,
        out: PathBuf::from("."),
        size: (1280, 720),
        seed: 1,
        map: None,
        mute: false,
        no_play: false,
        software: false,
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
            "--map" => a.map = Some(val().into()),
            "--mute" => a.mute = true,
            "--no-play" => a.no_play = true,
            "--software" => a.software = true,
            _ => panic!("unknown argument {arg}"),
        }
    }
    a
}

fn main() {
    let args = parse_args();
    if let Some(path) = &args.map {
        let size = args.size.0;
        let rgba = zawnabout_game::map::render(
            &zawnabout_game::sim::terrain::Terrain::default(),
            size,
            true,
        );
        save_png(path, &rgba, (size, size));
    } else if args.headless {
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
    software: bool,
) -> (wgpu::Adapter, wgpu::Device, wgpu::Queue) {
    let ask = |fallback: bool| {
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: surface,
            force_fallback_adapter: fallback,
            ..Default::default()
        }))
    };
    // With no GPU (a CI machine), a software one: WARP on Windows, lavapipe on Linux.
    let adapter = ask(software)
        .or_else(|_| ask(true))
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
        "hide-mesh" => {
            gfx.renderer.hide = words[1..].iter().filter_map(|w| w.parse().ok()).collect();
        }
        "stats" => {
            // Renders once so the numbers match the current view.
            let _ = gfx.capture(game);
            let s = &gfx.renderer.stats;
            println!(
                "stats draws={} (static {} terrain {} water {}) triangles={} terrain={} props={} water={} meshes={}",
                s.draws,
                s.draws_static,
                s.draws_terrain,
                s.draws_water,
                s.triangles,
                s.terrain,
                s.props,
                s.water,
                s.meshes
            );
            if words.get(1).is_some_and(|w| w == "detail") {
                let mut top = s.heaviest.clone();
                top.sort_by_key(|t| std::cmp::Reverse(t.2));
                for (mesh, count, tris) in
                    top.iter()
                        .take(if words.get(2).is_some() { 400 } else { 30 })
                {
                    println!("  mesh {mesh} x{count} = {tris} triangles");
                }
            }
        }
        // clouds: the big clouds nearest the camera, and the weather where it is.
        "clouds" => {
            let (cam, time) = (game.camera.pos, game.time as f32);
            let w = gfx.weather();
            let here = w.here(cam, time);
            println!("clouds mist={:.2} rain={:.2}", here.mist, here.rain);
            for s in w.strikes(time + 60.0, 60.0) {
                println!(
                    "  strike at {:.0} {:.0} {:.0} in {:.1}s",
                    s.at.x,
                    s.at.y,
                    s.at.z,
                    s.time - time
                );
            }
            for (at, half, rain) in w.nearest(cam, time, 6) {
                println!(
                    "  cloud at {:.0} {:.0} {:.0} half {:.0} {:.0} {:.0}{}",
                    at.x,
                    at.y,
                    at.z,
                    half.x,
                    half.y,
                    half.z,
                    if rain { " rain" } else { "" }
                );
            }
        }
        // sound: what the world sounds like here, as the listener mixes it.
        "sound" => {
            let m = gfx.sound(game);
            println!(
                "sound master={:.0} wind={:.2} rush={:.2} surf={:.2} rain={:.2} falls={:.2} bustle={:.2} birds={:.2} organ={:.2} events={:?}",
                m.master,
                m.wind,
                m.rush,
                m.surf,
                m.rain,
                m.falls,
                m.bustle,
                m.birds,
                m.organ,
                m.events
            );
        }
        "profile" => {
            let frames = words.get(1).and_then(|n| n.parse().ok()).unwrap_or(300);
            println!("profile {}", gfx.profile(game, frames));
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
    let (adapter, device, queue) = device(&instance, None, args.software);
    let info = adapter.get_info();
    println!(
        "adapter {} ({:?}, {:?})",
        info.name, info.device_type, info.backend
    );
    let mut game = Game::new(args.seed);
    if args.no_play {
        game.play = None;
    }
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
        gfx.observe(&game);
        if game.wants_snap() {
            let (w, h) = gfx.renderer.size();
            let rgba = gfx.capture(&game);
            game.take_snap(&rgba, w as usize, h as usize);
        }
    }
    if script.failures > 0 {
        println!("{} playbook failure(s)", script.failures);
        std::process::exit(1);
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
    cursor: (f64, f64),
    /// The right mouse button is held, looking round.
    looking: bool,
    shots: u32,
    title_at: f32,
    notes: PathBuf,
    /// The world's sound, played on the default output, and the stream playing it.
    synth: Option<Arc<Mutex<Synth>>>,
    stream: Option<cpal::Stream>,
}

/// Starts sound on the default output device, if there is one that takes floats.
fn start_sound(synth: Arc<Mutex<Synth>>) -> Option<cpal::Stream> {
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
    let device = cpal::default_host().default_output_device()?;
    let config = device.default_output_config().ok()?;
    if config.sample_format() != cpal::SampleFormat::F32 {
        println!("sound: output is not float samples, staying quiet");
        return None;
    }
    let channels = config.channels() as usize;
    let rate = config.sample_rate().0 as f32;
    let mut stereo = Vec::new();
    let stream = device
        .build_output_stream(
            &config.into(),
            move |out: &mut [f32], _| {
                let frames = out.len() / channels.max(1);
                stereo.resize(frames * 2, 0.0);
                if let Ok(mut s) = synth.lock() {
                    s.fill(&mut stereo, rate);
                }
                for (f, lr) in out.chunks_mut(channels).zip(stereo.chunks(2)) {
                    for (c, v) in f.iter_mut().enumerate() {
                        *v = lr[c.min(1)];
                    }
                }
            },
            |e| eprintln!("sound: {e}"),
            None,
        )
        .ok()?;
    stream.play().ok()?;
    Some(stream)
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
        let mut game = Game::new(args.seed);
        if args.no_play {
            game.play = None;
        }
        Self {
            game,
            args,
            win: None,
            gilrs: Gilrs::new().ok(),
            last: Instant::now(),
            script,
            stdin,
            cursor: (0.0, 0.0),
            looking: false,
            shots: 0,
            title_at: 0.0,
            notes: notes_dir(),
            synth: None,
            stream: None,
        }
    }

    fn poll_gamepad(&mut self) {
        let Some(gilrs) = &mut self.gilrs else { return };
        while gilrs.next_event().is_some() {}
        let Some((_, pad)) = gilrs.gamepads().next() else {
            self.game.gamepad(&[], &[]);
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
        self.game.gamepad(&axes, &buttons);
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
        answer_asks(&mut self.game);
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
        // A postcard's view: read it back as its picture.
        if self.game.wants_snap() {
            let (w, h) = win.gfx.renderer.size();
            let rgba = win.gfx.capture(&self.game);
            self.game.take_snap(&rgba, w as usize, h as usize);
        }
        if let Some(s) = &self.synth {
            let mix = win.gfx.sound(&self.game);
            if let Ok(mut s) = s.lock() {
                s.set(mix);
            }
        }
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
        let (adapter, device, queue) = device(&instance, Some(&surface), self.args.software);
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
        // Played in a window, the game draws its own map and notes, with the player's
        // keys, settings and saved game, and starts on the journal, paused.
        self.game.hud.on = true;
        if let Some(text) = keys_file().and_then(|f| std::fs::read_to_string(f).ok()) {
            self.game.bindings.set_text(&text);
        }
        if let Some(text) = data_file("prefs.txt").and_then(|f| std::fs::read_to_string(f).ok()) {
            self.game.set_prefs_text(&text);
        }
        if let Some(text) = data_file(SLOT).and_then(|f| std::fs::read_to_string(f).ok())
            && let Err(e) = self.game.load_save(&text)
        {
            eprintln!("save: {e}");
        }
        if self.script.is_none() && !self.args.debug {
            self.game.open_book();
        }
        if !self.args.mute {
            let synth = Arc::new(Mutex::new(Synth::new()));
            self.stream = start_sound(synth.clone());
            if self.stream.is_some() {
                self.synth = Some(synth);
            }
        }
        self.game.hud.notes_where = format!("saved to {}", self.notes.display());
        self.game.set_notes(load_notes(&self.notes));
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
            // Closing keeps the game where it is, to carry on next time.
            WindowEvent::CloseRequested => {
                if self.script.is_none()
                    && let Some(text) = self.game.save_text()
                {
                    write_data(SLOT, &text);
                }
                el.exit()
            }
            WindowEvent::Resized(s) => self.resize(s.width, s.height),
            // Leaving the window opens the journal, as the web page does, unless that
            // is turned off.
            WindowEvent::Focused(false) => {
                self.game.release_keys();
                if !self.args.debug && self.game.pause_away && !self.game.feedback.active {
                    self.game.open_book();
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                // Holding the right button looks round.
                if self.looking {
                    let (dx, dy) = (position.x - self.cursor.0, position.y - self.cursor.1);
                    if self.game.feedback.active {
                        self.game.feedback_look(dx as f32, dy as f32);
                    } else {
                        self.game.look_drag(dx as f32, dy as f32);
                    }
                }
                self.cursor = (position.x, position.y);
                if let Some(win) = &self.win {
                    let (w, h) = (win.config.width as f64, win.config.height as f64);
                    self.game
                        .ui_hover((position.x / w) as f32, (position.y / h) as f32);
                }
            }
            WindowEvent::MouseInput {
                state,
                button: winit::event::MouseButton::Right,
                ..
            } => self.looking = state.is_pressed(),
            WindowEvent::MouseInput {
                state: winit::event::ElementState::Pressed,
                button: winit::event::MouseButton::Left,
                ..
            } => {
                if let Some(win) = &self.win {
                    let (w, h) = (win.config.width as f64, win.config.height as f64);
                    let (nx, ny, aspect) = (
                        (self.cursor.0 / w) as f32,
                        (self.cursor.1 / h) as f32,
                        (w / h) as f32,
                    );
                    // The journal and cards take clicks first. In notes mode a click starts a
                    // note there, otherwise it lands there.
                    if self.game.ui_click(nx, ny) {
                        answer_asks(&mut self.game);
                    } else if self.game.feedback.active {
                        self.game.notes_pick(nx, ny, aspect);
                    } else if !self.game.halted() {
                        self.game.tap(nx, ny, aspect);
                    }
                }
            }
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
                // The journal, cards, map and notes take keys first, and pause the game
                // while they are up. Held keys repeat only while writing a note.
                let writing = self.game.hud.writing.is_some();
                if event.state.is_pressed() && (!event.repeat || writing) {
                    let in_notes = self.game.feedback.active;
                    if self.game.ui_key(&name, event.text.as_deref()) == HudKey::Used {
                        sync_notes(&mut self.game, &self.notes);
                        // Entering notes mode reads the folder again, for replies.
                        if !in_notes && self.game.feedback.active {
                            self.game.set_notes(load_notes(&self.notes));
                        }
                        answer_asks(&mut self.game);
                        return;
                    }
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
