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
    /// Lakes in caves, each sitting in a basin void.
    pub pools: &'static [Lake],
    /// Parents come before their branches.
    pub rivers: &'static [River],
    pub towns: &'static [Town],
    pub shelves: &'static [Shelf],
    pub roads: &'static [Road],
    pub routes: &'static [Route],
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
    /// No crags: an even slope, such as for ski runs.
    pub smooth: bool,
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
    /// Snow lying on the ground, such as ski runs.
    Snow,
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
    /// A lake basin in a cave floor: `half` is the radii, with depth as y, and `center.y`
    /// the water level.
    Basin,
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
    pub decor: Decor,
    /// A mouth that lets daylight stream in.
    pub light: bool,
}

/// What grows or stands in a cavern. Placed by the sim, with crystal and plant colors
/// picked per cavern.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decor {
    Bare,
    /// Some of everything.
    Mixed,
    Crystal,
    Shroom,
    /// Warm, old lava rooms: amber crystals and glowing cracks.
    Ember,
    /// Fallen stonework of an old people.
    Ruins,
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

/// A river or branch. Points are x, z and, for cave rivers, a rough water height.
#[derive(Debug)]
pub struct River {
    pub id: &'static str,
    pub w: f32,
    pub under: bool,
    /// Index of the river this branch joins, or -1.
    pub parent: i32,
    /// The branch leaves its parent (a delta arm) instead of flowing into it.
    pub from_parent: bool,
    pub pts: &'static [[f32; 3]],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Style {
    Plain,
    Canal,
    Hill,
    Cliff,
    Stone,
    Seafaring,
    Modern,
    Cavern,
    Stilts,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Tech {
    Simple,
    Old,
    Mid,
    Modern,
}

/// A city or village. `tiers` are terrace heights, lowest first. Cave towns sit at `y`.
#[derive(Debug)]
pub struct Town {
    pub id: &'static str,
    pub style: Style,
    pub tech: Tech,
    pub village: bool,
    pub under: bool,
    pub y: f32,
    pub tiers: &'static [f32],
    pub pts: &'static [[f32; 2]],
}

/// A flat ledge at height `y` along a path, `w` wide.
#[derive(Debug)]
pub struct Shelf {
    pub id: &'static str,
    pub y: f32,
    pub depth: f32,
    pub w: f32,
    pub pts: &'static [[f32; 2]],
}

/// Heights below this in road points mean "follow the ground".
pub const NO_Y: f32 = -1.0e4;

/// A path a vehicle follows: `kind` is ferry, yacht, seaplane or plane. Points are x, z
/// and a height or [`NO_Y`] for sea level.
#[derive(Debug)]
pub struct Route {
    pub id: &'static str,
    pub kind: &'static str,
    /// Ends where it starts: it goes round instead of back and forth.
    pub looped: bool,
    pub pts: &'static [[f32; 3]],
}

/// A road or runway. Points are x, z and a forced height or [`NO_Y`].
#[derive(Debug)]
pub struct Road {
    pub id: &'static str,
    pub runway: bool,
    /// A railway: trains instead of cars, track instead of tarmac.
    pub rail: bool,
    pub w: f32,
    pub pts: &'static [[f32; 3]],
}

include!(concat!(env!("OUT_DIR"), "/world.rs"));
