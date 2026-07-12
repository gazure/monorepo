use dioxus::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PitchKind {
    Ball,
    Strike,
    Foul,
    InPlay,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Pitch {
    pub code: char,
    pub label: &'static str,
    pub kind: PitchKind,
}

/// Parse a `bbref` pitch-sequence string ("CBX", "BSBFB") into chips.
/// Annotation characters — `*` (pickoff), `>` (runner going), `.` separators,
/// digits — are skipped.
pub(super) fn parse_pitches(seq: &str) -> Vec<Pitch> {
    seq.chars()
        .filter_map(|c| {
            let (label, kind) = match c {
                'B' => ("ball", PitchKind::Ball),
                'I' => ("intentional ball", PitchKind::Ball),
                'P' => ("pitchout", PitchKind::Ball),
                'V' => ("automatic ball", PitchKind::Ball),
                'C' => ("called strike", PitchKind::Strike),
                'S' => ("swinging strike", PitchKind::Strike),
                'T' => ("foul tip", PitchKind::Strike),
                'M' => ("missed bunt", PitchKind::Strike),
                'F' => ("foul", PitchKind::Foul),
                'L' => ("foul bunt", PitchKind::Foul),
                'X' => ("in play", PitchKind::InPlay),
                'H' => ("hit by pitch", PitchKind::InPlay),
                _ => return None,
            };
            Some(Pitch { code: c, label, kind })
        })
        .collect()
}

/// One letter-carrying chip per pitch; identity is never color-alone.
#[component]
pub(super) fn PitchChips(sequence: Option<String>) -> Element {
    let pitches = sequence.as_deref().map(parse_pitches).unwrap_or_default();
    if pitches.is_empty() {
        return rsx! {};
    }
    rsx! {
        div { class: "replay-chips",
            for (i , p) in pitches.iter().enumerate() {
                span {
                    key: "{i}",
                    class: match p.kind {
                        PitchKind::Ball => "replay-chip chip-ball",
                        PitchKind::Strike => "replay-chip chip-strike",
                        PitchKind::Foul => "replay-chip chip-foul",
                        PitchKind::InPlay => "replay-chip chip-inplay",
                    },
                    title: "{p.label}",
                    "{p.code}"
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_sequence() {
        let pitches = parse_pitches("CBX");
        assert_eq!(
            pitches.iter().map(|p| p.kind).collect::<Vec<_>>(),
            vec![PitchKind::Strike, PitchKind::Ball, PitchKind::InPlay]
        );
    }

    #[test]
    fn parses_full_count_walk() {
        let pitches = parse_pitches("BSBFB");
        assert_eq!(pitches.len(), 5);
        assert_eq!(pitches.iter().filter(|p| p.kind == PitchKind::Ball).count(), 3);
    }

    #[test]
    fn skips_annotations() {
        // Pickoffs (*N), runner-going (>), and separators (.) are not pitches
        let pitches = parse_pitches("*>B.2FSS");
        assert_eq!(
            pitches.iter().map(|p| p.code).collect::<Vec<_>>(),
            vec!['B', 'F', 'S', 'S']
        );
    }

    #[test]
    fn empty_sequence_yields_no_chips() {
        assert!(parse_pitches("").is_empty());
        assert!(parse_pitches("*>.123").is_empty());
    }
}
