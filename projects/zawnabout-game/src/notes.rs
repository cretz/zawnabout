//! Design notes as files: what one holds, reading and writing it. The host keeps them, one
//! file each in `working-area/feedback/<id>.json`, and the HUD lists them in notes mode.

use glam::Vec3;

use crate::feedback::json_str;

/// A design note, laid out as the dev server writes it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Note {
    pub id: String,
    pub created: String,
    pub done: bool,
    pub text: String,
    pub spot: Vec3,
    /// Camera x, y, z, yaw and pitch when it was left.
    pub view: [f32; 5],
    /// Named places at and around the spot.
    pub near: Vec<String>,
    /// Who answered, and what.
    pub replies: Vec<(String, String)>,
}

impl Note {
    /// Reads a note file, `None` if it is not one.
    pub fn from_json(s: &str) -> Option<Note> {
        let v = Json::parse(s)?;
        let nums = |k: &str| -> Vec<f32> {
            v.get(k)
                .and_then(Json::arr)
                .map(|a| a.iter().filter_map(Json::num).map(|n| n as f32).collect())
                .unwrap_or_default()
        };
        let (spot, view) = (nums("spot"), nums("view"));
        let text = |j: &Json, k: &str| j.get(k).and_then(Json::str).unwrap_or("").to_string();
        let strings = |k: &str| -> Vec<&Json> {
            v.get(k)
                .and_then(Json::arr)
                .map(|a| a.iter().collect())
                .unwrap_or_default()
        };
        Some(Note {
            id: text(&v, "id"),
            created: text(&v, "created"),
            done: v.get("status").and_then(Json::str) == Some("done"),
            text: v.get("text")?.str()?.to_string(),
            spot: Vec3::new(*spot.first()?, *spot.get(1)?, *spot.get(2)?),
            view: std::array::from_fn(|i| view.get(i).copied().unwrap_or(0.0)),
            near: strings("near")
                .into_iter()
                .filter_map(Json::str)
                .map(String::from)
                .collect(),
            replies: strings("replies")
                .into_iter()
                .map(|r| (text(r, "from"), text(r, "text")))
                .collect(),
        })
    }

    /// The note as a file.
    pub fn to_json(&self) -> String {
        let list = |items: Vec<String>| {
            if items.is_empty() {
                "[]".to_string()
            } else {
                format!("[\n    {}\n  ]", items.join(",\n    "))
            }
        };
        let replies = self
            .replies
            .iter()
            .map(|(from, text)| {
                format!(
                    "{{ \"from\": {}, \"text\": {} }}",
                    json_str(from),
                    json_str(text)
                )
            })
            .collect();
        let view = self
            .view
            .iter()
            .enumerate()
            .map(|(i, n)| {
                if i < 3 {
                    format!("{n:.1}")
                } else {
                    format!("{n:.4}")
                }
            })
            .collect();
        let fields = [
            (
                "spot",
                list(
                    self.spot
                        .to_array()
                        .iter()
                        .map(|n| format!("{n:.1}"))
                        .collect(),
                ),
            ),
            ("view", list(view)),
            (
                "near",
                list(self.near.iter().map(|s| json_str(s)).collect()),
            ),
            ("text", json_str(&self.text)),
            ("id", json_str(&self.id)),
            ("created", json_str(&self.created)),
            ("status", json_str(if self.done { "done" } else { "open" })),
            ("replies", list(replies)),
        ];
        let body: Vec<String> = fields
            .iter()
            .map(|(k, v)| format!("  \"{k}\": {v}"))
            .collect();
        format!("{{\n{}\n}}\n", body.join(",\n"))
    }
}

/// A new note's id and creation time from seconds since 1970, as the dev server makes
/// them: `20261105-143000-k3x9` and `2026-11-05T14:30:00.000Z`, in UTC. `salt` varies
/// the last four letters.
pub fn note_stamp(unix: u64, salt: u64) -> (String, String) {
    let days = (unix / 86400) as i64;
    let secs = unix % 86400;
    // Days to a calendar date, after Howard Hinnant's civil_from_days.
    let z = days + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + (month <= 2) as i64;
    let (h, m, s) = (secs / 3600, secs / 60 % 60, secs % 60);
    let mut k = salt.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ unix;
    let tag: String = (0..4)
        .map(|_| {
            let c = b"0123456789abcdefghijklmnopqrstuvwxyz"[(k % 36) as usize] as char;
            k /= 36;
            c
        })
        .collect();
    (
        format!("{year:04}{month:02}{day:02}-{h:02}{m:02}{s:02}-{tag}"),
        format!("{year:04}-{month:02}-{day:02}T{h:02}:{m:02}:{s:02}.000Z"),
    )
}

/// Just enough JSON to read note files.
#[derive(Clone, Debug, PartialEq)]
enum Json {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<Json>),
    Obj(Vec<(String, Json)>),
}

type Chars<'a> = std::iter::Peekable<std::str::CharIndices<'a>>;

impl Json {
    fn parse(s: &str) -> Option<Json> {
        let mut p = s.char_indices().peekable();
        let v = Self::value(s, &mut p)?;
        Self::skip(&mut p);
        p.peek().is_none().then_some(v)
    }

    fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Obj(kv) => kv.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    fn str(&self) -> Option<&str> {
        match self {
            Json::Str(s) => Some(s),
            _ => None,
        }
    }

    fn num(&self) -> Option<f64> {
        match self {
            Json::Num(n) => Some(*n),
            _ => None,
        }
    }

    fn arr(&self) -> Option<&[Json]> {
        match self {
            Json::Arr(a) => Some(a),
            _ => None,
        }
    }

    fn skip(p: &mut Chars) {
        while p.next_if(|(_, c)| c.is_whitespace()).is_some() {}
    }

    fn value(s: &str, p: &mut Chars) -> Option<Json> {
        Self::skip(p);
        let &(at, c) = p.peek()?;
        match c {
            '{' => {
                p.next();
                let mut kv = Vec::new();
                loop {
                    Self::skip(p);
                    if p.next_if(|(_, c)| *c == '}').is_some() {
                        return Some(Json::Obj(kv));
                    }
                    let Json::Str(k) = Self::value(s, p)? else {
                        return None;
                    };
                    Self::skip(p);
                    p.next_if(|(_, c)| *c == ':')?;
                    kv.push((k, Self::value(s, p)?));
                    Self::skip(p);
                    p.next_if(|(_, c)| *c == ',');
                }
            }
            '[' => {
                p.next();
                let mut items = Vec::new();
                loop {
                    Self::skip(p);
                    if p.next_if(|(_, c)| *c == ']').is_some() {
                        return Some(Json::Arr(items));
                    }
                    items.push(Self::value(s, p)?);
                    Self::skip(p);
                    p.next_if(|(_, c)| *c == ',');
                }
            }
            '"' => {
                p.next();
                let mut out = String::new();
                loop {
                    match p.next()?.1 {
                        '"' => return Some(Json::Str(out)),
                        '\\' => match p.next()?.1 {
                            'n' => out.push('\n'),
                            't' => out.push('\t'),
                            'r' => out.push('\r'),
                            'b' | 'f' => {}
                            'u' => {
                                let hex: String =
                                    (0..4).filter_map(|_| p.next()).map(|(_, c)| c).collect();
                                let code = u32::from_str_radix(&hex, 16).ok()?;
                                out.push(char::from_u32(code).unwrap_or('?'));
                            }
                            c => out.push(c),
                        },
                        c => out.push(c),
                    }
                }
            }
            't' | 'f' | 'n' => {
                let mut word = String::new();
                while let Some((_, c)) = p.next_if(|(_, c)| c.is_ascii_alphabetic()) {
                    word.push(c);
                }
                match word.as_str() {
                    "true" => Some(Json::Bool(true)),
                    "false" => Some(Json::Bool(false)),
                    "null" => Some(Json::Null),
                    _ => None,
                }
            }
            _ => {
                let mut end = at;
                while let Some((i, c)) = p.next_if(|(_, c)| "+-.eE0123456789".contains(*c)) {
                    end = i + c.len_utf8();
                }
                s[at..end].parse().ok().map(Json::Num)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A note written by the dev server reads back, and one written here reads the same.
    #[test]
    fn notes_round_trip() {
        let file = r#"{
  "spot": [1.5, -2, 3e1],
  "view": [1, 2, 3, 0.5, -0.25],
  "near": ["ground 12m", "in \"home\" meadows"],
  "text": "Line one\nline two é",
  "id": "20261105-143000-abcd",
  "created": "2026-11-05T14:30:00.000Z",
  "status": "done",
  "replies": [{ "from": "agent", "text": "fixed", "at": null }]
}"#;
        let n = Note::from_json(file).expect("parses");
        assert_eq!(n.spot, Vec3::new(1.5, -2.0, 30.0));
        assert_eq!(n.view[4], -0.25);
        assert_eq!(n.near[1], "in \"home\" meadows");
        assert_eq!(n.text, "Line one\nline two \u{e9}");
        assert!(n.done);
        assert_eq!(n.replies, vec![("agent".into(), "fixed".into())]);
        assert_eq!(Note::from_json(&n.to_json()), Some(n));
        assert_eq!(Note::from_json("{\"text\": 1}"), None);
        let (id, created) = note_stamp(1_793_975_400, 7);
        assert!(id.starts_with("20261106-143000-") && id.len() == 20);
        assert_eq!(created, "2026-11-06T14:30:00.000Z");
    }
}
