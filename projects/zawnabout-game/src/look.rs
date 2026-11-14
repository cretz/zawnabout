//! How the player's dragon looks: colors from hand-picked swatches, a pattern, horns, a
//! crest, a tail tip, a name, and something from the hoard to wear. All of it is open
//! from the start. It lives in the prefs, so it is there with gameplay off too.

use crate::mesh::Rgb;

/// Body colors: the sides, then the darker back.
pub const BODY: [(&str, u32, u32); 12] = [
    ("Coral", 0xF2845C, 0xD9503A),
    ("Sky", 0x6FB7E8, 0x3E7FC4),
    ("Leaf", 0x7CC66A, 0x4A9A4E),
    ("Violet", 0xA98BE0, 0x7458B8),
    ("Sunny", 0xF5C445, 0xE0902A),
    ("Rose", 0xF29AB8, 0xD9608A),
    ("Teal", 0x4FC1B5, 0x2E8A8F),
    ("Midnight", 0x5A6AA8, 0x343E78),
    ("Snow", 0xF2F0EA, 0xC2C8D8),
    ("Plum", 0xB0607E, 0x7A3A5A),
    ("Copper", 0xC8784A, 0x8E4A2E),
    ("Mint", 0x9FE0C0, 0x5DB08F),
];

/// Belly plates, in two shades.
pub const BELLY: [(&str, u32, u32); 7] = [
    ("Cream", 0xFFE2B0, 0xF8CF94),
    ("Peach", 0xFFD0B8, 0xF8BC9E),
    ("Butter", 0xFFF2A8, 0xF5E08A),
    ("Mint", 0xD8F5DC, 0xC2EAC8),
    ("Sky", 0xDCEEFF, 0xC4DEF8),
    ("Lilac", 0xEADCFF, 0xDACAF5),
    ("Snow", 0xFFFFFF, 0xEEEEF2),
];

/// Wings, ear frills and feathers: the pale edge, then the deep root.
pub const WINGS: [(&str, u32, u32); 7] = [
    ("Peach", 0xFFD49A, 0xEE7B4C),
    ("Sky", 0xBFE4FF, 0x5A9AE0),
    ("Lilac", 0xE2D0FF, 0x9A78D8),
    ("Mint", 0xCFF5DD, 0x5CB88A),
    ("Rose", 0xFFD0DE, 0xE0708F),
    ("Gold", 0xFFE7A0, 0xE0A030),
    ("Dusk", 0xC8C8F0, 0x5A5A9A),
];

/// Crest blades, horn tips, claws and whiskers: the metal, then its shine.
pub const TRIM: [(&str, u32, u32); 6] = [
    ("Gold", 0xF5B62E, 0xFFE6A0),
    ("Silver", 0xB8C0CC, 0xF2F4F8),
    ("Rose gold", 0xE8A08A, 0xFFD8C8),
    ("Jade", 0x4FB88A, 0xB8EED0),
    ("Pearl", 0xE6DEF0, 0xFFFFFF),
    ("Bronze", 0xB07A3A, 0xE0B070),
];

pub const EYES: [(&str, u32); 5] = [
    ("Night", 0x2B2240),
    ("Ocean", 0x1F4A8A),
    ("Forest", 0x245A2A),
    ("Amber", 0x8A4A10),
    ("Berry", 0x7A1F4A),
];

pub const PATTERNS: [&str; 4] = ["Classic", "Stripes", "Spots", "Plain"];
pub const HORNS: [&str; 4] = ["Swept", "Short", "Long", "Curled"];
pub const CRESTS: [&str; 4] = ["Blades", "Little blades", "Feathers", "None"];
pub const TAILS: [&str; 3] = ["Feather fan", "Spade", "Tuft"];

/// Hoard finds that can be worn, on the head or round the neck.
pub const WEARABLE: [&str; 5] = ["crown", "hat", "ribbon", "medal", "gem"];

/// Names to pick from.
pub const NAMES: [&str; 24] = [
    "Ember", "Pip", "Sunny", "Biscuit", "Juniper", "Pebble", "Maple", "Saffron", "Marigold",
    "Clover", "Wren", "Toffee", "Puddle", "Comet", "Bramble", "Nutmeg", "Skye", "Poppy", "Fig",
    "Hazel", "Basil", "Tumble", "Sprout", "Mochi",
];

/// The rows the journal's You page lists, in order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Row {
    Body,
    Belly,
    Wings,
    Trim,
    Eyes,
    Pattern,
    Horns,
    Crest,
    Tail,
    Wear,
    Name,
    Surprise,
}

impl Row {
    pub const ALL: [Row; 12] = [
        Row::Body,
        Row::Belly,
        Row::Wings,
        Row::Trim,
        Row::Eyes,
        Row::Pattern,
        Row::Horns,
        Row::Crest,
        Row::Tail,
        Row::Wear,
        Row::Name,
        Row::Surprise,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Row::Body => "Scales",
            Row::Belly => "Belly",
            Row::Wings => "Wings",
            Row::Trim => "Trim",
            Row::Eyes => "Eyes",
            Row::Pattern => "Pattern",
            Row::Horns => "Horns",
            Row::Crest => "Crest",
            Row::Tail => "Tail tip",
            Row::Wear => "Wearing",
            Row::Name => "Name",
            Row::Surprise => "Surprise me",
        }
    }

    /// Whether its choices are colors, shown as swatches.
    pub fn swatches(self) -> bool {
        matches!(
            self,
            Row::Body | Row::Belly | Row::Wings | Row::Trim | Row::Eyes
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Look {
    pub body: usize,
    pub belly: usize,
    pub wings: usize,
    pub trim: usize,
    pub eyes: usize,
    pub pattern: usize,
    pub horns: usize,
    pub crest: usize,
    pub tail: usize,
    /// The hoard find worn, by its kind.
    pub wear: Option<String>,
    pub name: String,
}

impl Look {
    /// A look picked at random from `seed`, wearing nothing.
    pub fn random(seed: u64) -> Self {
        let mut s = seed ^ 0x9E37_79B9_7F4A_7C15;
        let mut pick = |n: usize| {
            s ^= s << 13;
            s ^= s >> 7;
            s ^= s << 17;
            (s % n as u64) as usize
        };
        Self {
            body: pick(BODY.len()),
            belly: pick(BELLY.len()),
            wings: pick(WINGS.len()),
            trim: pick(TRIM.len()),
            eyes: pick(EYES.len()),
            pattern: pick(PATTERNS.len()),
            horns: pick(HORNS.len()),
            crest: pick(CRESTS.len()),
            tail: pick(TAILS.len()),
            wear: None,
            name: NAMES[pick(NAMES.len())].to_string(),
        }
    }

    /// Its name, or a plain one before it has one.
    pub fn name(&self) -> &str {
        if self.name.is_empty() {
            "Dragon"
        } else {
            &self.name
        }
    }

    /// The choices on `row`, by name. Wearing lists nothing, then each of `wearables`.
    pub fn choices(row: Row, wearables: &[String]) -> Vec<String> {
        let names = |v: &[&str]| v.iter().map(|s| s.to_string()).collect();
        match row {
            Row::Body => names(&BODY.map(|c| c.0)),
            Row::Belly => names(&BELLY.map(|c| c.0)),
            Row::Wings => names(&WINGS.map(|c| c.0)),
            Row::Trim => names(&TRIM.map(|c| c.0)),
            Row::Eyes => names(&EYES.map(|c| c.0)),
            Row::Pattern => names(&PATTERNS),
            Row::Horns => names(&HORNS),
            Row::Crest => names(&CRESTS),
            Row::Tail => names(&TAILS),
            Row::Wear => std::iter::once("Nothing".to_string())
                .chain(wearables.iter().cloned())
                .collect(),
            Row::Name | Row::Surprise => Vec::new(),
        }
    }

    /// Each swatch's color on a color row.
    pub fn swatch(row: Row, k: usize) -> u32 {
        match row {
            Row::Body => BODY[k].1,
            Row::Belly => BELLY[k].1,
            Row::Wings => WINGS[k].2,
            Row::Trim => TRIM[k].1,
            Row::Eyes => EYES[k].1,
            _ => 0,
        }
    }

    /// Which choice on `row` it has.
    pub fn chosen(&self, row: Row, wearables: &[String]) -> usize {
        match row {
            Row::Body => self.body,
            Row::Belly => self.belly,
            Row::Wings => self.wings,
            Row::Trim => self.trim,
            Row::Eyes => self.eyes,
            Row::Pattern => self.pattern,
            Row::Horns => self.horns,
            Row::Crest => self.crest,
            Row::Tail => self.tail,
            Row::Wear => self
                .wear
                .as_ref()
                .and_then(|w| wearables.iter().position(|k| k == w))
                .map_or(0, |i| i + 1),
            Row::Name | Row::Surprise => 0,
        }
    }

    /// Takes choice `k` on `row`.
    pub fn choose(&mut self, row: Row, k: usize, wearables: &[String]) {
        match row {
            Row::Body => self.body = k % BODY.len(),
            Row::Belly => self.belly = k % BELLY.len(),
            Row::Wings => self.wings = k % WINGS.len(),
            Row::Trim => self.trim = k % TRIM.len(),
            Row::Eyes => self.eyes = k % EYES.len(),
            Row::Pattern => self.pattern = k % PATTERNS.len(),
            Row::Horns => self.horns = k % HORNS.len(),
            Row::Crest => self.crest = k % CRESTS.len(),
            Row::Tail => self.tail = k % TAILS.len(),
            Row::Wear => self.wear = k.checked_sub(1).and_then(|i| wearables.get(i)).cloned(),
            Row::Name | Row::Surprise => {}
        }
    }

    /// The next choice on `row`, round to the first after the last. The next name in the
    /// list for Name.
    pub fn next(&mut self, row: Row, wearables: &[String]) {
        if row == Row::Name {
            let at = NAMES.iter().position(|n| *n == self.name);
            self.name = NAMES[at.map_or(0, |i| (i + 1) % NAMES.len())].to_string();
            return;
        }
        let n = Self::choices(row, wearables).len().max(1);
        self.choose(row, (self.chosen(row, wearables) + 1) % n, wearables);
    }

    /// Every swap from the stock coral dragon's colors to this look's.
    pub fn colors(&self) -> Vec<(Rgb, Rgb)> {
        use crate::models::palette as p;
        let (scale, back) = (Rgb(BODY[self.body].1), Rgb(BODY[self.body].2));
        let (belly, belly2) = (Rgb(BELLY[self.belly].1), Rgb(BELLY[self.belly].2));
        let (pale, deep) = (Rgb(WINGS[self.wings].1), Rgb(WINGS[self.wings].2));
        let (gold, shine) = (Rgb(TRIM[self.trim].1), Rgb(TRIM[self.trim].2));
        let eye = Rgb(EYES[self.eyes].1);
        let mut out = vec![
            (p::SCALE, scale),
            (p::BACK, back),
            (p::BACK.mix(p::SCALE, 0.55), back.mix(scale, 0.55)),
            (p::BELLY, belly),
            (p::BELLY2, belly2),
            (p::MEMBRANE, pale),
            (p::MEMBRANE_DEEP, deep),
            (p::GOLD, gold),
            (p::GOLD_LIGHT, shine),
            (p::EYE, eye),
        ];
        // The stock wings keep their own frills and feathers.
        if self.wings == 0 {
            return out;
        }
        let tip = pale.mix(Rgb(0xFFFFFF), 0.5);
        out.extend([
            (p::FRILL, pale.mix(deep, 0.3)),
            (p::FEATHER, deep),
            (p::FEATHER_TIP, tip),
            (p::FEATHER.mix(p::FEATHER_TIP, 0.45), deep.mix(tip, 0.45)),
        ]);
        out
    }

    /// One prefs line.
    pub fn text(&self) -> String {
        format!(
            "look {} {} {} {} {} {} {} {} {} {} {}",
            self.body,
            self.belly,
            self.wings,
            self.trim,
            self.eyes,
            self.pattern,
            self.horns,
            self.crest,
            self.tail,
            self.wear.as_deref().unwrap_or("-"),
            self.name.replace('\n', " "),
        )
    }

    /// Reads a prefs line's words after `look`, none when they do not fit.
    pub fn parse(words: &str) -> Option<Self> {
        let mut it = words.splitn(11, ' ');
        let mut n = |max: usize| -> Option<usize> {
            let v: usize = it.next()?.parse().ok()?;
            (v < max).then_some(v)
        };
        let mut look = Self {
            body: n(BODY.len())?,
            belly: n(BELLY.len())?,
            wings: n(WINGS.len())?,
            trim: n(TRIM.len())?,
            eyes: n(EYES.len())?,
            pattern: n(PATTERNS.len())?,
            horns: n(HORNS.len())?,
            crest: n(CRESTS.len())?,
            tail: n(TAILS.len())?,
            ..Self::default()
        };
        let wear = it.next()?;
        look.wear = WEARABLE.contains(&wear).then(|| wear.to_string());
        look.name = it.next().unwrap_or("").trim().chars().take(24).collect();
        Some(look)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefs_round_trip() {
        for seed in 0..50 {
            let mut look = Look::random(seed);
            if seed % 2 == 0 {
                look.wear = Some("crown".into());
                look.name = "Ember the Bold".into();
            }
            let line = look.text();
            let back = Look::parse(line.strip_prefix("look ").unwrap()).unwrap();
            assert_eq!(back, look);
        }
    }

    /// Every shape builds, and a new color really reaches the scales.
    #[test]
    fn every_look_builds() {
        let stock = crate::models::dragon();
        for pattern in 0..PATTERNS.len() {
            for horns in 0..HORNS.len() {
                for crest in 0..CRESTS.len() {
                    for tail in 0..TAILS.len() {
                        let look = Look {
                            body: 1,
                            pattern,
                            horns,
                            crest,
                            tail,
                            ..Look::default()
                        };
                        let m = crate::models::dragon_look(&look);
                        assert!(!m.head.verts.is_empty() && !m.plume.verts.is_empty());
                        let coral = Rgb(BODY[0].1).bytes();
                        assert!(m.body.verts.iter().all(|v| v.color != coral));
                    }
                }
            }
        }
        let same = crate::models::dragon_look(&Look::default());
        assert_eq!(same.body.verts.len(), stock.body.verts.len());
    }

    #[test]
    fn next_goes_round() {
        let worn = vec!["crown".to_string(), "hat".to_string()];
        let mut look = Look::default();
        for _ in 0..3 {
            look.next(Row::Wear, &worn);
        }
        assert_eq!(look.wear, None);
        look.next(Row::Wear, &worn);
        assert_eq!(look.wear.as_deref(), Some("crown"));
        for _ in 0..BODY.len() {
            look.next(Row::Body, &worn);
        }
        assert_eq!(look.body, 0);
    }
}
