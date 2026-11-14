//! CPU time per part of a frame. Headless runs profile with it, and the web page shows it
//! on the device itself with `?perf`. Off until [`enable`], so it costs nothing in play.

use std::cell::{Cell, RefCell};

/// Parts by name, with their seconds.
type Parts = Vec<(&'static str, f64)>;

thread_local! {
    static ON: Cell<bool> = const { Cell::new(false) };
    /// Each part's name, total seconds, how many times it ran and its slowest run.
    static PARTS: RefCell<Vec<(&'static str, f64, u32, f64)>> = const { RefCell::new(Vec::new()) };
    /// Each part's seconds in the frame so far.
    static FRAME: RefCell<Parts> = const { RefCell::new(Vec::new()) };
    /// When this frame began, and the slowest frame since the last report with its parts.
    static FRAMES: RefCell<(f64, f64, Parts)> = const { RefCell::new((0.0, 0.0, Vec::new())) };
}

/// Starts timing.
pub fn enable() {
    ON.with(|on| on.set(true));
}

fn on() -> bool {
    ON.with(Cell::get)
}

/// Seconds on a steady clock.
pub fn clock() -> f64 {
    #[cfg(not(target_arch = "wasm32"))]
    {
        static START: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
        START
            .get_or_init(std::time::Instant::now)
            .elapsed()
            .as_secs_f64()
    }
    #[cfg(target_arch = "wasm32")]
    {
        web_sys::window()
            .and_then(|w| w.performance())
            .map_or(0.0, |p| p.now() / 1000.0)
    }
}

/// Adds `secs` to part `name`.
fn record(name: &'static str, secs: f64) {
    PARTS.with(|parts| {
        let mut parts = parts.borrow_mut();
        match parts.iter_mut().find(|p| p.0 == name) {
            Some(p) => {
                p.1 += secs;
                p.2 += 1;
                p.3 = p.3.max(secs);
            }
            None => parts.push((name, secs, 1, secs)),
        }
    });
    FRAME.with(|frame| {
        let mut frame = frame.borrow_mut();
        match frame.iter_mut().find(|p| p.0 == name) {
            Some(p) => p.1 += secs,
            None => frame.push((name, secs)),
        }
    });
}

/// Adds `secs` to part `name`, for a time measured some other way, such as the GPU
/// finishing a frame.
pub fn add(name: &'static str, secs: f64) {
    if on() {
        record(name, secs);
    }
}

/// Whether timing is on.
pub fn enabled() -> bool {
    on()
}

/// Runs `f`, adding the time it took to part `name`.
pub fn time<T>(name: &'static str, f: impl FnOnce() -> T) -> T {
    if !on() {
        return f();
    }
    let start = clock();
    let out = f();
    record(name, clock() - start);
    out
}

/// Times the steps of one long job in turn: each [`Laps::lap`] adds the time since the
/// last one to its part.
pub struct Laps {
    last: Option<f64>,
}

impl Laps {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            last: on().then(clock),
        }
    }

    pub fn lap(&mut self, name: &'static str) {
        if let Some(last) = &mut self.last {
            let t = clock();
            record(name, t - *last);
            *last = t;
        }
    }
}

/// Marks the start of a frame.
pub fn begin_frame() {
    if on() {
        FRAME.with(|f| f.borrow_mut().clear());
        FRAMES.with(|f| f.borrow_mut().0 = clock());
    }
}

/// Marks the end of a frame, keeping its parts if it is the slowest yet.
pub fn end_frame() {
    if !on() {
        return;
    }
    let parts = FRAME.with(|f| std::mem::take(&mut *f.borrow_mut()));
    FRAMES.with(|f| {
        let mut f = f.borrow_mut();
        let secs = clock() - f.0;
        if secs > f.1 {
            f.1 = secs;
            f.2 = parts;
        }
    });
}

/// Average milliseconds per run of every part since the last report, the slowest run and
/// how many runs, slowest on average first, then the slowest frame and what took its time,
/// each part after a semicolon.
/// Starts counting afresh.
pub fn report() -> String {
    let mut parts = PARTS.with(|parts| std::mem::take(&mut *parts.borrow_mut()));
    parts.sort_by(|a, b| (b.1 / b.2 as f64).total_cmp(&(a.1 / a.2 as f64)));
    let average = parts
        .iter()
        .map(|(name, secs, n, max)| {
            format!(
                "{name} {:.3}ms (max {:.2}) x{n}",
                secs / *n as f64 * 1000.0,
                max * 1000.0
            )
        })
        .collect::<Vec<_>>()
        .join("; ");
    let (worst, mut most) = FRAMES.with(|f| {
        let mut f = f.borrow_mut();
        let out = (f.1, std::mem::take(&mut f.2));
        f.1 = 0.0;
        out
    });
    most.sort_by(|a, b| b.1.total_cmp(&a.1));
    let most = most
        .iter()
        .filter(|p| p.1 >= 0.00005)
        .map(|(name, secs)| format!("{name} {:.2}", secs * 1000.0))
        .collect::<Vec<_>>()
        .join("; ");
    format!("{average}\n  worst frame {:.2}ms: {most}", worst * 1000.0)
}
