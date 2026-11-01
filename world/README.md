# World Files

Each world is a folder of KDL v2 files compiled into Rust at build time. `world.kdl` places islands, open ocean finds and anything spanning islands. Each island file holds its own features.

## Conventions

- Meters. +x east, -z north, y up. Sea level is y 0.
- Island features use x/z relative to the island. `world.kdl` uses world x/z. Heights (y, h, top, level) are absolute.
- Every named node has an id that is stable forever (saves, treasure, multiplayer). Ids are unique within an island and referenced across islands as `island/id`.
- `p x z` is a 2D point (outlines, surface paths). `p x y z` is a 3D point (caves, bridges). Outlines and paths are smoothed unless a point has `sharp=#true`.
- Shapes should read like real geography: ragged coasts, inlets, headlands, long uneven ridges. Hand points carry the big shape (every 100 to 300m). `jag=` adds smaller fixed roughness.
- `note "..."` on any node records design intent. It is never shown in game.
- A `y` left off a point, beacon or perch means on the ground.
- `island id file= x= z=` in `world.kdl` places an island. `caves id` groups one cave system. Systems only connect where a passage joins them.
- Caves are always air filled. Mouths open above the waterline.

## Terrain that adds

- `land` outline of an island. Each point sets the coast from there on: `edge=beach|rocky|cliff`. `base` is the default inland height. `jag` is coast roughness in meters. Takes an id when an island has several pieces.
- `ridge id width=` points with `h=`. A mountain range along a line.
- `mountain id x= z= h= r= shape=cone|dome|crag|crater`. Crater takes `rim=` height, `inner=` radius and `floor=`.
- `hill id x= z= h= r=`. Gentle dome.
- Mountains and hills take `stretch=` (long axis over short) and `rot=` degrees.
- `plateau id top= edge=cliff|slope` outline. Clipped to its island's land, so it can run past the coast to make a sea cliff.
- `spire id x= z= h= r= top=flat|point`. Sea stacks and needles.
- `arch id w= thick=` 3D path. Natural arches and land bridges.
- `shelf id y= depth= w=` 2D path along a face. Ledges for cliff towns and perching.
- `pillar id x= z= r=`. Floor to ceiling column inside a cavern.

## Terrain that carves

- `valley id w= depth=` 2D path.
- `gorge id w=` 2D path with `depth=` per point. Narrow, steep sided.
- `bowl id x= z= r= floor=`. Basins for hidden lakes.
- `cavern id x= y= z= w= h= d= rot= shape=hall|dome|shaft|carved`. `y` is the center. `glow=` 0 to 1 base light.
- `passage id w=` 3D path. Tubes linking caverns. 40m wide or more unless marked `tight=#true` (20m min).
- `mouth id kind=cliff|waterfall|sinkhole|chimney|stack|gate w= hidden=#true light=#true` 3D path from a cavern or passage out to open air. `light` streams daylight in.
- `dugout id x= y= z= facing= r=`. Small one room cave in a face.

## Water

- `sea level= floor=` in `world.kdl`. Default sea floor away from land.
- `lake id level=` outline, or `x= z= r=`. Above or below ground.
- `river id w=` path from source to mouth. Surface points follow terrain, cave points need y. Carves its own bed.
- `branch id w=` inside a river. A tributary ends on a point of its parent, a distributary (delta) starts on one.
- `canal id w= level=` path. Cut waterway at a fixed water level, through towns or between waters.
- `falls id h=` inside a river, between the two points where it drops. `h` is the drop.

## Life and places

- `forest id kind=dense|open|rain trees=` outline. `open` is big widely spaced trees to fly between.
- `meadow id` outline.
- `city id style= tech=simple|old|mid|modern` outline. Reserved space, no buildings yet. `tier y=` children for stacked levels.
- `village id` same as city but small.
- `site id kind=` reserved spot for something built later (airport, lighthouse, docks, carving, wreck, etc).
- `road id w=` path. 3D points force a grade, cutting into or building up the ground. `runway id w=` 2D path.
- `route id kind=ferry|yacht|plane|seaplane` path for vehicles later. 2D over sea, 3D in air.
- `beacon id x= y= z=`. Lit once with fire breath, stays lit forever.
- `perch id x= y= z=` and `roost id`. Landing spots and future bird homes.
- `spawn x= z=`.
