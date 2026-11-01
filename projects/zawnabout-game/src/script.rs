//! Debug scripts: one command per line, `#` comments, and `wait <seconds>` to let time
//! pass. Game commands run directly. Host commands (`shot`, `size`, `save-trace`, `quit`)
//! are handed back to whoever runs the script.

use crate::game::Game;

pub enum Output {
    Log(String),
    Host(Vec<String>),
}

pub struct Script {
    lines: Vec<String>,
    pc: usize,
    wait: f32,
}

const HOST: [&str; 4] = ["shot", "size", "save-trace", "quit"];

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
        }
    }

    pub fn done(&self) -> bool {
        self.pc >= self.lines.len() && self.wait <= 0.0
    }

    /// Runs every line that is due after `dt` more seconds.
    pub fn update(&mut self, game: &mut Game, dt: f32) -> Vec<Output> {
        let mut out = Vec::new();
        self.wait -= dt;
        while self.wait <= 0.0 && self.pc < self.lines.len() {
            let line = self.lines[self.pc].clone();
            self.pc += 1;
            let words: Vec<String> = line.split_whitespace().map(String::from).collect();
            if words[0] == "wait" {
                match words.get(1).and_then(|w| w.parse::<f32>().ok()) {
                    Some(secs) => self.wait += secs,
                    None => out.push(Output::Log(format!("bad wait: {line}"))),
                }
            } else if HOST.contains(&words[0].as_str()) {
                out.push(Output::Host(words));
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
