//! What townsfolk say. Anyone in town can be talked to: a name and a line picked by who
//! they are, the time of day, the weather, where they live, and what the dragon has done.
//! Now and then they pass on a rumor of something shiny.

/// Friendly first names, one per person.
const NAMES: [&str; 32] = [
    "Bea", "Ollie", "Mim", "Rufus", "Tilly", "Jem", "Nan", "Otto", "Wren", "Fitz", "Posy", "Hal",
    "Ivy", "Bart", "Lula", "Sid", "Clem", "Dot", "Ned", "Fay", "Gus", "Hettie", "Arlo", "Mabel",
    "Percy", "Rue", "Toby", "Elsie", "Monty", "Pearl", "Wally", "Iris",
];

const ANY: &[&str] = &[
    "Oh! A dragon! Hello! Sorry, I don't meet many dragons.",
    "Lovely day for flying, I should think. Not that I'd know.",
    "My neighbor says dragons are good luck. I hope she's right.",
    "Mind the washing on the line when you land, dear.",
    "I've lived here all my life and I still find new corners.",
    "If you're ever passing, I make a very good cup of tea.",
    "You're bigger than I expected. In a nice way!",
    "The gulls have been very cheeky lately. Very cheeky.",
    "Have you been up to the summit? I hear you can see everything.",
    "I was just going to see a friend. Lovely to meet you, though!",
    "Do dragons like soup? I've got far too much soup.",
    "My grandad says there used to be lots of dragons. Where did they all go?",
    "Every island has its own song, they say. Listen for it.",
    "Fly safe! Well, you probably always do.",
];

const MORNING: &[&str] = &[
    "Good morning! The bakery's just opened, if you smell that.",
    "Early bird, are you? Early dragon, I mean.",
    "Morning mist on the hills. My favorite.",
];

const EVENING: &[&str] = &[
    "The sunset's lovely from the cliffs. You should go and look.",
    "Nearly supper time. The whole town smells of it.",
    "Lamps are coming on. It always feels cozy then.",
];

const NIGHT: &[&str] = &[
    "Out late? So am I. Shh, don't tell anyone.",
    "Look at the stars tonight! You can see every one.",
    "The town's ever so quiet at night. Just the waves.",
];

const RAIN: &[&str] = &[
    "Bit wet today! Good for the gardens, mind.",
    "I forgot my umbrella again. Every single time.",
    "Rain on the roof is the best sound for sleeping.",
];

const UNDER: &[&str] = &[
    "Welcome to the deep! It's never cold down here.",
    "Up top they have the sun. Down here we have the glow. I prefer the glow.",
    "Mind the mushrooms, some of them are very old.",
    "Sometimes I hear water far below. A whole river, they say.",
];

const KIND: &[&str] = &[
    "Everyone's talking about a kind dragon who helps people. Is that you?",
    "You're the helpful one! My friend told me all about you.",
    "Thank you for looking after everyone. It means a lot.",
];

const DARK: &[&str] = &[
    "Since the lights went out, nobody goes out after dark. It's ever so gloomy.",
    "Tobin the Lamplighter says his old lamps could wake the lights again. If only.",
    "I keep bumping into things at night. I miss the street lamps.",
];

const RELIT: &[&str] = &[
    "The lights are back! Was that you? Thank you, dragon!",
    "Look at the windows all glowing again. Isn't it lovely?",
    "I'm staying up late tonight, just to look at the lamps.",
];

const RUMOR: &[&str] = &[
    "I'm sure I saw something glinting near {}.",
    "My cousin says there's treasure near {}. Probably nonsense. Probably.",
    "Something shiny near {}. The gulls were very excited about it.",
];

/// Everything a townsperson's line can depend on.
pub struct Scene {
    pub hour: f32,
    pub rain: bool,
    pub under: bool,
    /// Favors done anywhere.
    pub kind: usize,
    /// A place with something still to find, for a rumor.
    pub rumor: Option<String>,
    /// Something said about a course or race, word for word.
    pub dare: Option<String>,
    /// While the lights story runs: whether it is still dark here, or lit again.
    pub dark: Option<bool>,
    /// While the dragon has no lair: a tip about where one is, word for word.
    pub lair: Option<String>,
}

fn mix(mut x: u32) -> u32 {
    x ^= x >> 16;
    x = x.wrapping_mul(0x7FEB_352D);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846C_A68B);
    x ^ (x >> 16)
}

/// The name of townsperson `seed`.
pub fn name(seed: u32) -> &'static str {
    NAMES[mix(seed) as usize % NAMES.len()]
}

/// What townsperson `seed` says the `nth` time anyone is talked to.
pub fn line(seed: u32, nth: u32, scene: &Scene) -> String {
    let pick = mix(seed ^ nth.wrapping_mul(0x9E37_79B9));
    if let Some(place) = &scene.rumor
        && pick.is_multiple_of(3)
    {
        return RUMOR[(pick / 3) as usize % RUMOR.len()].replace("{}", place);
    }
    if let Some(tip) = &scene.lair
        && pick % 4 == 1
    {
        return tip.clone();
    }
    if let Some(dark) = scene.dark
        && pick.is_multiple_of(2)
    {
        let pool = if dark { DARK } else { RELIT };
        return pool[(pick / 2) as usize % pool.len()].to_string();
    }
    if let Some(dare) = &scene.dare
        && pick % 5 == 1
    {
        return dare.clone();
    }
    let mut pools: Vec<&[&str]> = vec![ANY];
    if scene.under {
        pools.push(UNDER);
        pools.push(UNDER);
    } else if scene.rain {
        pools.push(RAIN);
    }
    if !scene.under {
        if (5.0..10.0).contains(&scene.hour) {
            pools.push(MORNING);
        } else if (18.0..21.5).contains(&scene.hour) {
            pools.push(EVENING);
        } else if !(5.0..21.5).contains(&scene.hour) {
            pools.push(NIGHT);
        }
    }
    if scene.kind > 0 {
        pools.push(KIND);
    }
    let pool = pools[(pick / 7) as usize % pools.len()];
    pool[(pick / 31) as usize % pool.len()].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Lines come out whole, a rumor names its place, and names are stable.
    #[test]
    fn lines_and_rumors() {
        let scene = Scene {
            hour: 8.0,
            rain: false,
            under: false,
            kind: 1,
            rumor: Some("Kettle Light".into()),
            lair: None,
            dare: Some("Fly under the bridges!".into()),
            dark: None,
        };
        let lines: Vec<String> = (0..40).map(|n| line(7, n, &scene)).collect();
        assert!(lines.iter().all(|l| !l.is_empty() && !l.contains("{}")));
        assert!(lines.iter().any(|l| l.contains("Kettle Light")));
        assert!(lines.iter().any(|l| l.contains("bridges")));
        assert_eq!(name(5), name(5));
    }
}
