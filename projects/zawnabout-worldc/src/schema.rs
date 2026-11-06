//! The world vocabulary: which nodes exist, what they take and what they may hold.
//! Property kinds are `n` number, `s` string and `b` bool. See `world/README.md`.

/// Which points a node takes, and the properties each point may carry.
#[derive(Clone, Copy)]
pub enum Pts {
    None,
    D2(&'static [&'static str]),
    D3(&'static [&'static str]),
    Any(&'static [&'static str]),
}

impl Pts {
    pub fn props(self) -> &'static [&'static str] {
        match self {
            Pts::None => &[],
            Pts::D2(p) | Pts::D3(p) | Pts::Any(p) => p,
        }
    }
}

pub struct Spec {
    /// Takes an id as its first argument.
    pub id: bool,
    pub id_optional: bool,
    /// Plain numeric arguments after the id.
    pub nums: usize,
    pub props: &'static [(&'static str, char, bool)],
    pub kids: &'static [&'static str],
    pub pts: Pts,
}

const fn s(
    props: &'static [(&'static str, char, bool)],
    kids: &'static [&'static str],
    pts: Pts,
) -> Spec {
    Spec {
        id: true,
        id_optional: false,
        nums: 0,
        props,
        kids,
        pts,
    }
}

const fn bare(nums: usize, props: &'static [(&'static str, char, bool)]) -> Spec {
    Spec {
        id: false,
        id_optional: false,
        nums,
        props,
        kids: &[],
        pts: Pts::None,
    }
}

const XZ: [(&str, char, bool); 2] = [("x", 'n', true), ("z", 'n', true)];
const MARK: &[(&str, char, bool)] = &[("x", 'n', true), ("y", 'n', false), ("z", 'n', true)];
const MARKS: &[&str] = &["site", "perch", "beacon"];
const TOWN: &[(&str, char, bool)] = &[
    ("style", 's', false),
    ("tech", 's', false),
    ("y", 'n', false),
];
const TOWN_KIDS: &[&str] = &["tier", "shelf", "canal", "site", "beacon", "perch"];

pub fn spec(name: &str) -> Option<Spec> {
    Some(match name {
        "world" => s(
            &[],
            &["bounds", "edge", "sky", "deep", "sea", "spawn"],
            Pts::None,
        ),
        "bounds" => bare(4, &[]),
        "edge" | "sky" | "deep" => bare(1, &[]),
        "sea" => bare(0, &[("level", 'n', true), ("floor", 'n', true)]),
        "spawn" => bare(0, &[("island", 's', true), XZ[0], XZ[1]]),
        "island" => s(
            &[("file", 's', true), XZ[0], XZ[1], ("climate", 's', false)],
            &[],
            Pts::None,
        ),
        "land" => Spec {
            id_optional: true,
            ..s(
                &[("base", 'n', true), ("jag", 'n', false)],
                &[],
                Pts::D2(&["edge", "sharp"]),
            )
        },
        "ridge" => s(
            &[("width", 'n', true), ("smooth", 'b', false)],
            &[],
            Pts::D2(&["h", "sharp"]),
        ),
        "mountain" => s(
            &[
                XZ[0],
                XZ[1],
                ("h", 'n', true),
                ("r", 'n', true),
                ("shape", 's', true),
                ("rim", 'n', false),
                ("inner", 'n', false),
                ("floor", 'n', false),
                ("stretch", 'n', false),
                ("rot", 'n', false),
            ],
            MARKS,
            Pts::None,
        ),
        "hill" => s(
            &[
                XZ[0],
                XZ[1],
                ("h", 'n', true),
                ("r", 'n', true),
                ("stretch", 'n', false),
                ("rot", 'n', false),
            ],
            MARKS,
            Pts::None,
        ),
        "plateau" => s(
            &[
                ("top", 'n', true),
                ("edge", 's', false),
                ("jag", 'n', false),
            ],
            &[],
            Pts::D2(&["sharp"]),
        ),
        "spire" => s(
            &[
                XZ[0],
                XZ[1],
                ("h", 'n', true),
                ("r", 'n', true),
                ("top", 's', false),
            ],
            &["site", "perch", "beacon", "shelf"],
            Pts::None,
        ),
        "arch" => s(&[("w", 'n', true), ("thick", 'n', true)], &[], Pts::D3(&[])),
        "shelf" => s(
            &[("y", 'n', true), ("depth", 'n', true), ("w", 'n', true)],
            &[],
            Pts::D2(&["sharp"]),
        ),
        "pillar" => s(&[XZ[0], XZ[1], ("r", 'n', true)], &[], Pts::None),
        "spout" => s(&[XZ[0], XZ[1], ("w", 'n', true)], &[], Pts::None),
        "valley" => s(
            &[("w", 'n', true), ("depth", 'n', true)],
            &[],
            Pts::D2(&["sharp"]),
        ),
        "gorge" => s(&[("w", 'n', true)], &[], Pts::D2(&["depth", "sharp"])),
        "bowl" => s(
            &[XZ[0], XZ[1], ("r", 'n', true), ("floor", 'n', true)],
            &[],
            Pts::None,
        ),
        "caves" => s(
            &[],
            &["cavern", "passage", "mouth", "river", "dugout"],
            Pts::None,
        ),
        "cavern" => s(
            &[
                XZ[0],
                ("y", 'n', true),
                XZ[1],
                ("w", 'n', true),
                ("h", 'n', true),
                ("d", 'n', true),
                ("rot", 'n', false),
                ("shape", 's', false),
                ("glow", 'n', false),
                ("decor", 's', false),
            ],
            &[
                "pillar", "city", "lake", "beacon", "perch", "site", "ruin", "spout",
            ],
            Pts::None,
        ),
        "passage" => s(
            &[
                ("w", 'n', true),
                ("tight", 'b', false),
                ("decor", 's', false),
                ("glow", 'n', false),
            ],
            &["beacon"],
            Pts::D3(&[]),
        ),
        "mouth" => s(
            &[
                ("kind", 's', true),
                ("w", 'n', true),
                ("hidden", 'b', false),
                ("light", 'b', false),
                ("tight", 'b', false),
                ("decor", 's', false),
            ],
            &[],
            Pts::D3(&[]),
        ),
        "dugout" => s(
            &[
                XZ[0],
                ("y", 'n', true),
                XZ[1],
                ("facing", 's', true),
                ("r", 'n', true),
            ],
            &[],
            Pts::None,
        ),
        "lake" => s(
            &[
                ("level", 'n', true),
                ("x", 'n', false),
                ("z", 'n', false),
                ("r", 'n', false),
            ],
            &[],
            Pts::D2(&["sharp"]),
        ),
        "river" => s(
            &[("w", 'n', true)],
            &["falls", "branch"],
            Pts::Any(&["sharp"]),
        ),
        "falls" => s(&[("h", 'n', true)], &[], Pts::None),
        "branch" => s(&[("w", 'n', true)], &[], Pts::Any(&["sharp"])),
        "canal" => s(
            &[("w", 'n', true), ("level", 'n', true)],
            &[],
            Pts::D2(&["sharp"]),
        ),
        "forest" => s(
            &[("kind", 's', true), ("trees", 's', true)],
            &[],
            Pts::D2(&["sharp"]),
        ),
        "meadow" | "snowfield" => s(&[], &[], Pts::D2(&["sharp"])),
        "city" | "village" => s(TOWN, TOWN_KIDS, Pts::D2(&["sharp"])),
        "tier" => bare(0, &[("y", 'n', true)]),
        "site" => s(
            &[
                ("kind", 's', true),
                ("x", 'n', true),
                ("y", 'n', false),
                ("z", 'n', true),
            ],
            &["runway"],
            Pts::None,
        ),
        "road" => s(&[("w", 'n', true)], &[], Pts::Any(&["sharp", "tunnel"])),
        "runway" => s(&[("w", 'n', true)], &[], Pts::D2(&[])),
        "railway" => s(
            &[
                ("subway", 'b', false),
                ("tour", 'b', false),
                ("mine", 'b', false),
            ],
            &["station"],
            Pts::Any(&["sharp", "tunnel"]),
        ),
        "station" => s(XZ.as_slice(), &[], Pts::None),
        "route" => s(&[("kind", 's', true)], &[], Pts::Any(&["sharp"])),
        "beacon" | "perch" => s(MARK, &[], Pts::None),
        "ruin" => s(
            &[
                ("kind", 's', true),
                ("x", 'n', true),
                ("y", 'n', false),
                ("z", 'n', true),
            ],
            &[],
            Pts::None,
        ),
        "landmark" => s(
            &[
                ("kind", 's', true),
                ("x", 'n', true),
                ("y", 'n', false),
                ("z", 'n', true),
                ("rot", 'n', false),
                ("text", 's', false),
            ],
            &[],
            Pts::None,
        ),
        "roost" => s(&[], &[], Pts::D2(&[])),
        _ => return None,
    })
}
