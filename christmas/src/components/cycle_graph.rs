//! The giving ring: a cycle drawn as names around a circle, joined by arcs that
//! point from giver to receiver.
//!
//! Layout maths lives in free functions so it can be unit tested without a
//! renderer.

use std::f64::consts::{FRAC_PI_2, TAU};

use dioxus::prelude::*;

const VIEW: f64 = 400.0;
const CENTRE: f64 = VIEW / 2.0;
const RADIUS: f64 = 128.0;
const LABEL_RADIUS: f64 = RADIUS + 18.0;
/// Keeps arc ends clear of the node dots and their arrowheads.
const TRIM: f64 = 11.0;
/// How far arcs bow toward the centre. 0 is a straight chord.
const BOW: f64 = 0.16;

/// The single sanctioned precision cast — cycle sizes are tiny.
#[expect(clippy::cast_precision_loss, reason = "cycle lengths are far below 2^53")]
fn f(n: usize) -> f64 {
    n as f64
}

/// Angle of node `i` of `n`, starting at twelve o'clock and running clockwise.
fn angle(i: usize, n: usize) -> f64 {
    TAU * f(i) / f(n) - FRAC_PI_2
}

/// Evenly spaced points around the ring.
pub fn ring_points(n: usize, radius: f64) -> Vec<(f64, f64)> {
    (0..n)
        .map(|i| {
            let a = angle(i, n);
            (CENTRE + radius * a.cos(), CENTRE + radius * a.sin())
        })
        .collect()
}

/// Pulls both ends of a segment inward by `by`, so arcs stop short of the dots.
pub fn trim(p1: (f64, f64), p2: (f64, f64), by: f64) -> ((f64, f64), (f64, f64)) {
    let (dx, dy) = (p2.0 - p1.0, p2.1 - p1.1);
    let len = dx.hypot(dy);
    if len <= by * 2.0 {
        return (p1, p2);
    }
    let (ux, uy) = (dx / len, dy / len);
    ((p1.0 + ux * by, p1.1 + uy * by), (p2.0 - ux * by, p2.1 - uy * by))
}

/// Quadratic control point, offset toward the centre to bow the arc inward.
pub fn control(p1: (f64, f64), p2: (f64, f64), bow: f64) -> (f64, f64) {
    let mid = (f64::midpoint(p1.0, p2.0), f64::midpoint(p1.1, p2.1));
    (mid.0 + (CENTRE - mid.0) * bow, mid.1 + (CENTRE - mid.1) * bow)
}

/// Which side of the ring a label sits on, so text grows away from the circle.
pub fn label_anchor(x: f64) -> &'static str {
    let dx = x - CENTRE;
    if dx > 6.0 {
        "start"
    } else if dx < -6.0 {
        "end"
    } else {
        "middle"
    }
}

/// One ring. `index` only has to be unique within the page so the arrowhead
/// marker ids do not collide between rings.
#[component]
pub fn CycleRing(cycle: Vec<String>, index: usize, letter: Option<char>, show_letter: bool) -> Element {
    let mut hovered = use_signal(|| None::<usize>);

    let n = cycle.len();
    if n == 0 {
        return rsx! {};
    }

    let nodes = ring_points(n, RADIUS);
    let labels = ring_points(n, LABEL_RADIUS);
    let marker = format!("arrow-{index}");
    let active = hovered();

    // Caption narrates whatever the pointer is on, so the ring stays readable
    // without labelling every arc.
    let caption = active.map(|i| (cycle[i].clone(), cycle[(i + 1) % n].clone()));

    rsx! {
        svg {
            class: "ring",
            view_box: "0 0 {VIEW} {VIEW}",
            role: "img",
            "aria-label": "Giving ring of {n} participants",

            // Two markers per ring rather than one with `currentColor`: inside
            // `<defs>` currentColor resolves against the marker's own context,
            // not the path referencing it, which renders every arrowhead white.
            defs {
                marker {
                    id: "{marker}",
                    view_box: "0 0 10 10",
                    ref_x: "8",
                    ref_y: "5",
                    marker_width: "5",
                    marker_height: "5",
                    orient: "auto-start-reverse",
                    path { d: "M 0 1 L 9 5 L 0 9 z", class: "ring-arrow" }
                }
                marker {
                    id: "{marker}-lit",
                    view_box: "0 0 10 10",
                    ref_x: "8",
                    ref_y: "5",
                    marker_width: "5",
                    marker_height: "5",
                    orient: "auto-start-reverse",
                    path { d: "M 0 1 L 9 5 L 0 9 z", class: "ring-arrow lit" }
                }
            }

            // Arcs first so the dots sit on top of them.
            for i in 0..n {
                {
                    let from = nodes[i];
                    let to = nodes[(i + 1) % n];
                    let (a, b) = trim(from, to, TRIM);
                    let c = control(a, b, BOW);
                    let lit_edge = active.is_some_and(|h| h == i);
                    let class = match active {
                        Some(h) if h == i => "ring-edge lit",
                        Some(_) => "ring-edge dimmed",
                        None => "ring-edge",
                    };
                    let arrow = if lit_edge {
                        format!("url(#{marker}-lit)")
                    } else {
                        format!("url(#{marker})")
                    };
                    // Stagger the draw-in so the ring assembles rather than flashing.
                    let delay = f(i) * 0.05;
                    rsx! {
                        path {
                            key: "e{i}",
                            class: "{class}",
                            style: "animation-delay: {delay}s",
                            marker_end: "{arrow}",
                            d: "M {a.0:.2} {a.1:.2} Q {c.0:.2} {c.1:.2} {b.0:.2} {b.1:.2}",
                        }
                    }
                }
            }

            if show_letter {
                if let Some(ch) = letter {
                    text { class: "ring-centre-letter", x: "{CENTRE}", y: "{CENTRE}", "{ch}" }
                }
            }

            for i in 0..n {
                {
                    let (nx, ny) = nodes[i];
                    let (lx, ly) = labels[i];
                    let receiving = active.is_some_and(|h| (h + 1) % n == i);
                    let lit = active.is_some_and(|h| h == i);
                    let node_class = if lit {
                        "ring-node lit"
                    } else if receiving {
                        "ring-node receiving"
                    } else {
                        "ring-node"
                    };
                    let label_class = if lit {
                        "ring-label lit"
                    } else if receiving {
                        "ring-label receiving"
                    } else if active.is_some() {
                        "ring-label dimmed"
                    } else {
                        "ring-label"
                    };
                    let name = cycle[i].clone();
                    rsx! {
                        g {
                            key: "n{i}",
                            onmouseenter: move |_| hovered.set(Some(i)),
                            onmouseleave: move |_| hovered.set(None),
                            circle {
                                class: "{node_class}",
                                cx: "{nx:.2}",
                                cy: "{ny:.2}",
                                r: if lit || receiving { "6.5" } else { "4.5" },
                            }
                            text {
                                class: "{label_class}",
                                x: "{lx:.2}",
                                y: "{ly:.2}",
                                text_anchor: label_anchor(lx),
                                dominant_baseline: "middle",
                                "{name}"
                            }
                        }
                    }
                }
            }
        }

        div { class: "ring-caption",
            match caption {
                Some((giver, receiver)) => rsx! {
                    strong { "{giver}" }
                    span { class: "arrow", "gives to" }
                    strong { "{receiver}" }
                },
                None => rsx! { span { class: "muted", "Hover a name to follow the chain" } },
            }
        }
    }
}

/// All the rings of one draw, each in its own card.
#[component]
pub fn CycleBoard(cycles: Vec<Vec<String>>, letter: Option<char>) -> Element {
    let single = cycles.len() == 1;

    rsx! {
        div { class: "ring-grid",
            for (i, cycle) in cycles.into_iter().enumerate() {
                div { key: "ring{i}", class: "ring-card",
                    if !single {
                        div { class: "ring-card-head",
                            h3 { "Ring {i + 1}" }
                            span { class: "count", "{cycle.len()} people" }
                        }
                    }
                    CycleRing {
                        cycle: cycle.clone(),
                        index: i,
                        letter,
                        // Only centre the letter when there is one ring to centre it in.
                        show_letter: single,
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ring_points_are_evenly_spaced_on_the_circle() {
        for n in [3usize, 5, 10, 14] {
            let pts = ring_points(n, RADIUS);
            assert_eq!(pts.len(), n);
            for (x, y) in pts {
                let d = (x - CENTRE).hypot(y - CENTRE);
                assert!((d - RADIUS).abs() < 1e-9, "point off the circle: {d}");
            }
        }
    }

    #[test]
    fn first_node_sits_at_twelve_oclock() {
        let pts = ring_points(4, RADIUS);
        assert!((pts[0].0 - CENTRE).abs() < 1e-9);
        assert!(pts[0].1 < CENTRE, "first node should be above centre");
    }

    #[test]
    fn trim_pulls_both_ends_inward() {
        let (a, b) = trim((0.0, 0.0), (100.0, 0.0), 10.0);
        assert!((a.0 - 10.0).abs() < 1e-9);
        assert!((b.0 - 90.0).abs() < 1e-9);
    }

    #[test]
    fn trim_leaves_degenerate_segments_alone() {
        // Two nodes closer together than the trim would consume.
        let p1 = (0.0, 0.0);
        let p2 = (5.0, 0.0);
        assert_eq!(trim(p1, p2, 10.0), (p1, p2));
    }

    #[test]
    fn control_point_bows_toward_the_centre() {
        let p1 = (CENTRE - 100.0, CENTRE);
        let p2 = (CENTRE + 100.0, CENTRE);
        let c = control(p1, p2, BOW);
        // Midpoint is already the centre here, so it should not move.
        assert!((c.0 - CENTRE).abs() < 1e-9);

        let p1 = (CENTRE, CENTRE - 100.0);
        let p2 = (CENTRE + 100.0, CENTRE);
        let c = control(p1, p2, BOW);
        let mid = (f64::midpoint(p1.0, p2.0), f64::midpoint(p1.1, p2.1));
        let before = (mid.0 - CENTRE).hypot(mid.1 - CENTRE);
        let after = (c.0 - CENTRE).hypot(c.1 - CENTRE);
        assert!(after < before, "control point should move toward the centre");
    }

    #[test]
    fn labels_anchor_away_from_the_ring() {
        assert_eq!(label_anchor(CENTRE + 100.0), "start");
        assert_eq!(label_anchor(CENTRE - 100.0), "end");
        assert_eq!(label_anchor(CENTRE), "middle");
    }

    #[test]
    fn nobody_is_named_before_the_reading_starts() {
        for i in 0..5 {
            assert!(!is_named(i, 0), "position {i} should be empty at the start");
        }
    }

    #[test]
    fn the_first_beat_names_the_giver_and_their_receiver() {
        // A→B→C→D: reading "A gives to B" names exactly A and B.
        assert!(is_named(0, 1));
        assert!(is_named(1, 1));
        assert!(!is_named(2, 1));
        assert!(!is_named(3, 1));
    }

    #[test]
    fn each_later_beat_names_exactly_one_more_person() {
        for revealed in 1..6 {
            let named = (0..6).filter(|i| is_named(*i, revealed)).count();
            assert_eq!(
                named,
                revealed + 1,
                "beat {revealed} should have named {} people",
                revealed + 1
            );
        }
    }

    #[test]
    fn the_last_beat_leaves_nobody_hidden() {
        // The closing edge points back at position 0, who was named first.
        let n = 4;
        assert!((0..n).all(|i| is_named(i, n)), "everyone is named once the ring closes");
    }

    #[test]
    fn a_name_arrives_on_exactly_one_beat() {
        for i in 0..6 {
            let arrivals: Vec<usize> = (0..8).filter(|r| is_arriving(i, *r)).collect();
            assert_eq!(arrivals.len(), 1, "position {i} arrived on {arrivals:?}");
            // And it arrives on the same beat it first becomes visible.
            let first_named = (0..8).find(|r| is_named(i, *r)).expect("named eventually");
            assert_eq!(arrivals[0], first_named);
        }
    }
}

/// Whether position `i` of the ring has been named yet, given how much of the
/// draw has been read out.
///
/// Reading out edge `e` names two people: `cycle[e]` as the giver and
/// `cycle[e + 1]` as the receiver. So after `revealed` edges, positions `0`
/// through `revealed` have all been said out loud, and nobody else has. The
/// first position is a special case only in that it arrives with the very first
/// edge rather than as somebody's receiver.
pub fn is_named(i: usize, revealed: usize) -> bool {
    revealed > 0 && i <= revealed
}

/// Whether position `i` is being named for the first time on this exact beat,
/// so it can be given an entrance rather than simply appearing.
pub fn is_arriving(i: usize, revealed: usize) -> bool {
    if i == 0 { revealed == 1 } else { revealed == i }
}

/// One ring mid-ceremony: edges appear one at a time as the draw is read out.
///
/// Names stay off the board until they are called. Showing the whole roster up
/// front and merely dimming it gave the ending away — you could read who was
/// left and work out the last few pairings before they were announced.
///
/// Separate from [`CycleRing`] because the interaction is different — nothing
/// responds to the pointer, and the state is "how much has been revealed".
#[component]
pub fn RevealRing(cycle: Vec<String>, revealed: usize, letter: Option<char>, show_letter: bool) -> Element {
    let n = cycle.len();
    if n == 0 {
        return rsx! {};
    }

    let nodes = ring_points(n, RADIUS);
    let labels = ring_points(n, LABEL_RADIUS);
    // The giver of the edge currently being drawn.
    let active = revealed.checked_sub(1).filter(|_| revealed <= n);

    rsx! {
        svg {
            class: "ring reveal-ring",
            view_box: "0 0 {VIEW} {VIEW}",
            role: "img",
            "aria-label": "Draw in progress",

            defs {
                marker {
                    id: "reveal-arrow",
                    view_box: "0 0 10 10",
                    ref_x: "8",
                    ref_y: "5",
                    marker_width: "5",
                    marker_height: "5",
                    orient: "auto-start-reverse",
                    path { d: "M 0 1 L 9 5 L 0 9 z", class: "ring-arrow lit" }
                }
            }

            for i in 0..n.min(revealed) {
                {
                    let (a, b) = trim(nodes[i], nodes[(i + 1) % n], TRIM);
                    let c = control(a, b, BOW);
                    let newest = active == Some(i);
                    let class = if newest { "ring-edge lit newest" } else { "ring-edge lit settled" };
                    rsx! {
                        path {
                            key: "re{i}",
                            class: "{class}",
                            marker_end: "url(#reveal-arrow)",
                            d: "M {a.0:.2} {a.1:.2} Q {c.0:.2} {c.1:.2} {b.0:.2} {b.1:.2}",
                        }
                    }
                }
            }

            if show_letter {
                if let Some(ch) = letter {
                    text { class: "ring-centre-letter", x: "{CENTRE}", y: "{CENTRE}", "{ch}" }
                }
            }

            for i in 0..n {
                {
                    let (nx, ny) = nodes[i];
                    let (lx, ly) = labels[i];
                    let giving = active == Some(i);
                    let receiving = active.is_some_and(|a| (a + 1) % n == i);
                    // Anyone whose turn has passed stays lit, so the ring fills in.
                    let done = i < revealed;
                    let named = is_named(i, revealed);
                    let node_class = if giving {
                        "ring-node lit"
                    } else if receiving {
                        "ring-node receiving"
                    } else if done {
                        "ring-node"
                    } else {
                        "ring-node waiting"
                    };
                    let label_class = if giving {
                        "ring-label lit"
                    } else if receiving {
                        "ring-label receiving"
                    } else {
                        "ring-label"
                    };
                    // Re-applied on the beat a name arrives, which is what
                    // restarts the entrance; every later beat drops it again.
                    let label_class = if is_arriving(i, revealed) {
                        format!("{label_class} arriving")
                    } else {
                        label_class.to_string()
                    };
                    let name = cycle[i].clone();
                    rsx! {
                        g { key: "rn{i}",
                            circle {
                                class: "{node_class}",
                                cx: "{nx:.2}",
                                cy: "{ny:.2}",
                                // An empty seat is a smaller mark than a taken
                                // one: the ring's shape reads without saying
                                // who is standing where.
                                r: if giving || receiving {
                                    "7"
                                } else if named {
                                    "4.5"
                                } else {
                                    "3"
                                },
                            }
                            if named {
                                text {
                                    class: "{label_class}",
                                    x: "{lx:.2}",
                                    y: "{ly:.2}",
                                    text_anchor: label_anchor(lx),
                                    dominant_baseline: "middle",
                                    "{name}"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
