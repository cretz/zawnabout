//! Compiles the gameplay file (`play.kdl`) into Rust source for the play crate: lairs,
//! sets and finds. It sits beside the world and never changes it. Coordinates are
//! world x and z, and a find without y rests on whatever is under it.
//!
//! ```kdl
//! lair east-hollow name="East Hollow" x=10 y=-40 z=20 r=30
//! set lenses name="Lighthouse lenses"
//! find gull-crown kind=crown name="The Gull Crown" x=1 z=2 story="..."
//! find lens-north kind=lens set=lenses name="North lens" x=1 z=2 hint="..." story="..."
//! coins pile-1 x=1 z=2 n=20
//! postcard high-falls name="High Falls" island=home x=1 z=2 r=60 line="..."
//! local mara name="Old Mara" town="Kettle Harbor" x=1 z=2 facing=90 look=3 {
//!     line "Something they say."
//!     request kind=return target=nook-ring ask="..." hint="..." thanks="..." gold=20 \
//!         gift-kind=shell gift-name="..." gift-story="..."
//! }
//! ```
//!
//! Flying for fun: stunts (through a gate, a move, or riding a vehicle), courses of
//! gates passed in any order, and races against a vehicle. A gate is a line on the
//! ground (`x1 z1 x2 z2`) crossed between heights `lo` and `hi`, or with `r` a column
//! round `x z`. Vehicles are named as the sim's kinds, lower case (`train`, `ferry`).
//!
//! ```kdl
//! stunt under-sea-arch name="..." line="..." kind=gap x1=1 z1=2 x2=3 z2=4 lo=-10 hi=40
//! stunt falling-star name="Falling Star" line="..." kind=dive
//! stunt train-surfer name="Train Surfer" line="..." kind=ride vehicle=train
//! course kettle-bridges name="..." line="..." rumor="..." gold=15 prize-kind=ribbon \
//!         prize-name="..." prize-story="..." {
//!     gate x1=1 z1=2 x2=3 z2=4 lo=-10 hi=12
//!     gate x=1 z=2 r=40 lo=0 hi=90
//! }
//! race ferry-race name="..." vehicle=ferry from="Kettle Harbor" from-x=1 from-z=2 \
//!     to="Tidewater" to-x=3 to-z=4 r=80 rumor="..." gold=15 prize-kind=ribbon ...
//! ```
//!
//! A request is one of: `return` a find from the hoard (`target` the find), `light` a
//! beacon (`target` its world id), bring a `postcard` (`target` the postcard), or
//! `fetch` a lost thing lying at `item-x`, `item-z` (`item-y` to look down from), or
//! give a `ride` to a postcard spot (`target` the postcard).

use std::collections::HashSet;
use std::fmt::Write;
use std::path::Path;

use kdl::{KdlDocument, KdlNode};

/// A prop: its name, `s` for text or `n` for a number, and whether it is required.
type Prop = (&'static str, char, bool);

/// What each node takes.
const NODES: &[(&str, &[Prop])] = &[
    (
        "lair",
        &[
            ("name", 's', true),
            ("x", 'n', true),
            ("y", 'n', true),
            ("z", 'n', true),
            ("r", 'n', false),
        ],
    ),
    ("set", &[("name", 's', true), ("done", 's', false)]),
    (
        "happening",
        &[
            ("name", 's', true),
            ("place", 's', true),
            ("rumor", 's', true),
            ("seen", 's', true),
            ("x", 'n', true),
            ("z", 'n', true),
            ("r", 'n', false),
        ],
    ),
    (
        "favor",
        &[("town", 's', true), ("at", 'n', true), ("done", 's', true)],
    ),
    (
        "find",
        &[
            ("kind", 's', true),
            ("name", 's', true),
            ("story", 's', true),
            ("x", 'n', true),
            ("y", 'n', false),
            ("z", 'n', true),
            ("set", 's', false),
            ("hint", 's', false),
            ("gold", 'n', false),
            ("when", 's', false),
            ("sound", 's', false),
        ],
    ),
    (
        "postcard",
        &[
            ("name", 's', true),
            ("line", 's', true),
            ("island", 's', true),
            ("x", 'n', true),
            ("y", 'n', false),
            ("z", 'n', true),
            ("r", 'n', false),
            ("aim", 'n', false),
        ],
    ),
    (
        "local",
        &[
            ("name", 's', true),
            ("town", 's', true),
            ("x", 'n', true),
            ("y", 'n', false),
            ("z", 'n', true),
            ("facing", 'n', false),
            ("look", 'n', false),
        ],
    ),
    (
        "stunt",
        &[
            ("name", 's', true),
            ("line", 's', true),
            ("kind", 's', true),
            ("vehicle", 's', false),
            ("x1", 'n', false),
            ("z1", 'n', false),
            ("x2", 'n', false),
            ("z2", 'n', false),
            ("x", 'n', false),
            ("z", 'n', false),
            ("r", 'n', false),
            ("lo", 'n', false),
            ("hi", 'n', false),
        ],
    ),
    (
        "course",
        &[
            ("name", 's', true),
            ("line", 's', true),
            ("rumor", 's', true),
            ("gold", 'n', false),
            ("prize-kind", 's', true),
            ("prize-name", 's', true),
            ("prize-story", 's', true),
        ],
    ),
    (
        "story",
        &[
            ("name", 's', true),
            ("giver", 's', true),
            ("ask", 's', true),
            ("about", 's', true),
            ("done", 's', true),
            ("gold", 'n', false),
            ("prize-kind", 's', true),
            ("prize-name", 's', true),
            ("prize-story", 's', true),
        ],
    ),
    (
        "race",
        &[
            ("name", 's', true),
            ("vehicle", 's', true),
            ("from", 's', true),
            ("from-x", 'n', true),
            ("from-z", 'n', true),
            ("to", 's', true),
            ("to-x", 'n', true),
            ("to-z", 'n', true),
            ("r", 'n', false),
            ("rumor", 's', true),
            ("gold", 'n', false),
            ("prize-kind", 's', true),
            ("prize-name", 's', true),
            ("prize-story", 's', true),
        ],
    ),
    (
        "dragon",
        &[
            ("name", 's', true),
            ("kind", 's', true),
            ("island", 's', true),
            ("x", 'n', true),
            ("y", 'n', false),
            ("z", 'n', true),
            ("tint", 's', false),
            ("size", 'n', false),
            ("meet", 's', true),
            ("about", 's', true),
            ("rumor", 's', true),
            ("secret", 's', true),
            ("gift-kind", 's', true),
            ("gift-name", 's', true),
            ("gift-story", 's', true),
            ("race-to", 's', false),
            ("race-x", 'n', false),
            ("race-z", 'n', false),
            ("race-r", 'n', false),
            ("prize-kind", 's', false),
            ("prize-name", 's', false),
            ("prize-story", 's', false),
            ("want", 's', false),
            ("trade-kind", 's', false),
            ("trade-name", 's', false),
            ("trade-story", 's', false),
        ],
    ),
    (
        "coins",
        &[
            ("x", 'n', true),
            ("y", 'n', false),
            ("z", 'n', true),
            ("n", 'n', true),
        ],
    ),
];

/// What a `request` child takes.
const REQUEST: &[Prop] = &[
    ("kind", 's', true),
    ("target", 's', false),
    ("ask", 's', true),
    ("hint", 's', true),
    ("thanks", 's', true),
    ("gold", 'n', false),
    ("gift-kind", 's', false),
    ("gift-name", 's', false),
    ("gift-story", 's', false),
    ("item-kind", 's', false),
    ("item-name", 's', false),
    ("item-x", 'n', false),
    ("item-y", 'n', false),
    ("item-z", 'n', false),
];

/// Model kinds a find can be. The game has a mesh for each.
pub const KINDS: &[&str] = &[
    "crown",
    "bell",
    "trumpet",
    "goblet",
    "ring",
    "compass",
    "spyglass",
    "figurehead",
    "chest",
    "lens",
    "chess",
    "lantern",
    "anchor",
    "harp",
    "egg",
    "map",
    "teapot",
    "music-box",
    "kite",
    "basket",
    "hat",
    "shell",
    "painting",
    "ribbon",
    "trophy",
    "pot",
    "pearl",
    "vase",
    "key",
    "book",
    "medal",
    "fossil",
    "flute",
    "toy-boat",
    "gem",
    "clock",
    "candle",
];

/// Stunt kinds: through a gate, a move, or a ride.
const STUNTS: &[&str] = &[
    "gap", "near", "dive", "comet", "skim", "glide", "high", "deep", "spin", "cling", "splash",
    "fast", "ride",
];

/// Vehicles to ride or race, as the sim's kinds in lower case.
const VEHICLES: &[&str] = &[
    "train",
    "ferry",
    "balloon",
    "ferriscar",
    "liftchair",
    "liftcabin",
    "punt",
    "trawler",
    "cruise",
    "coastercar",
    "car",
    "seaplane",
    "sailboat",
    "dropring",
    "speedboat",
    "motoryacht",
    "supply",
    "airliner",
    "helicopter",
    "racecar",
];

/// A gate from a node's `x1 z1 x2 z2` or `x z r`, and `lo`, `hi`, as Rust source.
fn gate(node: &KdlNode) -> Result<String, String> {
    let lo = num(node, "lo").unwrap_or(-10000.0);
    let hi = num(node, "hi").unwrap_or(10000.0);
    if let (Some(x), Some(z), Some(r)) = (num(node, "x"), num(node, "z"), num(node, "r")) {
        return Ok(format!(
            "GateDef {{ a: [{x:.2}, {z:.2}], b: [{x:.2}, {z:.2}], r: {r:.2}, lo: {lo:.2}, hi: {hi:.2} }}"
        ));
    }
    match (
        num(node, "x1"),
        num(node, "z1"),
        num(node, "x2"),
        num(node, "z2"),
    ) {
        (Some(x1), Some(z1), Some(x2), Some(z2)) => Ok(format!(
            "GateDef {{ a: [{x1:.2}, {z1:.2}], b: [{x2:.2}, {z2:.2}], r: 0.0, lo: {lo:.2}, hi: {hi:.2} }}"
        )),
        _ => Err("a gate needs x1 z1 x2 z2, or x z r".into()),
    }
}

/// What an effect child of a set takes.
const EFFECT: &[&str] = &[
    "x", "z", "h", "r", "x1", "z1", "h1", "x2", "z2", "h2", "name", "line", "facing", "look",
];

/// A set's effect child (`folk`, `bunting`, `lanterns`) as Rust source.
fn effect(kid: &KdlNode) -> Result<String, String> {
    let kind = kid.name().value();
    for e in kid.entries().iter() {
        let key = e.name().map(|n| n.value()).unwrap_or_default();
        if !EFFECT.contains(&key) {
            return Err(format!("{kind}: unknown prop {key}"));
        }
    }
    let n = |k: &str| num(kid, k).unwrap_or(0.0);
    match kind {
        "folk" => {
            if num(kid, "x").is_none() || num(kid, "z").is_none() || text(kid, "line").is_none() {
                return Err("folk needs x, z and a line".into());
            }
            Ok(format!(
                "EffectDef {{ kind: \"folk\", a: [{:.2}, 0.0, {:.2}], b: [{:.2}, 0.0, {:.2}], name: {:?}, line: {:?}, facing: {:.2}, look: {} }}",
                n("x"),
                n("z"),
                n("x"),
                n("z"),
                text(kid, "name").unwrap_or_else(|| "Somebody".into()),
                text(kid, "line").unwrap_or_default(),
                n("facing"),
                n("look") as u32,
            ))
        }
        // A show of fireworks over `x`, `z`, bursting `h` up, at night. A weather vane
        // on a pole `h` tall: a little gold dragon turning in the wind.
        "fireworks" | "vane" => {
            if num(kid, "x").is_none() || num(kid, "z").is_none() {
                return Err(format!("{kind} needs x and z"));
            }
            let h = num(kid, "h").unwrap_or(if kind == "vane" { 8.0 } else { 90.0 });
            Ok(format!(
                "EffectDef {{ kind: {kind:?}, a: [{:.2}, {h:.2}, {:.2}], b: [{:.2}, {h:.2}, {:.2}], name: \"\", line: \"\", facing: 0.0, look: 0 }}",
                n("x"),
                n("z"),
                n("x"),
                n("z"),
            ))
        }
        // A skein of geese flying a loop `r` round `x`, `z` at `h` up from the sea, the
        // far end of the loop kept in `b`.
        "geese" => {
            if num(kid, "x").is_none() || num(kid, "z").is_none() {
                return Err("geese needs x and z".into());
            }
            let h = num(kid, "h").unwrap_or(70.0);
            Ok(format!(
                "EffectDef {{ kind: \"geese\", a: [{:.2}, {h:.2}, {:.2}], b: [{:.2}, {h:.2}, {:.2}], name: \"\", line: \"\", facing: 0.0, look: 0 }}",
                n("x"),
                n("z"),
                n("x") + num(kid, "r").unwrap_or(250.0),
                n("z"),
            ))
        }
        // A ghost ship at anchor on the sea at `x`, `z`, only out at night, swinging round
        // its anchor `r` away, kept in `b`.
        "ghostship" => {
            if num(kid, "x").is_none() || num(kid, "z").is_none() {
                return Err("ghostship needs x and z".into());
            }
            Ok(format!(
                "EffectDef {{ kind: \"ghostship\", a: [{:.2}, 0.0, {:.2}], b: [{:.2}, 0.0, {:.2}], name: \"\", line: \"\", facing: 0.0, look: 0 }}",
                n("x"),
                n("z"),
                n("x") + num(kid, "r").unwrap_or(40.0),
                n("z"),
            ))
        }
        "bunting" | "lanterns" => {
            if ["x1", "z1", "x2", "z2"]
                .iter()
                .any(|k| num(kid, k).is_none())
            {
                return Err(format!("{kind} needs x1 z1 x2 z2"));
            }
            Ok(format!(
                "EffectDef {{ kind: {kind:?}, a: [{:.2}, {:.2}, {:.2}], b: [{:.2}, {:.2}, {:.2}], name: \"\", line: \"\", facing: 0.0, look: 0 }}",
                n("x1"),
                num(kid, "h1").unwrap_or(6.0),
                n("z1"),
                n("x2"),
                num(kid, "h2").unwrap_or(6.0),
                n("z2"),
            ))
        }
        other => Err(format!("unknown node {other}")),
    }
}

/// A prize from `prize-kind`, `prize-name` and `prize-story`, as Rust source.
fn prize(node: &KdlNode) -> Result<String, String> {
    gift(node, "prize")
}

/// A gift from `{key}-kind`, `{key}-name` and `{key}-story`, as Rust source.
fn gift(node: &KdlNode, key: &str) -> Result<String, String> {
    let kind = text(node, &format!("{key}-kind")).unwrap_or_default();
    if !KINDS.contains(&kind.as_str()) {
        return Err(format!("unknown {key}-kind {kind}"));
    }
    Ok(format!(
        "GiftDef {{ kind: {kind:?}, name: {:?}, story: {:?} }}",
        text(node, &format!("{key}-name")).unwrap_or_default(),
        text(node, &format!("{key}-story")).unwrap_or_default()
    ))
}

/// Points from children named `want`, as `[x, y, z]` with y NaN when not given.
fn points(src: &str, node: &KdlNode, want: &str, errors: &mut Vec<String>) -> Vec<String> {
    node.iter_children()
        .filter(|k| k.name().value() == want)
        .filter_map(|k| match (num(k, "x"), num(k, "z")) {
            (Some(x), Some(z)) => Some(format!(
                "[{x:.2}, {}, {z:.2}]",
                num(k, "y").map_or("f32::NAN".into(), |y| format!("{y:.2}"))
            )),
            _ => {
                errors.push(format!("play.kdl:{}: {want} needs x and z", line(src, k)));
                None
            }
        })
        .collect()
}

fn line(src: &str, node: &KdlNode) -> usize {
    let at = node.span().offset().min(src.len());
    src[..at].matches('\n').count() + 1
}

fn num(node: &KdlNode, key: &str) -> Option<f64> {
    let v = node.get(key)?;
    v.as_float().or_else(|| v.as_integer().map(|i| i as f64))
}

fn text(node: &KdlNode, key: &str) -> Option<String> {
    node.get(key).and_then(|v| v.as_string()).map(String::from)
}

fn opt(v: Option<f64>) -> String {
    v.map_or("None".into(), |v| format!("Some({v:.2})"))
}

/// The gameplay file as Rust source, defining `pub static PLAY: PlayDef`.
pub fn compile(path: &Path) -> Result<String, String> {
    let src = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let doc: KdlDocument = src.parse().map_err(|e: kdl::KdlError| {
        e.diagnostics
            .iter()
            .map(|d| {
                let at = d.span.offset().min(src.len());
                let msg = d.message.clone().unwrap_or_else(|| "bad KDL".into());
                format!("play.kdl:{}: {msg}", src[..at].matches('\n').count() + 1)
            })
            .collect::<Vec<_>>()
            .join("\n")
    })?;
    let mut errors = Vec::new();
    let mut ids = HashSet::new();
    let (mut lairs, mut sets, mut finds) = (String::new(), String::new(), String::new());
    let mut cards = String::new();
    let mut locals = String::new();
    let (mut stunts, mut courses, mut races) = (String::new(), String::new(), String::new());
    let mut stories = String::new();
    let mut kin = String::new();
    let (mut happenings, mut favors) = (String::new(), String::new());
    // Secrets the dragons lead to, checked once every find is read: (line, find id).
    let mut secrets: Vec<(usize, String)> = Vec::new();
    // Story givers, checked once every local is read: (line, local id).
    let mut givers: Vec<(usize, String)> = Vec::new();
    // Request targets, checked once every node is read: (line, kind, target).
    let mut targets: Vec<(usize, String, String)> = Vec::new();
    let mut set_ids = Vec::new();
    for node in doc.nodes() {
        let name = node.name().value();
        let mut err =
            |msg: String| errors.push(format!("play.kdl:{}: {name}: {msg}", line(&src, node)));
        let Some((_, props)) = NODES.iter().find(|(n, _)| *n == name) else {
            err("unknown node".into());
            continue;
        };
        let args: Vec<_> = node
            .entries()
            .iter()
            .filter(|e| e.name().is_none())
            .collect();
        let id = match args.as_slice() {
            [a] => a.value().as_string().unwrap_or_default().to_string(),
            _ => String::new(),
        };
        if id.is_empty() {
            err("needs one id".into());
            continue;
        }
        if !ids.insert(id.clone()) {
            err(format!("{id} is used twice"));
        }
        for e in node.entries().iter().filter(|e| e.name().is_some()) {
            let key = e.name().map(|n| n.value()).unwrap_or_default();
            match props.iter().find(|(k, _, _)| *k == key) {
                None => err(format!("unknown prop {key}")),
                Some((_, 's', _)) if e.value().as_string().is_none() => {
                    err(format!("{key} wants text"))
                }
                Some((_, 'n', _)) if num(node, key).is_none() => {
                    err(format!("{key} wants a number"))
                }
                _ => {}
            }
        }
        for (key, _, required) in props.iter() {
            if *required && node.get(*key).is_none() {
                err(format!("missing {key}"));
            }
        }
        let (x, y, z) = (num(node, "x"), num(node, "y"), num(node, "z"));
        match name {
            "lair" => {
                let _ = writeln!(
                    lairs,
                    "    LairDef {{ id: {id:?}, name: {:?}, at: [{:.2}, {:.2}, {:.2}], r: {:.2} }},",
                    text(node, "name").unwrap_or_default(),
                    x.unwrap_or(0.0),
                    y.unwrap_or(0.0),
                    z.unwrap_or(0.0),
                    num(node, "r").unwrap_or(25.0),
                );
            }
            "local" => {
                let mut lines = Vec::new();
                let mut request = "None".to_string();
                for kid in node.iter_children() {
                    let at = line(&src, kid);
                    match kid.name().value() {
                        "line" => match kid.entries().first().and_then(|e| e.value().as_string()) {
                            Some(t) => lines.push(format!("{t:?}")),
                            None => errors.push(format!("play.kdl:{at}: line: wants text")),
                        },
                        "request" => {
                            for e in kid.entries().iter().filter(|e| e.name().is_some()) {
                                let key = e.name().map(|n| n.value()).unwrap_or_default();
                                if !REQUEST.iter().any(|(k, _, _)| *k == key) {
                                    errors.push(format!(
                                        "play.kdl:{at}: request: unknown prop {key}"
                                    ));
                                }
                            }
                            for (key, _, required) in REQUEST {
                                if *required && kid.get(*key).is_none() {
                                    errors.push(format!("play.kdl:{at}: request: missing {key}"));
                                }
                            }
                            let kind = text(kid, "kind").unwrap_or_default();
                            if !["return", "light", "postcard", "fetch", "ride"]
                                .contains(&kind.as_str())
                            {
                                errors.push(format!("play.kdl:{at}: request: unknown kind {kind}"));
                            }
                            let target = text(kid, "target").unwrap_or_default();
                            if kind != "fetch" {
                                targets.push((at, kind.clone(), target.clone()));
                            }
                            let gift = match text(kid, "gift-kind") {
                                Some(k) => {
                                    if !KINDS.contains(&k.as_str()) {
                                        errors.push(format!(
                                            "play.kdl:{at}: request: unknown gift-kind {k}"
                                        ));
                                    }
                                    format!(
                                        "Some(GiftDef {{ kind: {k:?}, name: {:?}, story: {:?} }})",
                                        text(kid, "gift-name").unwrap_or_default(),
                                        text(kid, "gift-story").unwrap_or_default()
                                    )
                                }
                                None => "None".into(),
                            };
                            let item = match text(kid, "item-kind") {
                                Some(k) if kind == "fetch" => {
                                    if !KINDS.contains(&k.as_str()) {
                                        errors.push(format!(
                                            "play.kdl:{at}: request: unknown item-kind {k}"
                                        ));
                                    }
                                    format!(
                                        "Some(ItemDef {{ kind: {k:?}, name: {:?}, x: {:.2}, y: {}, z: {:.2} }})",
                                        text(kid, "item-name").unwrap_or_default(),
                                        num(kid, "item-x").unwrap_or(0.0),
                                        opt(num(kid, "item-y")),
                                        num(kid, "item-z").unwrap_or(0.0)
                                    )
                                }
                                _ if kind == "fetch" => {
                                    errors.push(format!(
                                        "play.kdl:{at}: request: fetch needs item-kind"
                                    ));
                                    "None".into()
                                }
                                _ => "None".into(),
                            };
                            request = format!(
                                "Some(RequestDef {{ kind: {kind:?}, target: {target:?}, ask: {:?}, hint: {:?}, thanks: {:?}, gold: {}, gift: {gift}, item: {item} }})",
                                text(kid, "ask").unwrap_or_default(),
                                text(kid, "hint").unwrap_or_default(),
                                text(kid, "thanks").unwrap_or_default(),
                                num(kid, "gold").unwrap_or(0.0) as u32,
                            );
                        }
                        other => errors.push(format!("play.kdl:{at}: unknown node {other}")),
                    }
                }
                if lines.is_empty() {
                    errors.push(format!(
                        "play.kdl:{}: local: needs at least one line",
                        line(&src, node)
                    ));
                }
                let _ = writeln!(
                    locals,
                    "    LocalDef {{ id: {id:?}, name: {:?}, town: {:?}, x: {:.2}, y: {}, z: {:.2}, facing: {:.2}, look: {}, lines: &[{}], request: {request} }},",
                    text(node, "name").unwrap_or_default(),
                    text(node, "town").unwrap_or_default(),
                    x.unwrap_or(0.0),
                    opt(y),
                    z.unwrap_or(0.0),
                    num(node, "facing").unwrap_or(0.0),
                    num(node, "look").unwrap_or(0.0) as u32,
                    lines.join(", "),
                );
            }
            "postcard" => {
                let _ = writeln!(
                    cards,
                    "    PostcardDef {{ id: {id:?}, name: {:?}, line: {:?}, island: {:?}, x: {:.2}, y: {}, z: {:.2}, r: {:.2}, aim: {:.2} }},",
                    text(node, "name").unwrap_or_default(),
                    text(node, "line").unwrap_or_default(),
                    text(node, "island").unwrap_or_default(),
                    x.unwrap_or(0.0),
                    opt(y),
                    z.unwrap_or(0.0),
                    num(node, "r").unwrap_or(30.0),
                    num(node, "aim").unwrap_or(0.0),
                );
            }
            "stunt" => {
                let kind = text(node, "kind").unwrap_or_default();
                let vehicle = text(node, "vehicle").unwrap_or_default();
                if !STUNTS.contains(&kind.as_str()) {
                    err(format!("unknown kind {kind}"));
                }
                if (kind == "ride") == vehicle.is_empty() {
                    err("a ride stunt, and only a ride stunt, names a vehicle".into());
                } else if kind == "ride" && !VEHICLES.contains(&vehicle.as_str()) {
                    err(format!("unknown vehicle {vehicle}"));
                }
                let g = if kind == "gap" || kind == "near" {
                    match gate(node) {
                        Ok(g) => format!("Some({g})"),
                        Err(e) => {
                            err(e);
                            "None".into()
                        }
                    }
                } else {
                    "None".into()
                };
                let _ = writeln!(
                    stunts,
                    "    StuntDef {{ id: {id:?}, name: {:?}, line: {:?}, kind: {kind:?}, vehicle: {vehicle:?}, gate: {g} }},",
                    text(node, "name").unwrap_or_default(),
                    text(node, "line").unwrap_or_default(),
                );
            }
            "course" => {
                let (mut gates, mut bad) = (Vec::new(), Vec::new());
                for kid in node.iter_children() {
                    let at = line(&src, kid);
                    if kid.name().value() != "gate" {
                        bad.push(format!(
                            "play.kdl:{at}: unknown node {}",
                            kid.name().value()
                        ));
                        continue;
                    }
                    for e in kid.entries().iter() {
                        let key = e.name().map(|n| n.value()).unwrap_or_default();
                        if !["x1", "z1", "x2", "z2", "x", "z", "r", "lo", "hi"].contains(&key) {
                            bad.push(format!("play.kdl:{at}: gate: unknown prop {key}"));
                        }
                    }
                    match gate(kid) {
                        Ok(g) => gates.push(g),
                        Err(e) => bad.push(format!("play.kdl:{at}: {e}")),
                    }
                }
                if gates.len() < 2 {
                    err("a course needs at least two gates".into());
                }
                let p = prize(node).unwrap_or_else(|e| {
                    err(e);
                    String::new()
                });
                let _ = writeln!(
                    courses,
                    "    CourseDef {{ id: {id:?}, name: {:?}, line: {:?}, rumor: {:?}, gold: {}, prize: {p}, gates: &[{}] }},",
                    text(node, "name").unwrap_or_default(),
                    text(node, "line").unwrap_or_default(),
                    text(node, "rumor").unwrap_or_default(),
                    num(node, "gold").unwrap_or(0.0) as u32,
                    gates.join(", "),
                );
                errors.extend(bad);
            }
            "story" => {
                let mut regions = Vec::new();
                let mut bad = Vec::new();
                for kid in node.iter_children() {
                    let at = line(&src, kid);
                    let rid = kid
                        .entries()
                        .iter()
                        .find(|e| e.name().is_none())
                        .and_then(|e| e.value().as_string())
                        .unwrap_or_default()
                        .to_string();
                    let (Some(x), Some(z), Some(r)) = (num(kid, "x"), num(kid, "z"), num(kid, "r"))
                    else {
                        bad.push(format!("play.kdl:{at}: region needs an id, x, z and r"));
                        continue;
                    };
                    if kid.name().value() != "region" || rid.is_empty() {
                        bad.push(format!("play.kdl:{at}: a story holds regions with ids"));
                        continue;
                    }
                    let mut lamps = Vec::new();
                    for lamp in kid.iter_children() {
                        let lat = line(&src, lamp);
                        match (
                            lamp.name().value(),
                            num(lamp, "x"),
                            num(lamp, "z"),
                            text(lamp, "name"),
                        ) {
                            ("lamp", Some(lx), Some(lz), Some(name)) => lamps.push(format!(
                                "LampDef {{ name: {name:?}, x: {lx:.2}, y: {}, z: {lz:.2} }}",
                                opt(num(lamp, "y"))
                            )),
                            _ => bad.push(format!(
                                "play.kdl:{lat}: a region holds lamps with name, x and z"
                            )),
                        }
                    }
                    if lamps.is_empty() {
                        bad.push(format!("play.kdl:{at}: region {rid} has no lamps"));
                    }
                    regions.push(format!(
                        "RegionDef {{ id: {rid:?}, name: {:?}, x: {x:.2}, z: {z:.2}, r: {r:.2}, lamps: &[{}] }}",
                        text(kid, "name").unwrap_or_else(|| rid.clone()),
                        lamps.join(", ")
                    ));
                }
                if regions.is_empty() {
                    err("a story needs regions".into());
                }
                let p = prize(node).unwrap_or_else(|e| {
                    err(e);
                    String::new()
                });
                let giver = text(node, "giver").unwrap_or_default();
                givers.push((line(&src, node), giver.clone()));
                let _ = writeln!(
                    stories,
                    "    StoryDef {{ id: {id:?}, name: {:?}, giver: {giver:?}, ask: {:?}, about: {:?}, done: {:?}, gold: {}, prize: {p}, regions: &[{}] }},",
                    text(node, "name").unwrap_or_default(),
                    text(node, "ask").unwrap_or_default(),
                    text(node, "about").unwrap_or_default(),
                    text(node, "done").unwrap_or_default(),
                    num(node, "gold").unwrap_or(0.0) as u32,
                    regions.join(", "),
                );
                errors.extend(bad);
            }
            "dragon" => {
                let kind = text(node, "kind").unwrap_or_default();
                if !["wise", "racer", "shy", "collector"].contains(&kind.as_str()) {
                    err(format!("unknown kind {kind}"));
                }
                let mut bad = Vec::new();
                let perches = points(&src, node, "perch", &mut bad);
                let way = points(&src, node, "way", &mut bad);
                if way.is_empty() {
                    bad.push(format!(
                        "play.kdl:{}: a dragon needs a way",
                        line(&src, node)
                    ));
                }
                let lines: Vec<String> = node
                    .iter_children()
                    .filter(|k| k.name().value() == "line")
                    .filter_map(|k| k.entries().first()?.value().as_string())
                    .map(|t| format!("{t:?}"))
                    .collect();
                for k in node.iter_children() {
                    if !["perch", "way", "line"].contains(&k.name().value()) {
                        bad.push(format!(
                            "play.kdl:{}: a dragon holds perch, way and line",
                            line(&src, k)
                        ));
                    }
                }
                let g = gift(node, "gift").unwrap_or_else(|e| {
                    err(e);
                    String::new()
                });
                let race = match (
                    text(node, "race-to"),
                    num(node, "race-x"),
                    num(node, "race-z"),
                ) {
                    (Some(to), Some(rx), Some(rz)) => {
                        let p = prize(node).unwrap_or_else(|e| {
                            err(e);
                            String::new()
                        });
                        format!(
                            "Some(({to:?}, [{rx:.2}, {rz:.2}], {:.2}, {p}))",
                            num(node, "race-r").unwrap_or(60.0)
                        )
                    }
                    _ => "None".into(),
                };
                let trade = match text(node, "want") {
                    Some(want) => {
                        if !KINDS.contains(&want.as_str()) {
                            err(format!("unknown want {want}"));
                        }
                        let t = gift(node, "trade").unwrap_or_else(|e| {
                            err(e);
                            String::new()
                        });
                        format!("Some(({want:?}, {t}))")
                    }
                    None => "None".into(),
                };
                if kind == "racer" && race == "None" {
                    err("a racer needs race-to, race-x and race-z".into());
                }
                if kind == "collector" && trade == "None" {
                    err("a collector needs a want".into());
                }
                let secret = text(node, "secret").unwrap_or_default();
                secrets.push((line(&src, node), secret.clone()));
                let tint = text(node, "tint")
                    .map(|t| u32::from_str_radix(t.trim_start_matches('#'), 16))
                    .unwrap_or(Ok(0x808080))
                    .unwrap_or_else(|_| {
                        err("tint wants hex like 80A0FF".into());
                        0x808080
                    });
                let _ = writeln!(
                    kin,
                    "    KinDef {{ id: {id:?}, name: {:?}, kind: {kind:?}, island: {:?}, x: {:.2}, y: {}, z: {:.2}, tint: {tint:#08X}, size: {:.2}, meet: {:?}, about: {:?}, rumor: {:?}, lines: &[{}], perches: &[{}], way: &[{}], secret: {secret:?}, gift: {g}, race: {race}, trade: {trade} }},",
                    text(node, "name").unwrap_or_default(),
                    text(node, "island").unwrap_or_default(),
                    x.unwrap_or(0.0),
                    opt(y),
                    z.unwrap_or(0.0),
                    num(node, "size").unwrap_or(1.0),
                    text(node, "meet").unwrap_or_default(),
                    text(node, "about").unwrap_or_default(),
                    text(node, "rumor").unwrap_or_default(),
                    lines.join(", "),
                    perches.join(", "),
                    way.join(", "),
                );
                errors.extend(bad);
            }
            "race" => {
                let vehicle = text(node, "vehicle").unwrap_or_default();
                if !VEHICLES.contains(&vehicle.as_str()) {
                    err(format!("unknown vehicle {vehicle}"));
                }
                let p = prize(node).unwrap_or_else(|e| {
                    err(e);
                    String::new()
                });
                let _ = writeln!(
                    races,
                    "    RaceDef {{ id: {id:?}, name: {:?}, vehicle: {vehicle:?}, from: {:?}, from_at: [{:.2}, {:.2}], to: {:?}, to_at: [{:.2}, {:.2}], r: {:.2}, rumor: {:?}, gold: {}, prize: {p} }},",
                    text(node, "name").unwrap_or_default(),
                    text(node, "from").unwrap_or_default(),
                    num(node, "from-x").unwrap_or(0.0),
                    num(node, "from-z").unwrap_or(0.0),
                    text(node, "to").unwrap_or_default(),
                    num(node, "to-x").unwrap_or(0.0),
                    num(node, "to-z").unwrap_or(0.0),
                    num(node, "r").unwrap_or(80.0),
                    text(node, "rumor").unwrap_or_default(),
                    num(node, "gold").unwrap_or(0.0) as u32,
                );
            }
            "happening" | "favor" => {
                let mut effects = Vec::new();
                let mut bad = Vec::new();
                for kid in node.iter_children() {
                    match effect(kid) {
                        Ok(e) => effects.push(e),
                        Err(e) => bad.push(format!("play.kdl:{}: {e}", line(&src, kid))),
                    }
                }
                if effects.is_empty() {
                    err("needs effects to show".into());
                }
                if name == "happening" {
                    let _ = writeln!(
                        happenings,
                        "    HappeningDef {{ id: {id:?}, name: {:?}, place: {:?}, rumor: {:?}, seen: {:?}, x: {:.2}, z: {:.2}, r: {:.2}, effects: &[{}] }},",
                        text(node, "name").unwrap_or_default(),
                        text(node, "place").unwrap_or_default(),
                        text(node, "rumor").unwrap_or_default(),
                        text(node, "seen").unwrap_or_default(),
                        x.unwrap_or(0.0),
                        z.unwrap_or(0.0),
                        num(node, "r").unwrap_or(90.0),
                        effects.join(", "),
                    );
                } else {
                    let _ = writeln!(
                        favors,
                        "    FavorDef {{ id: {id:?}, town: {:?}, at: {}, done: {:?}, effects: &[{}] }},",
                        text(node, "town").unwrap_or_default(),
                        num(node, "at").unwrap_or(1.0) as usize,
                        text(node, "done").unwrap_or_default(),
                        effects.join(", "),
                    );
                }
                errors.extend(bad);
            }
            "set" => {
                set_ids.push(id.clone());
                let mut effects = Vec::new();
                let mut bad = Vec::new();
                for kid in node.iter_children() {
                    match effect(kid) {
                        Ok(e) => effects.push(e),
                        Err(e) => bad.push(format!("play.kdl:{}: {e}", line(&src, kid))),
                    }
                }
                if !effects.is_empty() && node.get("done").is_none() {
                    err("a set with effects needs a done line".into());
                }
                let _ = writeln!(
                    sets,
                    "    SetDef {{ id: {id:?}, name: {:?}, done: {:?}, effects: &[{}] }},",
                    text(node, "name").unwrap_or_default(),
                    text(node, "done").unwrap_or_default(),
                    effects.join(", "),
                );
                errors.extend(bad);
            }
            "find" | "coins" => {
                let coins = name == "coins";
                let kind = if coins {
                    "coins".to_string()
                } else {
                    text(node, "kind").unwrap_or_default()
                };
                if !coins && !KINDS.contains(&kind.as_str()) {
                    err(format!("unknown kind {kind}"));
                }
                let set = text(node, "set");
                if let Some(s) = &set
                    && !set_ids.contains(s)
                {
                    err(format!("set {s} is not defined above"));
                }
                if set.is_some() && node.get("hint").is_none() {
                    err("a set piece needs a hint".into());
                }
                let when = text(node, "when");
                if when.as_deref().is_some_and(|w| w != "night") {
                    err("when can only be night".into());
                }
                let sound = text(node, "sound");
                if sound.as_deref().is_some_and(|s| s != "chimes") {
                    err("sound can only be chimes".into());
                }
                let gold = if coins {
                    num(node, "n")
                } else {
                    num(node, "gold")
                }
                .unwrap_or(5.0);
                let _ = writeln!(
                    finds,
                    "    FindDef {{ id: {id:?}, kind: {kind:?}, name: {:?}, story: {:?}, x: {:.2}, y: {}, z: {:.2}, set: {:?}, hint: {:?}, gold: {}, night: {}, chimes: {} }},",
                    text(node, "name").unwrap_or_default(),
                    text(node, "story").unwrap_or_default(),
                    x.unwrap_or(0.0),
                    opt(y),
                    z.unwrap_or(0.0),
                    set,
                    text(node, "hint").unwrap_or_default(),
                    gold as u32,
                    when.is_some(),
                    sound.is_some(),
                );
            }
            _ => {}
        }
    }
    for (at, secret) in secrets {
        if !finds.contains(&format!("FindDef {{ id: {secret:?}")) {
            errors.push(format!("play.kdl:{at}: dragon: no find {secret}"));
        }
    }
    for (at, giver) in givers {
        if !locals.contains(&format!("LocalDef {{ id: {giver:?}")) {
            errors.push(format!("play.kdl:{at}: story: no local {giver}"));
        }
    }
    // A return wants a find, a postcard request a postcard. Beacons are the world's, and
    // the play crate's tests check those.
    for (at, kind, target) in targets {
        let want = match kind.as_str() {
            "return" => "FindDef { id: ",
            "postcard" | "ride" => "PostcardDef { id: ",
            _ => continue,
        };
        if !finds.contains(&format!("{want}{target:?}"))
            && !cards.contains(&format!("{want}{target:?}"))
        {
            errors.push(format!("play.kdl:{at}: request: no {kind} target {target}"));
        }
    }
    if !errors.is_empty() {
        return Err(errors.join("\n"));
    }
    Ok(format!(
        "pub static PLAY: PlayDef = PlayDef {{\n  lairs: &[\n{lairs}  ],\n  sets: &[\n{sets}  ],\n  finds: &[\n{finds}  ],\n  postcards: &[\n{cards}  ],\n  locals: &[\n{locals}  ],\n  stunts: &[\n{stunts}  ],\n  courses: &[\n{courses}  ],\n  races: &[\n{races}  ],\n  stories: &[\n{stories}  ],\n  kin: &[\n{kin}  ],\n  happenings: &[\n{happenings}  ],\n  favors: &[\n{favors}  ],\n}};\n"
    ))
}
