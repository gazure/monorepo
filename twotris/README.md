# Twotris

It's tetris with two grids, lmao

Two playfields fall at once and you only have one set of hands. `F` swaps which
board takes your input; the board you are *not* looking at keeps falling at half
speed. Score, level and lines are shared across both boards, so neglecting one
costs you the whole run.

## Running

```bash
cargo run -p twotris
```

Run it through cargo rather than invoking the binary directly — Bevy resolves the
`assets/` directory relative to the crate manifest.

## Controls

| Keys | Action |
| --- | --- |
| `←` `→` | Move (hold to auto-repeat) |
| `↓` | Soft drop (1 point per row) |
| `Space` | Hard drop (2 points per row) |
| `Z` / `X` `↑` | Rotate counter-clockwise / clockwise |
| `C` | Hold |
| `F` `Tab` | Swap which board has focus |
| `Esc` | Pause |
| `R` | Restart (while paused) |
| `T` | Back to the title screen |

## Rules

- 7-bag randomiser, so no shape droughts longer than two bags.
- Rotation wall-kicks off the walls and the floor.
- A grounded piece waits out a 0.5s lock delay; moving or rotating refreshes it,
  up to 15 times, so you cannot stall forever.
- Guideline scoring: 100/300/500/800 per 1/2/3/4 rows, times the level. Combos
  add 50 × combo × level, and a tetris straight after another tetris pays 1.5×.
- Gravity speeds up every 10 lines, up to level 20.

## Layout

| File | Contents |
| --- | --- |
| `piece.rs` | Shapes as const rotation tables, plus the 7-bag |
| `board.rs` | The playfield of locked cells; collision, kicks, row clearing |
| `game.rs` | Input, gravity, locking, scoring |
| `render.rs` | Arena construction and per-frame board drawing |
| `ui.rs` | Score bug, side panels, and the full-screen overlays |
| `effects.rs` | Confetti, flashes, score popups, screen shake |
| `theme.rs` | Palette and cell sizing |

The falling piece is never written into the board — it lives on the board entity
as an `Active` component and is composited at draw time, so collision queries
never have to reason about a piece colliding with itself.
