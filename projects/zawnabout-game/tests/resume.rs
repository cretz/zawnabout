//! A save carries on where the game was left.

#![cfg(feature = "play")]

use glam::Vec3;
use zawnabout_game::Game;
use zawnabout_game::book::{Card, CardKind};

/// The dragon's spot, the time of day and an unread card all come back from a save.
#[test]
fn carries_on_where_it_was_left() {
    let mut a = Game::new(1);
    a.spawn_player(Vec3::new(-1500.0, 0.0, 1200.0), 1.2, 0.0);
    a.time = 4321.5;
    a.hud.book.cards.push(Card {
        title: "A find".into(),
        body: "Something | shiny".into(),
        foot: String::new(),
        kind: CardKind::Find,
    });
    let text = a.save_text().expect("gameplay is on");
    assert!(text.contains("[where]"));
    let mut b = Game::new(1);
    b.load_save(&text).expect("the save loads");
    let (pa, pb) = (a.player_dragon().pos, b.player_dragon().pos);
    assert!(pa.distance(pb) < 0.1, "{pa} came back as {pb}");
    assert!((a.player_dragon().heading - b.player_dragon().heading).abs() < 0.01);
    assert_eq!(b.time, 4321.5);
    assert_eq!(b.hud.book.cards.len(), 1);
    assert_eq!(b.hud.book.cards[0].title, "A find");
    // Saving again keeps one place, not two.
    assert_eq!(b.save_text().unwrap().matches("[where]").count(), 1);
}
