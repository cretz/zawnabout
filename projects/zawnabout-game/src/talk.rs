//! What townsfolk say. Anyone in town can be talked to: a name and a line picked by who
//! they are, the time of day, the weather, where they live, and what the dragon has done.
//! Now and then they pass on a rumor of something shiny. Nobody says what someone else
//! already said: lines heard are skipped, and once the written ones run out, new ones are
//! put together from parts.

/// Friendly first names, one per person.
const NAMES: [&str; 32] = [
    "Bea", "Ollie", "Mim", "Rufus", "Tilly", "Jem", "Nan", "Otto", "Wren", "Fitz", "Posy", "Hal",
    "Ivy", "Bart", "Lula", "Sid", "Clem", "Dot", "Ned", "Fay", "Gus", "Hettie", "Arlo", "Mabel",
    "Percy", "Rue", "Toby", "Elsie", "Monty", "Pearl", "Wally", "Iris",
];

const ANY: &[&str] = &[
    "Oh! A dragon! Hello! Sorry, I don't meet many dragons.",
    "I've been learning the accordion. The neighbors are being very patient.",
    "Is it true dragons can see in the dark? I can barely see in the light.",
    "I'm off to the post office. Do dragons get letters? I'd write you one.",
    "My cat would love you. She loves anything that sits still and is warm.",
    "I lost a button this morning. Not a big loss, but it was my favorite button.",
    "We're having a picnic on Sunday. Bring a friend, if dragons have friends.",
    "I painted my door yellow last week. Everyone says it's very cheerful.",
    "Have you tried the lemonade by the fair? It's mostly sugar. Wonderful.",
    "I used to be scared of heights. Then I got a bicycle with a bell.",
    "Do you ever get hiccups? I imagine dragon hiccups are quite something.",
    "There's a radio show at eight about the weather. It's mostly about clouds.",
    "My little brother says you're a lizard with wings. I told him to be polite.",
    "I'm saving up for roller skates. The red ones, with the stripe.",
    "The ferry captain waves at everyone. Even the seagulls. Even me!",
    "I knit a scarf so long it went twice round the house. Want it?",
    "I keep a diary. Today's page is going to be all about you.",
    "Have you met the lighthouse keeper? She knows every star by name.",
    "I'm practicing my whistle. Listen! ...No, that wasn't it.",
    "If you see a blue kite stuck somewhere, that's mine. From 1979.",
    "Some days I just sit by the water and count boats. Twelve so far!",
    "My grandmother says a dragon flew over the day I was born. I like to think so.",
    "The bus driver lets me ring the bell. It's the best part of my day.",
    "I'm going to be a pilot when I grow up. Or a dragon. Can you learn to be one?",
    "Our garden has a gnome called Gerald. He's seen things.",
    "I always wave at the train. The driver always waves back. We're friends now.",
    "There's a bakery that does a cinnamon bun the size of my head.",
    "We had a power cut last week and everyone came out to look at the stars.",
    "I collect shells. I have four hundred and six. Now four hundred and seven.",
    "My dog chases the gulls. He's never caught one. He never will. He's happy.",
    "I'm reading a book about dragons. You're much friendlier than in the book.",
    "The fishing boats come back at noon. Everyone goes down to see the catch.",
    "I made a sandcastle with a moat this morning. The sea took it. Fair enough.",
    "Do you sleep upside down? No, that's bats. Sorry!",
    "Somebody's been leaving flowers on the bench by the square. I think it's lovely.",
    "My uncle swears he saw a whale wink at him. Twice.",
    "I tried to make a hot air balloon once. It was mostly a hot air bag.",
    "The postman whistles the same song every day. Now I can't stop humming it.",
    "If you fly high enough, can you see tomorrow? That's what my sister thinks.",
    "There's a jigsaw at the library with a dragon on it. Four pieces missing.",
    "The clock tower is two minutes fast. Nobody fixes it. It's tradition now.",
    "I named my goldfish after the mayor. They have the same mustache.",
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
    "Milk's just been delivered. Clink clink, every morning, right on time.",
    "I'm off to open the shop. First customer gets a free sweet. Is that you?",
    "The dew's still on the grass. It's like the whole island got washed.",
    "Breakfast was porridge. It's always porridge. I like porridge.",
    "Early bird, are you? Early dragon, I mean.",
    "Morning mist on the hills. My favorite.",
];

const EVENING: &[&str] = &[
    "The sunset's lovely from the cliffs. You should go and look.",
    "Off home for supper. It's fish pie night. Best night.",
    "The swallows go mad this time of day. Look at them go!",
    "My feet are tired. Good tired, though. Busy day.",
    "Someone's playing a trumpet down by the water. Every evening. Badly. Lovely.",
    "Nearly supper time. The whole town smells of it.",
    "Lamps are coming on. It always feels cozy then.",
];

const NIGHT: &[&str] = &[
    "Out late? So am I. Shh, don't tell anyone.",
    "Couldn't sleep, so I came out to listen to the owls.",
    "The night bus comes at midnight. It's always empty and always on time.",
    "I'm walking the dog. He thinks it's an adventure. It is, a bit.",
    "See the lights out on the water? Night fishing. They sing to pass the time.",
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
    "We grow the best mushrooms on any island. Up top they don't believe us.",
    "My grandad dug this street with a spoon. Well, he says he did.",
    "The glowworms come out brighter when it's quiet. Listen.",
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

/// What a rider says on getting there, a different one each time round.
const ARRIVED: &[&str] = &[
    "Thank you, dragon! What a ride. I'll remember that one for ever.",
    "Here already? That was quicker than the bus, and much more fun.",
    "My hair will never be the same. Worth it! Thank you!",
    "I waved at everyone on the way. I hope they saw me!",
    "Down we come. You're a gentle lander, you know that?",
    "Perfect. Just in time, too. You're a star.",
    "Wait till I tell everyone I came by dragon!",
    "Thank you! My knees are wobbly, but my heart is happy.",
];

/// What someone says when the parcel gets there.
const DELIVERED: &[&str] = &[
    "Somebody here was waiting for that, and now they're beaming. Special delivery, by dragon.",
    "Oh, it's here! Tell them thank you. And thank you too, dragon.",
    "Delivered by dragon! Nobody's going to believe the postmark.",
    "Just what we were hoping for. Not a scratch on it, either!",
    "There's a happy face at the door. That's your doing.",
    "Signed, sealed, delivered, by the fastest wings around.",
];

/// What someone with a favor says while another is under way.
const LATER: &[&str] = &[
    "Oh, you're busy helping someone already! Come back when that's done and I'll tell you mine.",
    "You've got your claws full! Finish that one first, then come and find me.",
    "One thing at a time, dragon. I'll be right here when you're free.",
    "I could use a hand, but it can wait. Go on, finish what you started.",
];

/// Says thanks for a ride, `nth` of them so far.
pub fn arrived(nth: usize) -> &'static str {
    ARRIVED[nth % ARRIVED.len()]
}

/// Says thanks for a delivery, `nth` of them so far.
pub fn delivered(nth: usize) -> &'static str {
    DELIVERED[nth % DELIVERED.len()]
}

/// Says to come back after the favor under way.
pub fn later(seed: u32) -> &'static str {
    LATER[mix(seed ^ 0x51ED_270B) as usize % LATER.len()]
}

/// Asks for a ride to `place`: just a lift when it is close, a dream when it is far.
pub fn ride_ask(seed: u32, place: &str, far: bool) -> String {
    let h = mix(seed ^ 0x2C1B_3C6D) as usize;
    if far {
        let asks = [
            "I've always wanted to see {}. Could you fly me there?",
            "They say {} is beautiful this time of year. Would you take me?",
            "I've only ever seen {} on a postcard. Could you fly me there for real?",
            "My grandad used to talk about {}. Could you take me to see it?",
        ];
        asks[h % asks.len()].replace("{}", place)
    } else {
        let asks = [
            "My car's broken down, and I'm meant to be at {}. Could you give me a lift?",
            "I've missed the bus! Could you drop me at {}? It's not far.",
            "My bike's got a flat. Any chance of a lift to {}?",
            "My feet are tired. Could you hop me over to {}?",
            "I'm running late for tea at {}. Could you fly me there quick?",
        ];
        asks[h % asks.len()].replace("{}", place)
    }
}

/// Asks for a lift from beside a broken-down car.
pub fn stranded_ask(seed: u32, place: &str) -> String {
    let asks = [
        "Oh, thank goodness! My car's conked out and I'm meant to be at {}. Could you fly me there?",
        "Steam everywhere, and not a garage for miles. Any chance of a lift to {}?",
        "Flat tire, no spare. Typical! Could you take me to {}? I'll owe you one.",
        "The engine went bang and then went quiet. Could you get me to {}?",
        "I ran out of petrol, would you believe it. Fly me to {}? Please?",
    ];
    asks[mix(seed ^ 0x6A09_E667) as usize % asks.len()].replace("{}", place)
}

/// Asks for a lift from a becalmed sailboat.
pub fn adrift_ask(seed: u32, place: &str) -> String {
    let asks = [
        "Ahoy! Not a breath of wind all morning, and I'm due at {}. Could you fly me there?",
        "The wind's gone and so has my patience. Any chance of a lift to {}?",
        "My rudder snapped clean off! I'll send for the boat later. Could you take me to {}?",
        "I've been drifting in circles for an hour. Fly me to {}? Please?",
        "Oh, a dragon! Just what a becalmed sailor needs. Could you get me to {}?",
    ];
    asks[mix(seed ^ 0xBB67_AE85) as usize % asks.len()].replace("{}", place)
}

/// What drivers say when a dragon lands on their roof.
const DRIVER: &[&str] = &[
    "Whoa! A dragon on the roof! Hold on tight!",
    "Morning! Mind the aerial, it's new.",
    "Well, that's a first. Hello up there!",
    "Ha! My kids will never believe this.",
    "Comfy up there? The sunroof doesn't open, sorry.",
    "Toot toot! That's my horn saying hello.",
    "You're heavier than my mother-in-law. Don't tell her I said that.",
    "I've got the radio on. Shout if you want it louder!",
];

/// What a driver says first.
pub fn driver(seed: u32, nth: u32) -> String {
    DRIVER[mix(seed ^ nth.wrapping_mul(0x9E37_79B9)) as usize % DRIVER.len()].to_string()
}

/// What an animal or someone in a boat says, never one `heard` already while there are
/// others.
pub fn critter(name: &str, nth: u32, heard: &dyn Fn(&str) -> bool) -> String {
    let pool: &[&str] = match name {
        "Cow" => &[
            "Moooo. (It chews slowly and looks you up and down.)",
            "Mooooooo! (It seems pleased to see you.)",
            "Moo. (It goes back to the grass. Very important grass.)",
            "Mmmooo? (It sniffs your nose. Then it sneezes.)",
        ],
        "Sheep" => &[
            "Baaa! (The whole flock looks up at once.)",
            "Baa. (It trots off, then trots back to look again.)",
            "Meeeh. (It is not sure about dragons yet.)",
        ],
        "Seal" => &[
            "Arf arf! (It claps its flippers.)",
            "Arf! (It flops over to show you its tummy.)",
            "Hrrmph. (It goes back to sunbathing.)",
        ],
        "Elephant" => &[
            "PAWOOO! (It waves its trunk at you, very grandly.)",
            "Hrrrmm. (It sprays a little water at your feet. A greeting, probably.)",
            "PAAAWOO! (The keepers say it never forgets a face. Now it knows yours.)",
        ],
        "Giraffe" => &[
            "Mmmm. (The giraffe hums softly and nibbles a leaf from very high up.)",
            "(It bends all the way down to look you in the eye. Then all the way back up.)",
            "Hmmm. (It licks its own nose. Showing off.)",
        ],
        "Zebra" => &[
            "Hee-haw-hee! (It does a little skip.)",
            "Hrrrnh! (It shakes its stripy mane at you.)",
            "Whinny! (It nudges the zebra next to it to look at you too.)",
        ],
        "Lion" => &[
            "ROAAAAR! (Then it yawns and rolls over for a nap.)",
            "Rrrrrr. (It purrs like a very big cat. Because it is one.)",
            "ROAR! (It looks very proud of that one.)",
        ],
        "Mountain goat" => &[
            "Meh-eh-eh! (It hops onto a rock that should be far too small for it.)",
            "Mehhh. (It chews something. You hope it's grass.)",
            "Meh! (It headbutts the air in your direction. Friendly, probably.)",
        ],
        "Penguin" => &[
            "Squawk! (It waddles a little closer. It likes you.)",
            "Squaaawk! (It slides on its belly to show off.)",
            "Squawk squawk! (The others all turn round to see.)",
        ],
        "Kayaker" => &[
            "Ahoy, dragon! The river's quick today!",
            "Paddling all the way to the sea! Want to race? Never mind, you'd win.",
            "Hello! Watch out for the rapids round the bend!",
            "Lovely day for it! My arms will be sore tomorrow.",
        ],
        _ => &[
            "Ahoy! Just rowing round and round. It's very relaxing.",
            "Hello! I'm the only one out here. Well, now there's two of us.",
            "Oh! You gave me a fright. A nice fright!",
            "The fish aren't biting. I don't mind. I just like the boat.",
        ],
    };
    let start = mix(nth) as usize;
    (0..pool.len())
        .map(|k| pool[(start + k) % pool.len()])
        .find(|l| !heard(l))
        .unwrap_or(pool[start % pool.len()])
        .to_string()
}

/// The start, middle and end of lines put together once the written ones run out.
const OPEN: &[&str] = &[
    "Oh, hello!",
    "Well, hi there!",
    "Afternoon!",
    "Ooh, a dragon!",
    "Hello again, island!",
    "Nice to meet you!",
    "Goodness!",
    "Hi!",
    "Oh my!",
    "Hello, hello!",
];
const MIDDLE: &[&str] = &[
    "I've just been to the market for {}.",
    "I'm on my way to fetch {}.",
    "My neighbor lent me {} and I forgot to give it back.",
    "I'm looking for {}. Have you seen one about?",
    "Somebody left {} on the bench in the square.",
    "I won {} at the fair last summer.",
    "My aunt is sending me {} in the post.",
    "I've been saving up for {}.",
    "I dreamed last night about {}.",
    "There's a sale on {} down the road.",
];
const THING: &[&str] = &[
    "a pot of jam",
    "a new kettle",
    "a ball of red wool",
    "a pair of wellies",
    "a tin of biscuits",
    "a bunch of tulips",
    "a cassette of sea shanties",
    "a fishing net",
    "a striped umbrella",
    "a jar of honey",
    "a paper lantern",
    "a pocket radio",
    "a loaf of bread",
    "a teapot shaped like a cat",
    "a bag of marbles",
    "a straw hat",
    "a little telescope",
    "a box of crayons",
    "a watering can",
    "a rubber duck",
];
const CLOSE: &[&str] = &[
    "Anyway, lovely to see you!",
    "Fly safe!",
    "Mind how you go!",
    "See you around!",
    "Don't be a stranger!",
    "Bye for now!",
    "Off I pop!",
    "Have a sunny day!",
    "Come back soon!",
    "Toodle-oo!",
];

/// A line put together from parts, number `k` of a great many, each different.
fn made(k: u32) -> String {
    let k = mix(k);
    let open = OPEN[k as usize % OPEN.len()];
    let k2 = k / OPEN.len() as u32;
    let middle = MIDDLE[k2 as usize % MIDDLE.len()];
    let k3 = k2 / MIDDLE.len() as u32;
    let thing = THING[k3 as usize % THING.len()];
    let k4 = k3 / THING.len() as u32;
    let close = CLOSE[k4 as usize % CLOSE.len()];
    format!("{open} {} {close}", middle.replace("{}", thing))
}

/// What townsperson `seed` says the `nth` time anyone is talked to, never a line
/// `heard` already, so nobody says what someone else said.
pub fn line(seed: u32, nth: u32, scene: &Scene, heard: &dyn Fn(&str) -> bool) -> String {
    for k in 0..40u32 {
        let l = written(seed, nth.wrapping_add(k.wrapping_mul(7919)), scene);
        if !heard(&l) {
            return l;
        }
    }
    // Everything fitting has been said: a written line not yet heard from any pool, then
    // ones put together.
    let all = [ANY, MORNING, EVENING, NIGHT, RAIN, UNDER];
    if let Some(l) = all.iter().flat_map(|p| p.iter()).find(|l| !heard(l)) {
        return l.to_string();
    }
    (0..20_000u32)
        .map(|k| made(seed.wrapping_add(nth).wrapping_add(k)))
        .find(|l| !heard(l))
        .unwrap_or_else(|| made(nth))
}

fn written(seed: u32, nth: u32, scene: &Scene) -> String {
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
        let lines: Vec<String> = (0..40).map(|n| line(7, n, &scene, &|_| false)).collect();
        assert!(lines.iter().all(|l| !l.is_empty() && !l.contains("{}")));
        assert!(lines.iter().any(|l| l.contains("Kettle Light")));
        assert!(lines.iter().any(|l| l.contains("bridges")));
        assert_eq!(name(5), name(5));
    }

    /// However many people are talked to, nobody says what someone already said.
    #[test]
    fn never_the_same_twice() {
        let scene = Scene {
            hour: 13.0,
            rain: false,
            under: true,
            kind: 0,
            rumor: None,
            lair: None,
            dare: None,
            dark: None,
        };
        let mut heard = std::collections::HashSet::new();
        for n in 0..400 {
            let l = line(n * 31 + 7, n, &scene, &|l| heard.contains(l));
            assert!(heard.insert(l.clone()), "said twice: {l}");
        }
    }
}
