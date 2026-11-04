//! Debug scripts and playbooks: one command per line, `#` comments, and `wait <seconds>`
//! to let time pass. `wait-until <condition> <timeout>` waits for a condition and
//! `expect <condition>` checks one, both counting failures (see [`Game::check`]). Game
//! commands run directly. Host commands (`shot`, `size`, `save-trace`, `quit`) are handed
//! back to whoever runs the script.

use crate::game::Game;

pub enum Output {
    Log(String),
    Host(Vec<String>),
}

pub struct Script {
    lines: Vec<String>,
    pc: usize,
    wait: f32,
    until: Option<(String, f32)>,
    /// Failed `expect` and timed out `wait-until` lines.
    pub failures: u32,
}

const HOST: [&str; 5] = ["shot", "size", "save-trace", "stats", "quit"];

impl Script {
    pub fn new(text: &str) -> Self {
        Self {
            lines: text
                .lines()
                .map(|l| l.split('#').next().unwrap_or("").trim().to_string())
                .filter(|l| !l.is_empty())
                .collect(),
            pc: 0,
            wait: 0.0,
            until: None,
            failures: 0,
        }
    }

    pub fn done(&self) -> bool {
        self.pc >= self.lines.len() && self.wait <= 0.0 && self.until.is_none()
    }

    /// Runs every line that is due after `dt` more seconds.
    pub fn update(&mut self, game: &mut Game, dt: f32) -> Vec<Output> {
        let mut out = Vec::new();
        // A wait only counts down once any wait-until before it is over.
        if self.until.is_none() {
            self.wait -= dt;
        }
        if let Some((cond, left)) = &mut self.until {
            let words: Vec<&str> = cond.split_whitespace().collect();
            match game.check(&words) {
                Ok(true) => {
                    self.until = None;
                    self.wait = 0.0;
                }
                Ok(false) if *left > dt => *left -= dt,
                Ok(false) => {
                    out.push(Output::Log(format!("FAIL timed out waiting for {cond}")));
                    self.failures += 1;
                    self.until = None;
                }
                Err(e) => {
                    out.push(Output::Log(format!("FAIL {e}")));
                    self.failures += 1;
                    self.until = None;
                }
            }
        }
        while self.wait <= 0.0 && self.until.is_none() && self.pc < self.lines.len() {
            let line = self.lines[self.pc].clone();
            self.pc += 1;
            let words: Vec<String> = line.split_whitespace().map(String::from).collect();
            if words[0] == "wait" {
                match words.get(1).and_then(|w| w.parse::<f32>().ok()) {
                    Some(secs) => self.wait += secs,
                    None => out.push(Output::Log(format!("bad wait: {line}"))),
                }
            } else if words[0] == "wait-until" && words.len() >= 3 {
                let timeout = words[words.len() - 1].parse::<f32>().unwrap_or(30.0);
                self.until = Some((words[1..words.len() - 1].join(" "), timeout));
            } else if words[0] == "expect" {
                let cond: Vec<&str> = words[1..].iter().map(String::as_str).collect();
                match game.check(&cond) {
                    Ok(true) => out.push(Output::Log(format!("ok {}", cond.join(" ")))),
                    Ok(false) => {
                        out.push(Output::Log(format!(
                            "FAIL expected {} ({})",
                            cond.join(" "),
                            game.state()
                        )));
                        self.failures += 1;
                    }
                    Err(e) => {
                        out.push(Output::Log(format!("FAIL {e}")));
                        self.failures += 1;
                    }
                }
            } else if HOST.contains(&words[0].as_str()) {
                // Stop here so the host runs it before any later line changes the game.
                out.push(Output::Host(words));
                break;
            } else {
                match game.command(&line) {
                    Ok(msg) if msg.is_empty() => {}
                    Ok(msg) => out.push(Output::Log(msg)),
                    Err(e) => out.push(Output::Log(format!("error: {e}"))),
                }
            }
        }
        out
    }
}
