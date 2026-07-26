# Baseball

Nine innings, one set of hands. You are the home team: you bat in the bottom of
every inning and pitch in the top, and the game ends when the rules say it does —
including walk-offs and extra innings.

## Running

```bash
cargo run -p baseball
```

Run it through cargo rather than the binary directly; Bevy resolves `assets/`
relative to the crate manifest.

The world inspector is off by default because it draws over the score bug:

```bash
cargo run -p baseball --features debug-inspector
```

## Controls

### Batting

| Keys | Action |
| --- | --- |
| `Space` | Swing |
| `↑` | Lift swing — the only way to hit a home run, helpless up in the zone |
| `↓` | Level swing — keeps the ball on the ground |
| *(neither)* | Normal swing |

### Pitching

| Keys | Action |
| --- | --- |
| `1` `2` `3` `4` | Fastball / slider / curveball / changeup |
| `Q` `E` | Cycle pitch selection |
| `←` `→` `↑` `↓` | Aim, including off the plate |
| `Space` | Throw |

### Anywhere

| Keys | Action |
| --- | --- |
| `Esc` | Pause |
| `T` | Title screen (while paused, or after a game) |
| `R` | Play again (after a game) |
| `Enter` | Start (on the title screen) |

## How a pitch works

The at-bat happens in a view from behind the catcher. The ball leaves the
pitcher's hand and grows as it comes, and a breaking ball tracks toward one spot
and finishes at another. The moment it is struck the camera cuts to the field.

**Nothing about the result is decided in advance.** A swing produces an exit
velocity, a launch angle and a spray angle; the ball then flies under gravity and
drag; the nine fielders run a real pursuit against that trajectory; and the
outcome is whatever the geometry and the footrace produce. That is the difference
from the previous version, where swing timing chose the result outright and the
ball flight was decoration — perfect timing was a guaranteed home run no matter
where the ball went.

Consequences worth knowing:

- **Timing decides how flush the contact is**, and pulls the ball. Being on time
  is no use if the bat is a foot under the ball.
- **Pitch location decides where the ball goes** as much as timing does. Turn on
  an inside pitch and it goes to left field at full exit velocity; without this
  the only way to pull would be to mis-hit it, and every home run would go to
  dead centre.
- **Swing style is a gamble.** Only an uppercut can drive a ball out of the park,
  and an uppercut cannot reach a pitch at the letters at all.
- **Foul balls are not a special case.** The worst-timed contact simply hooks
  outside the foul line and lands there.
- **A ball that only just reaches the wall is not a home run.** It has to clear
  the wall with height to spare; otherwise it is off the wall, which is a
  different outcome.

The resulting mix of outcomes is checked against reality by a test: batting
average on balls in play, the share of hits that go for extra bases, and the share
of balls caught in the air all have to look like baseball.

## Layout

| File | Contents |
| --- | --- |
| `field.rs` | Every position on the field, in feet. The single source of truth |
| `ball.rs` | Ball flight, and the fielder pursuit run against it |
| `pitch.rs` | Pitch types, break, the strike zone, and the umpire |
| `bat.rs` | Swings, and the contact they produce |
| `fielding.rs` | Turning a trajectory into a result the rules engine understands |
| `flow.rs` | The pitch loop, and the AI on both sides of it |
| `view.rs` | The two cameras and the render layers that keep them apart |
| `scene.rs` | Drawing the ballpark, the players and the ball |
| `hud.rs` | The score bug and the pitch panel |
| `screens.rs` | Title, pause, inning card, and the box score |
| `effects.rs` | Dust, screen shake |
| `theme.rs` | Palette |

`rules/` is a separate crate holding the rules of baseball, with no dependency on
Bevy. It is built out of immutable value types — `Count`, `PlateAppearance`,
`HalfInning`, `Game` — that each expose `advance(outcome)` and hand back a result
enum, so illegal states are mostly unrepresentable.

### Two things about the design

**The field has one coordinate system.** Everything in `field.rs` is in feet with
home plate at the origin, and the field camera renders one world unit per foot, so
those numbers *are* the world coordinates. Previously the bases were drawn as
children of a 45°-rotated square while the nine fielders were positioned in
unrotated world space; the two disagreed and nothing in the code could tell you
which was right.

**A play is named, not described.** `PitchOutcome::InPlay` carries a `PlayResult`
— `Groundout`, `Double`, `SacrificeFly` — and `HalfInning` resolves it against the
baserunners it already owns. Callers used to build the end state themselves and
pass the runners in, which is a bug factory: `PlayOutcome::groundout()` was a
constant that hardcoded every base to empty, so **any ground ball cleared the
bases**.
