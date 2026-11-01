//! Compiles a world folder of KDL files into Rust source for the sim. Runs at build time.
//!
//! Every node is checked against the vocabulary in `world/README.md`, so typos fail the
//! build with a file and line. Outlines and paths are smoothed and roughened here, once,
//! with plain arithmetic only, so every build of the same files gives the same world.

use std::collections::HashSet;
use std::fmt::Write;
use std::path::Path;

use kdl::{KdlDocument, KdlNode, KdlValue};

mod schema;
mod shape;

use schema::{Pts, spec};
use shape::{jag, sincos_deg, smooth};

/// Spacing of smoothed outline points (m).
const STEP: f64 = 20.0;

/// The compiled world as Rust source, defining `pub static WORLD: WorldDef`.
pub fn compile(dir: &Path) -> Result<String, String> {
    let mut c = Compiler::default();
    c.world(dir);
    if c.errors.is_empty() {
        Ok(c.emit())
    } else {
        Err(c.errors.join("\n"))
    }
}

#[derive(Default)]
struct Land {
    id: String,
    base: f64,
    pts: Vec<[f64; 2]>,
    edges: Vec<u8>,
}

struct Ridge {
    id: String,
    width: f64,
    pts: Vec<[f64; 3]>,
}

struct Mount {
    id: String,
    x: f64,
    z: f64,
    h: f64,
    r: f64,
    shape: &'static str,
    rim: f64,
    inner: f64,
    floor: f64,
    stretch: f64,
    rot: f64,
}

struct Plateau {
    id: String,
    top: f64,
    cliff: bool,
    pts: Vec<[f64; 2]>,
}

struct Spire {
    id: String,
    x: f64,
    z: f64,
    h: f64,
    r: f64,
    flat: bool,
}

struct Carve {
    id: String,
    kind: &'static str,
    w: f64,
    level: f64,
    pts: Vec<[f64; 3]>,
}

struct Lake {
    id: String,
    level: f64,
    pts: Vec<[f64; 2]>,
}

struct Cover {
    id: String,
    kind: &'static str,
    density: &'static str,
    trees: &'static str,
    pts: Vec<[f64; 2]>,
}

/// An empty space in the rock: a cavern, dugout, or one capsule of a tube.
struct Void {
    id: String,
    shape: &'static str,
    center: [f64; 3],
    half: [f64; 3],
    rot: f64,
    /// Capsule ends and radius, for tubes.
    a: [f64; 3],
    b: [f64; 3],
    r: f64,
    glow: f64,
}

/// Rock left standing inside a cavern.
struct Pillar {
    id: String,
    x: f64,
    z: f64,
    r: f64,
    y0: f64,
    y1: f64,
}

struct Mark {
    id: String,
    kind: String,
    pos: [f64; 3],
    ground: bool,
}

/// Where a node sits: its file, island offset and id scope.
#[derive(Clone)]
struct Ctx<'a> {
    file: &'a str,
    src: &'a str,
    scope: String,
    off: [f64; 2],
    /// Inside a cave system, so not part of the surface yet.
    under: bool,
    /// Floor and roof of the cavern this node sits in, for pillars.
    cavern: Option<(f64, f64)>,
}

#[derive(Default)]
struct Compiler {
    errors: Vec<String>,
    ids: HashSet<String>,
    name: String,
    bounds: [f64; 4],
    edge: f64,
    sky: f64,
    deep: f64,
    sea_level: f64,
    sea_floor: f64,
    spawn: [f64; 2],
    spawn_island: String,
    islands: Vec<(String, [f64; 2])>,
    lands: Vec<Land>,
    ridges: Vec<Ridge>,
    mounts: Vec<Mount>,
    plateaus: Vec<Plateau>,
    spires: Vec<Spire>,
    carves: Vec<Carve>,
    lakes: Vec<Lake>,
    covers: Vec<Cover>,
    marks: Vec<Mark>,
    voids: Vec<Void>,
    pillars: Vec<Pillar>,
}

fn line_of(src: &str, node: &KdlNode) -> usize {
    let at = node.span().offset().min(src.len());
    src[..at].matches('\n').count() + 1
}

fn num_of(v: &KdlValue) -> Option<f64> {
    v.as_float().or_else(|| v.as_integer().map(|i| i as f64))
}

impl Compiler {
    fn err(&mut self, ctx: &Ctx, node: &KdlNode, msg: impl AsRef<str>) {
        self.errors.push(format!(
            "{}:{}: {}: {}",
            ctx.file,
            line_of(ctx.src, node),
            node.name().value(),
            msg.as_ref()
        ));
    }

    fn parse(&mut self, dir: &Path, file: &str) -> Option<(String, KdlDocument)> {
        let path = dir.join(file);
        let src = match std::fs::read_to_string(&path) {
            Ok(s) => s,
            Err(e) => {
                self.errors.push(format!("{}: {e}", path.display()));
                return None;
            }
        };
        match src.parse::<KdlDocument>() {
            Ok(doc) => Some((src, doc)),
            Err(e) => {
                for d in &e.diagnostics {
                    let at = d.span.offset().min(src.len());
                    let line = src[..at].matches('\n').count() + 1;
                    let msg = d.message.clone().unwrap_or_else(|| "bad KDL".into());
                    self.errors.push(format!("{file}:{line}: {msg}"));
                }
                None
            }
        }
    }

    fn world(&mut self, dir: &Path) {
        let Some((src, doc)) = self.parse(dir, "world.kdl") else {
            return;
        };
        let ctx = Ctx {
            file: "world.kdl",
            src: &src,
            scope: String::new(),
            off: [0.0, 0.0],
            under: false,
            cavern: None,
        };
        let mut found = false;
        for node in doc.nodes() {
            match node.name().value() {
                "world" => {
                    found = true;
                    self.world_node(&ctx, node);
                }
                "island" => {
                    if !self.check(&ctx, node, "island") {
                        continue;
                    }
                    let id = self.id(&ctx, node);
                    let file = self.str(node, "file").unwrap_or_default();
                    let off = [self.num(node, "x"), self.num(node, "z")];
                    self.islands.push((id.clone(), off));
                    if let Some((isrc, idoc)) = self.parse(dir, &file) {
                        let ictx = Ctx {
                            file: &file,
                            src: &isrc,
                            scope: format!("{id}/"),
                            off,
                            under: false,
                            cavern: None,
                        };
                        self.nodes(&ictx, idoc.nodes());
                    }
                }
                _ => self.node(&ctx, node),
            }
        }
        if !found {
            self.errors.push("world.kdl: missing world node".into());
        }
        match self.islands.iter().find(|(id, _)| *id == self.spawn_island) {
            Some((_, off)) => {
                self.spawn = [self.spawn[0] + off[0], self.spawn[1] + off[1]];
                self.marks.push(Mark {
                    id: format!("{}/spawn", self.spawn_island),
                    kind: "spawn".into(),
                    pos: [self.spawn[0], 0.0, self.spawn[1]],
                    ground: true,
                });
            }
            None => self.errors.push(format!(
                "world.kdl: spawn island {} not found",
                self.spawn_island
            )),
        }
    }

    fn world_node(&mut self, ctx: &Ctx, node: &KdlNode) {
        if !self.check(ctx, node, "world") {
            return;
        }
        self.name = self.arg_str(node).unwrap_or_default();
        for kid in node.iter_children() {
            if kid.name().value() == "note" || !self.check(ctx, kid, kid.name().value()) {
                continue;
            }
            let nums: Vec<f64> = kid
                .entries()
                .iter()
                .filter(|e| e.name().is_none())
                .filter_map(|e| num_of(e.value()))
                .collect();
            match kid.name().value() {
                "bounds" => self.bounds = [nums[0], nums[1], nums[2], nums[3]],
                "edge" => self.edge = nums[0],
                "sky" => self.sky = nums[0],
                "deep" => self.deep = nums[0],
                "sea" => {
                    self.sea_level = self.num(kid, "level");
                    self.sea_floor = self.num(kid, "floor");
                }
                "spawn" => {
                    // Island relative, resolved once all islands are placed.
                    self.spawn_island = self.str(kid, "island").unwrap_or_default();
                    self.spawn = [self.num(kid, "x"), self.num(kid, "z")];
                }
                _ => {}
            }
        }
    }

    fn nodes(&mut self, ctx: &Ctx, nodes: &[KdlNode]) {
        for node in nodes {
            self.node(ctx, node);
        }
    }

    /// Checks a node against the vocabulary. Reports and returns false when it is wrong.
    fn check(&mut self, ctx: &Ctx, node: &KdlNode, name: &str) -> bool {
        let Some(s) = spec(name) else {
            self.err(ctx, node, "unknown node");
            return false;
        };
        let mut ok = true;
        let args: Vec<&KdlValue> = node
            .entries()
            .iter()
            .filter(|e| e.name().is_none())
            .map(|e| e.value())
            .collect();
        let want = s.nums + usize::from(s.id);
        if args.len() != want && !(s.id_optional && args.len() == s.nums) {
            self.err(ctx, node, format!("expected {want} argument(s)"));
            ok = false;
        }
        for e in node.entries().iter().filter(|e| e.name().is_some()) {
            let key = e.name().map(|n| n.value()).unwrap_or_default();
            match s.props.iter().find(|p| p.0 == key) {
                None => {
                    self.err(ctx, node, format!("unknown property {key}"));
                    ok = false;
                }
                Some((_, kind, _)) => {
                    let v = e.value();
                    let good = match kind {
                        'n' => num_of(v).is_some(),
                        's' => v.as_string().is_some(),
                        'b' => v.as_bool().is_some(),
                        _ => true,
                    };
                    if !good {
                        self.err(ctx, node, format!("{key} has the wrong type"));
                        ok = false;
                    }
                }
            }
        }
        for (key, _, required) in s.props {
            if *required && node.get(*key).is_none() {
                self.err(ctx, node, format!("missing {key}"));
                ok = false;
            }
        }
        for kid in node.iter_children() {
            let k = kid.name().value();
            if k == "note" {
                continue;
            }
            if k == "p" {
                if matches!(s.pts, Pts::None) {
                    self.err(ctx, kid, format!("{name} takes no points"));
                    ok = false;
                }
                continue;
            }
            if !s.kids.contains(&k) {
                self.err(ctx, kid, format!("not allowed inside {name}"));
                ok = false;
            }
        }
        ok
    }

    /// Registers a node's id in its scope and returns the full id.
    fn id(&mut self, ctx: &Ctx, node: &KdlNode) -> String {
        let local = self.arg_str(node).unwrap_or_default();
        let full = format!("{}{local}", ctx.scope);
        if !self.ids.insert(full.clone()) {
            self.err(ctx, node, format!("duplicate id {full}"));
        }
        full
    }

    fn arg_str(&self, node: &KdlNode) -> Option<String> {
        node.entries()
            .iter()
            .find(|e| e.name().is_none())
            .and_then(|e| e.value().as_string())
            .map(String::from)
    }

    fn num(&self, node: &KdlNode, key: &str) -> f64 {
        node.get(key).and_then(num_of).unwrap_or(0.0)
    }

    fn opt(&self, node: &KdlNode, key: &str) -> Option<f64> {
        node.get(key).and_then(num_of)
    }

    fn str(&self, node: &KdlNode, key: &str) -> Option<String> {
        node.get(key).and_then(|v| v.as_string()).map(String::from)
    }

    fn flag(&self, node: &KdlNode, key: &str) -> bool {
        node.get(key).and_then(|v| v.as_bool()).unwrap_or(false)
    }

    /// One of a fixed set of words, as a static string.
    fn word(
        &mut self,
        ctx: &Ctx,
        node: &KdlNode,
        key: &str,
        allowed: &[&'static str],
        default: &'static str,
    ) -> &'static str {
        match self.str(node, key) {
            None => default,
            Some(v) => match allowed.iter().find(|a| **a == v) {
                Some(a) => a,
                None => {
                    self.err(
                        ctx,
                        node,
                        format!("{key} must be one of {}", allowed.join(", ")),
                    );
                    default
                }
            },
        }
    }

    /// The node's points, offset into world space, with their point properties.
    fn points(&mut self, ctx: &Ctx, node: &KdlNode, dims: Pts) -> Vec<Point> {
        let mut out = Vec::new();
        for kid in node.iter_children().filter(|k| k.name().value() == "p") {
            let nums: Vec<f64> = kid
                .entries()
                .iter()
                .filter(|e| e.name().is_none())
                .filter_map(|e| num_of(e.value()))
                .collect();
            let ok = match dims {
                Pts::D2(_) => nums.len() == 2,
                Pts::D3(_) => nums.len() == 3,
                Pts::Any(_) => nums.len() == 2 || nums.len() == 3,
                Pts::None => false,
            };
            if !ok {
                self.err(ctx, kid, "wrong number of coordinates");
                continue;
            }
            let allowed = dims.props();
            for e in kid.entries().iter().filter(|e| e.name().is_some()) {
                let key = e.name().map(|n| n.value()).unwrap_or_default();
                if !allowed.contains(&key) {
                    self.err(ctx, kid, format!("point does not take {key}"));
                }
            }
            let (x, y, z) = if nums.len() == 3 {
                (nums[0], Some(nums[1]), nums[2])
            } else {
                (nums[0], None, nums[1])
            };
            out.push(Point {
                x: x + ctx.off[0],
                y,
                z: z + ctx.off[1],
                sharp: self.flag(kid, "sharp"),
                edge: self.str(kid, "edge"),
                h: self.opt(kid, "h"),
                depth: self.opt(kid, "depth"),
            });
        }
        out
    }

    fn outline(&mut self, ctx: &Ctx, node: &KdlNode, jag_amp: f64, seed: u64) -> Vec<[f64; 2]> {
        let pts = self.points(ctx, node, Pts::D2(&["sharp", "edge"]));
        if pts.len() < 3 {
            self.err(ctx, node, "outline needs at least 3 points");
            return Vec::new();
        }
        let src: Vec<([f64; 2], bool)> = pts.iter().map(|p| ([p.x, p.z], p.sharp)).collect();
        let (dense, _) = smooth(&src, true, STEP);
        jag(&dense, true, jag_amp, seed)
    }

    fn mark(&mut self, ctx: &Ctx, node: &KdlNode, id: String, kind: String) {
        let ground = node.get("y").is_none();
        self.marks.push(Mark {
            id,
            kind,
            pos: [
                self.num(node, "x") + ctx.off[0],
                self.num(node, "y"),
                self.num(node, "z") + ctx.off[1],
            ],
            ground,
        });
    }

    fn node(&mut self, ctx: &Ctx, node: &KdlNode) {
        let name = node.name().value();
        if name == "note" || name == "p" {
            return;
        }
        if !self.check(ctx, node, name) {
            return;
        }
        let s = spec(name).expect("checked");
        let id = if s.id && !(s.id_optional && self.arg_str(node).is_none()) {
            self.id(ctx, node)
        } else if name == "land" {
            // An island in one piece is named for the island.
            ctx.scope.trim_end_matches('/').to_string()
        } else {
            format!("{}{name}", ctx.scope)
        };
        let seed = shape::hash_str(&id);
        match name {
            "land" => {
                let jag_amp = self.opt(node, "jag").unwrap_or(20.0);
                let pts = self.points(ctx, node, Pts::D2(&["sharp", "edge"]));
                if pts.len() < 3 {
                    self.err(ctx, node, "outline needs at least 3 points");
                    return;
                }
                let mut kind = 0u8;
                let mut kinds = Vec::new();
                for p in &pts {
                    if let Some(e) = &p.edge {
                        kind = match e.as_str() {
                            "beach" => 0,
                            "rocky" => 1,
                            "cliff" => 2,
                            _ => {
                                self.err(ctx, node, "edge must be beach, rocky or cliff");
                                0
                            }
                        };
                    }
                    kinds.push(kind);
                }
                // The first points take the last edge set when they set none themselves.
                if pts[0].edge.is_none() {
                    let last = kinds[kinds.len() - 1];
                    for (k, p) in kinds.iter_mut().zip(&pts) {
                        if p.edge.is_some() {
                            break;
                        }
                        *k = last;
                    }
                }
                let src: Vec<([f64; 2], bool)> =
                    pts.iter().map(|p| ([p.x, p.z], p.sharp)).collect();
                let (dense, from) = smooth(&src, true, STEP);
                let dense = jag(&dense, true, jag_amp, seed);
                self.lands.push(Land {
                    id,
                    base: self.num(node, "base"),
                    edges: from.iter().map(|i| kinds[*i]).collect(),
                    pts: dense,
                });
            }
            "ridge" => {
                let pts = self.points(ctx, node, Pts::D2(&["h", "sharp"]));
                if pts.iter().any(|p| p.h.is_none()) {
                    self.err(ctx, node, "every ridge point needs h");
                    return;
                }
                let src: Vec<([f64; 3], bool)> = pts
                    .iter()
                    .map(|p| ([p.x, p.z, p.h.unwrap_or(0.0)], p.sharp))
                    .collect();
                self.ridges.push(Ridge {
                    id,
                    width: self.num(node, "width"),
                    pts: smooth(&src, false, STEP).0,
                });
            }
            "mountain" | "hill" => {
                let shape = if name == "hill" {
                    "dome"
                } else {
                    self.word(
                        ctx,
                        node,
                        "shape",
                        &["cone", "dome", "crag", "crater"],
                        "cone",
                    )
                };
                let h = self.num(node, "h");
                self.mounts.push(Mount {
                    id,
                    x: self.num(node, "x") + ctx.off[0],
                    z: self.num(node, "z") + ctx.off[1],
                    h,
                    r: self.num(node, "r"),
                    shape,
                    rim: self.opt(node, "rim").unwrap_or(h),
                    inner: self.opt(node, "inner").unwrap_or(0.0),
                    floor: self.opt(node, "floor").unwrap_or(h),
                    stretch: self.opt(node, "stretch").unwrap_or(1.0),
                    rot: self.num(node, "rot"),
                });
                self.kids(ctx, node);
            }
            "plateau" => {
                let jag_amp = self.opt(node, "jag").unwrap_or(15.0);
                let cliff = self.word(ctx, node, "edge", &["cliff", "slope"], "cliff") == "cliff";
                let pts = self.outline(ctx, node, jag_amp, seed);
                self.plateaus.push(Plateau {
                    id,
                    top: self.num(node, "top"),
                    cliff,
                    pts,
                });
            }
            "spire" => {
                let top = self.word(ctx, node, "top", &["flat", "point"], "flat");
                self.spires.push(Spire {
                    id,
                    x: self.num(node, "x") + ctx.off[0],
                    z: self.num(node, "z") + ctx.off[1],
                    h: self.num(node, "h"),
                    r: self.num(node, "r"),
                    flat: top == "flat",
                });
                self.kids(ctx, node);
            }
            "valley" | "gorge" | "canal" => {
                let pts = self.points(ctx, node, Pts::D2(&["depth", "sharp"]));
                if name == "gorge" && pts.iter().any(|p| p.depth.is_none()) {
                    self.err(ctx, node, "every gorge point needs depth");
                    return;
                }
                let depth = self.num(node, "depth");
                let src: Vec<([f64; 3], bool)> = pts
                    .iter()
                    .map(|p| ([p.x, p.z, p.depth.unwrap_or(depth)], p.sharp))
                    .collect();
                if src.len() < 2 {
                    self.err(ctx, node, "path needs at least 2 points");
                    return;
                }
                self.carves.push(Carve {
                    id,
                    kind: name_static(name),
                    w: self.num(node, "w"),
                    level: self.num(node, "level"),
                    pts: smooth(&src, false, STEP).0,
                });
                self.kids(ctx, node);
            }
            "river" | "branch" => {
                let pts = self.points(ctx, node, Pts::Any(&["sharp"]));
                // Cave rivers have heights and wait for the cave stage.
                if !ctx.under && pts.iter().all(|p| p.y.is_none()) && pts.len() >= 2 {
                    let src: Vec<([f64; 3], bool)> =
                        pts.iter().map(|p| ([p.x, p.z, 3.0], p.sharp)).collect();
                    self.carves.push(Carve {
                        id,
                        kind: "river",
                        w: self.num(node, "w"),
                        level: 0.0,
                        pts: smooth(&src, false, STEP).0,
                    });
                }
                self.kids(ctx, node);
            }
            "lake" => {
                let level = self.num(node, "level");
                let mut pts = self.outline_or_none(ctx, node, seed);
                if pts.is_empty() {
                    match (
                        self.opt(node, "x"),
                        self.opt(node, "z"),
                        self.opt(node, "r"),
                    ) {
                        (Some(x), Some(z), Some(r)) => {
                            pts = (0..32)
                                .map(|i| {
                                    let (s, c) = sincos_deg(i as f64 * 360.0 / 32.0);
                                    [x + ctx.off[0] + c * r, z + ctx.off[1] + s * r]
                                })
                                .collect();
                        }
                        _ => {
                            self.err(ctx, node, "lake needs an outline or x, z and r");
                            return;
                        }
                    }
                }
                if !ctx.under {
                    self.lakes.push(Lake { id, level, pts });
                }
            }
            "forest" | "meadow" | "city" | "village" => {
                let (kind, density, trees) = match name {
                    "forest" => (
                        "forest",
                        self.word(ctx, node, "kind", &["dense", "open", "rain"], "dense"),
                        self.word(
                            ctx,
                            node,
                            "trees",
                            &["round", "pine", "palm", "jungle"],
                            "round",
                        ),
                    ),
                    "meadow" => ("meadow", "open", "round"),
                    _ => ("town", "open", "round"),
                };
                if name != "forest" {
                    self.word(
                        ctx,
                        node,
                        "tech",
                        &["simple", "old", "mid", "modern"],
                        "mid",
                    );
                }
                let pts = self.outline(ctx, node, 0.0, seed);
                if !ctx.under {
                    self.covers.push(Cover {
                        id,
                        kind,
                        density,
                        trees,
                        pts,
                    });
                }
                self.kids(ctx, node);
            }
            "beacon" | "perch" => self.mark(ctx, node, id, name.into()),
            "site" => {
                let kind = self.str(node, "kind").unwrap_or_default();
                self.mark(ctx, node, id, format!("site:{kind}"));
                self.kids(ctx, node);
            }
            "bowl" => self.mark(ctx, node, id, name.into()),
            "dugout" => {
                self.mark(ctx, node, id.clone(), name.into());
                let r = self.num(node, "r");
                let facing = self.str(node, "facing").unwrap_or_default();
                let (mut fx, mut fz) = (0.0f64, 0.0f64);
                for word in ["north", "south", "east", "west"] {
                    if facing.contains(word) {
                        match word {
                            "north" => fz -= 1.0,
                            "south" => fz += 1.0,
                            "east" => fx += 1.0,
                            _ => fx -= 1.0,
                        }
                    }
                }
                let len: f64 = (fx * fx + fz * fz).sqrt().max(1.0);
                // The room sits just inside the face, opening toward `facing`.
                let (x, y, z) = (
                    self.num(node, "x") + ctx.off[0] - fx / len * r * 0.5,
                    self.num(node, "y"),
                    self.num(node, "z") + ctx.off[1] - fz / len * r * 0.5,
                );
                self.voids.push(Void {
                    id,
                    shape: "dome",
                    center: [x, y + r * 0.3, z],
                    half: [r, r * 0.8, r],
                    rot: 0.0,
                    a: [0.0; 3],
                    b: [0.0; 3],
                    r: 0.0,
                    glow: 0.4,
                });
            }
            "pillar" => {
                self.mark(ctx, node, id.clone(), name.into());
                let (y0, y1) = ctx.cavern.unwrap_or((-1.0e4, 1.0e4));
                self.pillars.push(Pillar {
                    id,
                    x: self.num(node, "x") + ctx.off[0],
                    z: self.num(node, "z") + ctx.off[1],
                    r: self.num(node, "r"),
                    y0,
                    y1,
                });
            }
            "cavern" => {
                self.mark(ctx, node, id.clone(), name.into());
                let (y, h) = (self.num(node, "y"), self.num(node, "h"));
                let shape = self.word(
                    ctx,
                    node,
                    "shape",
                    &["hall", "dome", "shaft", "carved"],
                    "hall",
                );
                self.voids.push(Void {
                    id,
                    shape,
                    center: [
                        self.num(node, "x") + ctx.off[0],
                        y,
                        self.num(node, "z") + ctx.off[1],
                    ],
                    half: [
                        self.num(node, "w") * 0.5,
                        h * 0.5,
                        self.num(node, "d") * 0.5,
                    ],
                    rot: self.num(node, "rot"),
                    a: [0.0; 3],
                    b: [0.0; 3],
                    r: 0.0,
                    glow: self.opt(node, "glow").unwrap_or(0.3),
                });
                let under = Ctx {
                    under: true,
                    cavern: Some((y - h * 0.5, y + h * 0.5)),
                    ..ctx.clone()
                };
                self.kids(&under, node);
            }
            "passage" | "mouth" => {
                let pts = self.points(ctx, node, s.pts);
                let src: Vec<([f64; 3], bool)> = pts
                    .iter()
                    .map(|p| ([p.x, p.y.unwrap_or(0.0), p.z], p.sharp))
                    .collect();
                if src.len() < 2 {
                    self.err(ctx, node, "path needs at least 2 points");
                    return;
                }
                let r = self.num(node, "w") * 0.5;
                // Smooth on x and z (the first two coordinates), so reorder to x z y.
                let xzy: Vec<([f64; 3], bool)> =
                    src.iter().map(|(p, s)| ([p[0], p[2], p[1]], *s)).collect();
                let dense = smooth(&xzy, false, 30.0).0;
                for (i, w) in dense.windows(2).enumerate() {
                    self.voids.push(Void {
                        id: format!("{id}#{i}"),
                        shape: "tube",
                        center: [0.0; 3],
                        half: [0.0; 3],
                        rot: 0.0,
                        a: [w[0][0], w[0][2], w[0][1]],
                        b: [w[1][0], w[1][2], w[1][1]],
                        r,
                        glow: 0.3,
                    });
                }
                let under = Ctx {
                    under: true,
                    ..ctx.clone()
                };
                self.kids(&under, node);
            }
            "caves" => {
                let under = Ctx {
                    under: true,
                    ..ctx.clone()
                };
                self.kids(&under, node);
            }
            "arch" | "shelf" | "road" | "runway" | "route" | "roost" => {
                self.points(ctx, node, s.pts);
                self.kids(ctx, node);
            }
            "falls" | "tier" => {}
            _ => self.err(ctx, node, "not allowed here"),
        }
    }

    fn outline_or_none(&mut self, ctx: &Ctx, node: &KdlNode, seed: u64) -> Vec<[f64; 2]> {
        if node.iter_children().any(|k| k.name().value() == "p") {
            self.outline(ctx, node, 0.0, seed)
        } else {
            Vec::new()
        }
    }

    fn kids(&mut self, ctx: &Ctx, node: &KdlNode) {
        if let Some(doc) = node.children() {
            self.nodes(ctx, doc.nodes());
        }
    }

    fn emit(&self) -> String {
        let mut s = String::new();
        let f = |v: f64| format!("{:?}", v as f32);
        let p2 = |pts: &[[f64; 2]]| {
            pts.iter()
                .map(|p| format!("[{},{}]", f(p[0]), f(p[1])))
                .collect::<Vec<_>>()
                .join(",")
        };
        let p3 = |pts: &[[f64; 3]]| {
            pts.iter()
                .map(|p| format!("[{},{},{}]", f(p[0]), f(p[1]), f(p[2])))
                .collect::<Vec<_>>()
                .join(",")
        };
        let _ = writeln!(s, "// Generated by zawnabout-worldc. Do not edit.");
        let _ = writeln!(s, "pub static WORLD: WorldDef = WorldDef {{");
        let _ = writeln!(s, "name: {:?},", self.name);
        let b = self.bounds;
        let _ = writeln!(
            s,
            "bounds: [{},{},{},{}], edge: {}, sky: {}, deep: {}, sea_level: {}, sea_floor: {},",
            f(b[0]),
            f(b[1]),
            f(b[2]),
            f(b[3]),
            f(self.edge),
            f(self.sky),
            f(self.deep),
            f(self.sea_level),
            f(self.sea_floor)
        );
        let _ = writeln!(s, "spawn: [{},{}],", f(self.spawn[0]), f(self.spawn[1]));
        let _ = writeln!(s, "lands: &[");
        for l in &self.lands {
            let edges: Vec<String> = l.edges.iter().map(|e| e.to_string()).collect();
            let _ = writeln!(
                s,
                "Land {{ id: {:?}, base: {}, pts: &[{}], edges: &[{}] }},",
                l.id,
                f(l.base),
                p2(&l.pts),
                edges.join(",")
            );
        }
        let _ = writeln!(s, "],\nridges: &[");
        for r in &self.ridges {
            let _ = writeln!(
                s,
                "Ridge {{ id: {:?}, width: {}, pts: &[{}] }},",
                r.id,
                f(r.width),
                p3(&r.pts)
            );
        }
        let _ = writeln!(s, "],\nmounts: &[");
        for m in &self.mounts {
            let (sin, cos) = sincos_deg(m.rot);
            let shape = match m.shape {
                "dome" => "Dome",
                "crag" => "Crag",
                "crater" => "Crater",
                _ => "Cone",
            };
            let _ = writeln!(
                s,
                "Mount {{ id: {:?}, x: {}, z: {}, h: {}, r: {}, shape: MountShape::{shape}, rim: {}, inner: {}, floor: {}, stretch: {}, cos: {}, sin: {} }},",
                m.id,
                f(m.x),
                f(m.z),
                f(m.h),
                f(m.r),
                f(m.rim),
                f(m.inner),
                f(m.floor),
                f(m.stretch),
                f(cos),
                f(sin)
            );
        }
        let _ = writeln!(s, "],\nplateaus: &[");
        for p in &self.plateaus {
            let _ = writeln!(
                s,
                "Plateau {{ id: {:?}, top: {}, cliff: {}, pts: &[{}] }},",
                p.id,
                f(p.top),
                p.cliff,
                p2(&p.pts)
            );
        }
        let _ = writeln!(s, "],\nspires: &[");
        for p in &self.spires {
            let _ = writeln!(
                s,
                "Spire {{ id: {:?}, x: {}, z: {}, h: {}, r: {}, flat: {} }},",
                p.id,
                f(p.x),
                f(p.z),
                f(p.h),
                f(p.r),
                p.flat
            );
        }
        let _ = writeln!(s, "],\ncarves: &[");
        for c in &self.carves {
            let kind = match c.kind {
                "valley" => "Valley",
                "gorge" => "Gorge",
                "canal" => "Canal",
                _ => "River",
            };
            let _ = writeln!(
                s,
                "Carve {{ id: {:?}, kind: CarveKind::{kind}, w: {}, level: {}, pts: &[{}] }},",
                c.id,
                f(c.w),
                f(c.level),
                p3(&c.pts)
            );
        }
        let _ = writeln!(s, "],\nlakes: &[");
        for l in &self.lakes {
            let _ = writeln!(
                s,
                "Lake {{ id: {:?}, level: {}, pts: &[{}] }},",
                l.id,
                f(l.level),
                p2(&l.pts)
            );
        }
        let _ = writeln!(s, "],\ncovers: &[");
        for c in &self.covers {
            let cap = |w: &str| {
                let mut c = w.chars();
                c.next()
                    .map(|f| f.to_ascii_uppercase().to_string() + c.as_str())
                    .unwrap_or_default()
            };
            let _ = writeln!(
                s,
                "Cover {{ id: {:?}, kind: CoverKind::{}, density: Density::{}, trees: Trees::{}, pts: &[{}] }},",
                c.id,
                cap(c.kind),
                cap(c.density),
                cap(c.trees),
                p2(&c.pts)
            );
        }
        let _ = writeln!(s, "],\nmarks: &[");
        for m in &self.marks {
            let _ = writeln!(
                s,
                "Mark {{ id: {:?}, kind: {:?}, pos: [{},{},{}], ground: {} }},",
                m.id,
                m.kind,
                f(m.pos[0]),
                f(m.pos[1]),
                f(m.pos[2]),
                m.ground
            );
        }
        let _ = writeln!(s, "],\nvoids: &[");
        for v in &self.voids {
            let (sin, cos) = sincos_deg(v.rot);
            let shape = match v.shape {
                "dome" => "Dome",
                "shaft" => "Shaft",
                "carved" => "Carved",
                "tube" => "Tube",
                _ => "Hall",
            };
            let p3 = |p: [f64; 3]| format!("[{},{},{}]", f(p[0]), f(p[1]), f(p[2]));
            let _ = writeln!(
                s,
                "Void {{ id: {:?}, shape: VoidShape::{shape}, center: {}, half: {}, cos: {}, sin: {}, a: {}, b: {}, r: {}, glow: {} }},",
                v.id,
                p3(v.center),
                p3(v.half),
                f(cos),
                f(sin),
                p3(v.a),
                p3(v.b),
                f(v.r),
                f(v.glow)
            );
        }
        let _ = writeln!(s, "],\npillars: &[");
        for p in &self.pillars {
            let _ = writeln!(
                s,
                "Pillar {{ id: {:?}, x: {}, z: {}, r: {}, y0: {}, y1: {} }},",
                p.id,
                f(p.x),
                f(p.z),
                f(p.r),
                f(p.y0),
                f(p.y1)
            );
        }
        let _ = writeln!(s, "],\n}};");
        s
    }
}

struct Point {
    x: f64,
    y: Option<f64>,
    z: f64,
    sharp: bool,
    edge: Option<String>,
    h: Option<f64>,
    depth: Option<f64>,
}

fn name_static(name: &str) -> &'static str {
    match name {
        "valley" => "valley",
        "gorge" => "gorge",
        "canal" => "canal",
        _ => "river",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_world_compiles() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../world/first");
        let rust = compile(&dir).unwrap_or_else(|e| panic!("{e}"));
        assert!(rust.contains("pub static WORLD"));
    }

    #[test]
    fn sincos_matches() {
        for d in [-725.0f64, -90.0, 0.0, 30.0, 45.0, 180.0, 359.0] {
            let (s, c) = sincos_deg(d);
            let r = d.to_radians();
            assert!(
                (s - r.sin()).abs() < 1e-9 && (c - r.cos()).abs() < 1e-9,
                "{d}"
            );
        }
    }
}
