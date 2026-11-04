//! CPU time per part of a frame, for profiling headless runs. Native only: on the web the
//! timers do nothing.

use std::cell::RefCell;

thread_local! {
    /// Each part's name, total seconds and how many times it ran.
    static PARTS: RefCell<Vec<(&'static str, f64, u32)>> = const { RefCell::new(Vec::new()) };
}

/// Runs `f`, adding the time it took to part `name`.
pub fn time<T>(name: &'static str, f: impl FnOnce() -> T) -> T {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let start = std::time::Instant::now();
        let out = f();
        let secs = start.elapsed().as_secs_f64();
        PARTS.with(|parts| {
            let mut parts = parts.borrow_mut();
            match parts.iter_mut().find(|p| p.0 == name) {
                Some(p) => {
                    p.1 += secs;
                    p.2 += 1;
                }
                None => parts.push((name, secs, 1)),
            }
        });
        out
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = name;
        f()
    }
}

/// Average milliseconds per run of every part since the last report and how many runs,
/// slowest first, and starts counting afresh.
pub fn report() -> String {
    PARTS.with(|parts| {
        let mut parts = std::mem::take(&mut *parts.borrow_mut());
        parts.sort_by(|a, b| (b.1 / b.2 as f64).total_cmp(&(a.1 / a.2 as f64)));
        parts
            .iter()
            .map(|(name, secs, n)| format!("{name} {:.3}ms x{n}", secs / *n as f64 * 1000.0))
            .collect::<Vec<_>>()
            .join(" ")
    })
}
