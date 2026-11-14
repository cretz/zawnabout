# zawnabout-game

The client on top of the sim: input, camera, meshes, life (people, animals, weather) and wgpu rendering. The native and web hosts drive it, and it runs debug scripts and playbooks.

Its journal (`book.rs`) is also the pause screen: hoard (with pictures of every treasure, drawn once by `thumbs.rs`), places, people, flying (stunts, courses, races), stories (start or set aside a campaign), a map that shows where things are (Show me), help, and settings with the keys. A save keeps where the game was left, to carry on there. Gameplay from `zawnabout-play` comes in through `play.rs` behind the `play` feature.
