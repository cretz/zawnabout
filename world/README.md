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
- `ridge id width= smooth=#true` points with `h=`. A mountain range along a line. `smooth` leaves off the crags, for even slopes such as ski runs.
- `mountain id x= z= h= r= shape=cone|dome|crag|crater`. Crater takes `rim=` height, `inner=` radius and `floor=`.
- `hill id x= z= h= r=`. Gentle dome.
- Mountains and hills take `stretch=` (long axis over short) and `rot=` degrees.
- `plateau id top= edge=cliff|slope` outline. Clipped to its island's land, so it can run past the coast to make a sea cliff.
- `spire id x= z= h= r= top=flat|point`. Sea stacks and needles.
- `arch id w= thick=` 3D path. Natural arches and land bridges.
- `pillar id x= z= r=`. Floor to ceiling column inside a cavern.

## Terrain that carves

- `valley id w= depth=` 2D path.
- `gorge id w=` 2D path with `depth=` per point. Narrow, steep sided.
- `bowl id x= z= r= floor=`. Basins for hidden lakes.
- `cavern id x= y= z= w= h= d= rot= shape=hall|dome|shaft|carved`. `y` is the center. `glow=` 0 to 1 base light. Child `spout id x= z= w=`: water pouring from a crack in the roof down to the floor or the cave lake.
- `passage id w= decor=bare|tiled glow=` 3D path (a `mouth` takes `decor` too). Tubes linking caverns. 40m wide or more unless marked `tight=#true` (20m min). Road and rail tunnels take `decor=tiled` and a higher `glow`, and get lamps along the road.
- `mouth id kind=cliff|waterfall|sinkhole|chimney|stack|gate w= hidden=#true light=#true` 3D path from a cavern or passage out to open air. `light` streams daylight in.
- `dugout id x= y= z= facing= r=`. Small one room cave in a face.

## Water

- `sea level= floor=` in `world.kdl`. Default sea floor away from land.
- `lake id level=` outline, or `x= z= r=`. Above or below ground. A cave lake sits in a basin carved into the cavern floor, so set its level a few meters under the floor.
- `river id w=` path from source to mouth. Its water level is measured from the ground and never rises downstream, so steep drops become falls on their own. Surface rivers carve their bed (deep cuts widen into gorges), cave river points need y and cut a slot along the cave floor.
- `branch id w=` inside a river. A tributary ends on a point of its parent, a distributary (delta) starts on one.
- `canal id w= level=` path. Cut waterway at a fixed water level with stone quays, through towns or between waters.
- `falls id h=` inside a river, between the two points where it drops. Names the falls (a mark) and records the intended drop. Make sure the ground really drops there.

## Life and places

- `forest id kind=dense|open|rain trees=` outline. `open` is big widely spaced trees to fly between.
- `meadow id` outline.
- `snowfield id` outline. Snow on the ground, for ski runs. A few pines, skiers on it near lifts.
- `city id style= tech=simple|old|mid|modern` outline, filled with buildings laid out from the file. `style` is `plain`, `canal`, `hill`, `cliff`, `stone`, `seafaring`, `modern`, `cavern` or `stilts`. Buildings line canals, rivers, roads, shores, terrace edges and ledges, then fill the rest (terraced streets in old and busy towns, loose plots in villages). `tier y=` children step the ground into terraces. Cave towns take `y=`, the floor height.
- `village id` same as city but small.
- `site id kind=` a landmark: `lighthouse`, `tower` (TV tower), `docks`, `ferry`, `marina`, `seaplane`, `shipyard`, `airport` (around its runway), `wreck`. Others (`bridge`, `carving`) are reserved for later. Lighthouses and towers are perches named by the site id.
- `road id w=` path. Roads 14 or wider are highways: edge lines, a double center line and barriers. Roads should join others; one that ends inside a town ends in a paved square. 3D points force a grade, 2D points get a smooth grade from the ground, never steeper than 9% (3% for railways): where the ground climbs faster the road cuts in, where it dips fast it crosses on a viaduct. Roads take the height of wider roads where they join them. Plan routes up steep ground with `working-area/scratch/scripts/roadplan.py`, and check every road with the `road_audit` probe. Mix them to hold just a stretch, like a tunnel through a passage: heights run straight between held points. Bridges go up wherever a road crosses water. `runway id w=` 2D path, dead straight and flat.
- `railway id [subway=#true] [tour=#true] [mine=#true]` path, 2D or 3D, a loop when it ends where it starts. Subways run metro trains and get underground stations (give each a carved hall). Tour lines run one little open train through caverns (`mine=#true` makes it an engine pulling ore carts of miners), on trestles where the floor drops away (lay them with `scratch/scripts/caveline.py`). Track, two trains going round, and `station id x= z=` children snapped beside the track on that side. Inside a passage it runs in the tunnel, leaving the hill overhead.
- `shelf id y= depth= w=` path along a face. Sets the ground to `y` in a band `w` wide, so stacked shelves make a stepped cliff for cliff towns.
- `route id kind=ferry|cruise|yacht|plane|seaplane|balloon|chairlift|gondola|punt|trawler|power|heli` path for vehicles. Punts are cave tour boats: give 3D points at the water level. `power` is a power line: pylons along it and cables between them, no vehicles. `heli` is a helicopter's loop in 3D points: it sets down and waits on every helipad and hospital roof it passes over. Lifts take two 2D points, bottom then top: pylons, stations and cable follow the ground, and the chairs or cabins can be landed on and ridden. 2D over sea, 3D in air. Closed paths loop, open ones go out and back.
- `landmark id kind= x z [y] [rot] [text]` one odd, solid thing: `stones`, `turbine`, `campsite`, `picnic`, `phonebox`, `umbrellas`, `scarecrow`, `telescope`, `drivein`, `sign` (with `text`), `ferris`, `swing`, `maze`, `sandcastle`, `snowman`, `bench` (give `y` to float it), `gnomes`, `beehives`, `pumpkins`, `busstop`, `watertower`, `hotspring`, `chess`, `cairns`, `treehouse`, `well`, `lemonade`, `igloo`, `laundry`, `sundial`, `circus`, `fountain`, `postbox`, `bandstand`, `stadium`, `waterpark`, `themepark`, `zoo`, `statue`, `mall`, `grocery`, `park`, `station`, `metro-gate` (a subway entrance at the top of a stair tunnel, its stairs going down toward -z: canopy, railings round the cutting, M signs), `metro-stairs` (the stairs down the tunnel from a hall floor up toward +z, `text="LEN DROP"`, give `y`; lay the tunnel as a sloped `mouth` with `decor=tiled`), `span-tower` (an office block over a road: set it on the road with `rot` so its x runs across it), `market` (stalls round a square, with shoppers), `library`, `museum`, `cinema` (lit at night, a queue outside), `weir`, `penstock` (`text="LEN DROP"`, pipes down a slope along x), `hydro` (power house), `racetrack` (race cars lap it, land on one), `police`, `gasstation`, `golf` (golfers play the holes), `solarfarm`, `amphitheater` (a show with an audience), `aquarium`, `university` (students on the quad), `shroomfarm`, `mine`, `cafe` (cave places, give `y` to stand them on a cavern floor), `observatory`, `lookout` (fire lookout tower), `surfshack`, `ropebridge` (from the landmark along its x: `text="LENGTH RISE"`, give `y`), `hospital` (helipad on the roof), `firestation`, `school`, `townhall`, `farm`, `port` (a container quay: its front, +z, faces the water), `restaurant` (a mountain restaurant by a lift top, deck out toward +z over the drop, give `y`), `drawbridge` (a footbridge along x over water, `text="LEN"`, give `y=0`: its middle lifts as boats come by), `gallery` (a walkway round a carved hall's walls with stairs up, `text="HALF_LENGTH HALF_WIDTH"`, rot to match the hall), `skatepark`, `helipad`, `clocktower` (a town clock on a little square; its hands, like those on `townhall` and `university`, keep the game's time).
- `beacon id x= y= z=`. Lit once with fire breath, stays lit forever.
- `perch id x= y= z=` and `roost id`. Landing spots and future bird homes.
- `spawn x= z=`.
