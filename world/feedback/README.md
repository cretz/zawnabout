# Design Notes

Left in the game's notes mode (`N` or the pencil button) while running `pnpm dev`. One JSON file per note:

- `text`: the note.
- `spot`: x, y, z of the pinned surface.
- `view`: camera x, y, z, yaw, pitch. Replay it with the `view` command.
- `near`: named places at and around the spot.
- `status`: `open` or `done`. `replies`: `{ "from", "text", "at" }` entries, shown in the game's note list.
