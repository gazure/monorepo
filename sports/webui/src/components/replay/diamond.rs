use dioxus::prelude::*;

/// Base-occupancy from the `runners_before` mask: 3 chars, position 0/1/2 =
/// 1B/2B/3B, any non-`-` char = occupied; `None` = bases empty.
fn occupied(runners: Option<&str>) -> [bool; 3] {
    let bytes = runners.unwrap_or("---").as_bytes();
    let on = |i: usize| bytes.get(i).is_some_and(|&b| b != b'-');
    [on(0), on(1), on(2)]
}

fn base_class(on: bool) -> &'static str {
    if on { "replay-base on" } else { "replay-base" }
}

fn runners_label(on: [bool; 3]) -> String {
    let names = ["1st", "2nd", "3rd"];
    let occupied: Vec<&str> = on.iter().zip(names).filter(|(o, _)| **o).map(|(_, n)| n).collect();
    match occupied.len() {
        0 => "bases empty".to_string(),
        1 => format!("runner on {}", occupied[0]),
        _ => format!("runners on {}", occupied.join(", ")),
    }
}

/// Table-cell-sized base-state glyph: three bases, filled when occupied
#[component]
pub fn MiniDiamond(runners: Option<String>) -> Element {
    let bases = occupied(runners.as_deref());
    let [first, second, third] = bases;
    let label = runners_label(bases);

    rsx! {
        span { class: "mini-diamond", title: "{label}",
            svg { view_box: "0 0 24 24",
                rect {
                    class: if second { "mini-base on" } else { "mini-base" },
                    x: 8.5,
                    y: 3.5,
                    width: 7.0,
                    height: 7.0,
                    transform: "rotate(45 12 7)",
                }
                rect {
                    class: if third { "mini-base on" } else { "mini-base" },
                    x: 3.5,
                    y: 8.5,
                    width: 7.0,
                    height: 7.0,
                    transform: "rotate(45 7 12)",
                }
                rect {
                    class: if first { "mini-base on" } else { "mini-base" },
                    x: 13.5,
                    y: 8.5,
                    width: 7.0,
                    height: 7.0,
                    transform: "rotate(45 17 12)",
                }
            }
        }
    }
}

#[component]
pub(super) fn Diamond(runners: Option<String>) -> Element {
    let [first, second, third] = occupied(runners.as_deref());

    rsx! {
        div { class: "replay-diamond",
            svg { view_box: "0 0 120 120",
                polygon { class: "replay-infield", points: "60,32 88,60 60,88 32,60" }
                // Bases: rotated squares at 2B (60,32), 3B (32,60), 1B (88,60)
                rect {
                    class: base_class(second),
                    x: 53.0,
                    y: 25.0,
                    width: 14.0,
                    height: 14.0,
                    transform: "rotate(45 60 32)",
                }
                rect {
                    class: base_class(third),
                    x: 25.0,
                    y: 53.0,
                    width: 14.0,
                    height: 14.0,
                    transform: "rotate(45 32 60)",
                }
                rect {
                    class: base_class(first),
                    x: 81.0,
                    y: 53.0,
                    width: 14.0,
                    height: 14.0,
                    transform: "rotate(45 88 60)",
                }
                polygon { class: "replay-home", points: "54,84 66,84 66,90 60,95 54,90" }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_runner_masks() {
        assert_eq!(occupied(None), [false, false, false]);
        assert_eq!(occupied(Some("1--")), [true, false, false]);
        assert_eq!(occupied(Some("-2-")), [false, true, false]);
        assert_eq!(occupied(Some("123")), [true, true, true]);
        assert_eq!(occupied(Some("1-3")), [true, false, true]);
    }
}
