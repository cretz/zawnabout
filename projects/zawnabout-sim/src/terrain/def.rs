//! The world as data, generated at build time from `world/first/*.kdl` by
//! `zawnabout-worldc`. Points are world space. Ids are `island/name`.

#[derive(Debug)]
pub struct WorldDef {
    pub name: &'static str,
    /// Min x, min z, max x, max z.
    pub bounds: [f32; 4],
    /// Width of the soft edge band inside the bounds.
    pub edge: f32,
    pub sky: f32,
    pub deep: f32,
    pub sea_level: f32,
    pub sea_floor: f32,
    pub spawn: [f32; 2],
    pub lands: &'static [Land],
    pub ridges: &'static [Ridge],
    pub mounts: &'static [Mount],
    pub plateaus: &'static [Plateau],
    pub spires: &'static [Spire],
    pub carves: &'static [Carve],
    pub lakes: &'static [Lake],
    pub covers: &'static [Cover],
    pub marks: &'static [Mark],
    pub voids: &'static [Void],
    pub pillars: &'static [Pillar],
}

/// An island outline. `edges[i]` is the coast kind from point `i` to the next:
/// 0 beach, 1 rocky, 2 cliff.
#[derive(Debug)]
pub struct Land {
    pub id: &'static str,
    pub base: f32,
    pub pts: &'static [[f32; 2]],
    pub edges: &'static [u8],
}

/// A mountain range: points are x, z and crest height.
#[derive(Debug)]
pub struct Ridge {
    pub id: &'static str,
    pub width: f32,
    pub pts: &'static [[f32; 3]],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MountShape {
    Cone,
    Dome,
    Crag,
    Crater,
}

#[derive(Debug)]
pub struct Mount {
    pub id: &'static str,
    pub x: f32,
    pub z: f32,
    pub h: f32,
    pub r: f32,
    pub shape: MountShape,
    pub rim: f32,
    pub inner: f32,
    pub floor: f32,
    pub stretch: f32,
    pub cos: f32,
    pub sin: f32,
}

#[derive(Debug)]
pub struct Plateau {
    pub id: &'static str,
    pub top: f32,
    pub cliff: bool,
    pub pts: &'static [[f32; 2]],
}

#[derive(Debug)]
pub struct Spire {
    pub id: &'static str,
    pub x: f32,
    pub z: f32,
    pub h: f32,
    pub r: f32,
    pub flat: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CarveKind {
    Valley,
    Gorge,
    River,
    Canal,
}

/// A cut along a path: points are x, z and depth.
#[derive(Debug)]
pub struct Carve {
    pub id: &'static str,
    pub kind: CarveKind,
    pub w: f32,
    /// Water level, for canals.
    pub level: f32,
    pub pts: &'static [[f32; 3]],
}

#[derive(Debug)]
pub struct Lake {
    pub id: &'static str,
    pub level: f32,
    pub pts: &'static [[f32; 2]],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CoverKind {
    Forest,
    Meadow,
    Town,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Density {
    Dense,
    Open,
    Rain,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Trees {
    Round,
    Pine,
    Palm,
    Jungle,
}

#[derive(Debug)]
pub struct Cover {
    pub id: &'static str,
    pub kind: CoverKind,
    pub density: Density,
    pub trees: Trees,
    pub pts: &'static [[f32; 2]],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VoidShape {
    /// Wide rounded room with a flattish floor and roof.
    Hall,
    /// Rounded room with a flat floor.
    Dome,
    /// Tall rounded column.
    Shaft,
    /// Straight walls cut by people long ago.
    Carved,
    /// One capsule of a passage or mouth, from `a` to `b` with radius `r`.
    Tube,
}

/// An empty space in the rock. Rooms use `center`, `half` extents and a turn about y,
/// tubes use `a`, `b` and `r`. `glow` is the base light inside.
#[derive(Debug)]
pub struct Void {
    pub id: &'static str,
    pub shape: VoidShape,
    pub center: [f32; 3],
    pub half: [f32; 3],
    pub cos: f32,
    pub sin: f32,
    pub a: [f32; 3],
    pub b: [f32; 3],
    pub r: f32,
    pub glow: f32,
}

/// Rock left standing inside a cavern, from floor `y0` to roof `y1`.
#[derive(Debug)]
pub struct Pillar {
    pub id: &'static str,
    pub x: f32,
    pub z: f32,
    pub r: f32,
    pub y0: f32,
    pub y1: f32,
}

/// A named point: beacon, perch, spawn, `site:<kind>`, dugout, cavern and so on.
#[derive(Debug)]
pub struct Mark {
    pub id: &'static str,
    pub kind: &'static str,
    pub pos: [f32; 3],
    /// No height given, so it sits on the ground.
    pub ground: bool,
}

include!(concat!(env!("OUT_DIR"), "/world.rs"));
