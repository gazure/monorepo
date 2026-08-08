//! Wire types shared between the server functions and the UI.

use serde::{Deserialize, Serialize};

pub use crate::matching::{CycleMode, DEFAULT_MIN_CYCLE_LEN, DrawConfig, SwapChange, SwapViolation};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Participant {
    pub id: i32,
    pub name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationshipKind {
    Spouse,
    Household,
    Manual,
}

impl RelationshipKind {
    pub const ALL: [Self; 3] = [Self::Spouse, Self::Household, Self::Manual];

    pub fn as_db(self) -> &'static str {
        match self {
            Self::Spouse => "spouse",
            Self::Household => "household",
            Self::Manual => "manual",
        }
    }

    pub fn from_db(s: &str) -> Self {
        match s {
            "spouse" => Self::Spouse,
            "household" => Self::Household,
            _ => Self::Manual,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Spouse => "Spouse",
            Self::Household => "Household",
            Self::Manual => "Manual",
        }
    }

    /// Whether this relationship is suppressed by the "no spouses" toggle.
    pub fn is_partnership(self) -> bool {
        matches!(self, Self::Spouse | Self::Household)
    }
}

/// A symmetric constraint between two participants.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Relationship {
    pub id: i32,
    pub participant_a: Participant,
    pub participant_b: Participant,
    pub kind: RelationshipKind,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Pool {
    pub id: i32,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub sort_order: i32,
    pub member_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Pairing {
    pub giver: String,
    pub receiver: String,
}

/// Archived form of a constraint, stored inside the exchange snapshot.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExclusionSnapshot {
    pub a: String,
    pub b: String,
    pub reason: Option<String>,
}

/// A recorded draw. Immutable once written — re-drawing creates a new revision.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Exchange {
    pub id: i32,
    pub pool_id: i32,
    pub pool_name: String,
    pub pool_slug: String,
    pub year: i32,
    pub revision: i32,
    pub letter: Option<char>,
    pub participants: Vec<String>,
    pub exclusions: Vec<ExclusionSnapshot>,
    pub pairings: Vec<Pairing>,
    pub config: Option<DrawConfig>,
    pub seed: Option<i64>,
    /// The revision this one was hand-edited from, if it was.
    pub adjusted_from: Option<i32>,
    /// What that edit was, in words.
    pub adjustment_note: Option<String>,
}

/// Keeps only the live draw for each pool and year.
///
/// Expects the newest revision of each to come first, which is how
/// `load_exchanges` orders them.
pub fn only_current_revisions(exchanges: Vec<Exchange>) -> Vec<Exchange> {
    let mut seen = std::collections::HashSet::new();
    exchanges
        .into_iter()
        .filter(|e| seen.insert((e.pool_id, e.year)))
        .collect()
}

/// Every revision of one pool's draw for one year.
#[derive(Debug, Clone, PartialEq)]
pub struct RevisionHistory {
    pub pool_id: i32,
    pub pool_name: String,
    pub year: i32,
    /// Highest revision first, so the head is the live draw.
    pub revisions: Vec<Exchange>,
}

/// Groups draws by pool and year, so a year's revisions can be shown together.
///
/// Groups come out in the order the pools and years were first seen, which for
/// `load_exchanges` means newest year first. Revisions inside a group are sorted
/// here rather than assumed, so the head is the live draw whatever order the
/// caller had them in.
pub fn revision_histories(exchanges: Vec<Exchange>) -> Vec<RevisionHistory> {
    let mut groups: Vec<RevisionHistory> = Vec::new();

    for exchange in exchanges {
        if let Some(group) = groups
            .iter_mut()
            .find(|g| g.pool_id == exchange.pool_id && g.year == exchange.year)
        {
            group.revisions.push(exchange);
        } else {
            groups.push(RevisionHistory {
                pool_id: exchange.pool_id,
                pool_name: exchange.pool_name.clone(),
                year: exchange.year,
                revisions: vec![exchange],
            });
        }
    }

    for group in &mut groups {
        group.revisions.sort_by_key(|e| std::cmp::Reverse(e.revision));
    }

    groups
}

/// What a proposed swap would do, as shown before it is applied.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SwapPreview {
    pub exchange_id: i32,
    pub a: String,
    pub b: String,
    /// Only the pairings that move.
    pub changes: Vec<SwapChange>,
    /// Rules the swap would break. Not a refusal — see [`SwapViolation`].
    pub violations: Vec<SwapViolation>,
}

impl Exchange {
    /// Whether a manager stepped in after the solver — a swap, or an earlier
    /// revision put back in charge.
    ///
    /// The two differ in one consequence: a swap moves pairings the seed no
    /// longer produces, so it carries none, while a restore brings back pairings
    /// its seed still replays.
    pub fn was_adjusted(&self) -> bool {
        self.adjusted_from.is_some()
    }

    /// The permutation split into cycles, for visualization.
    pub fn cycles(&self) -> Vec<Vec<String>> {
        let next: std::collections::HashMap<&str, &str> = self
            .pairings
            .iter()
            .map(|p| (p.giver.as_str(), p.receiver.as_str()))
            .collect();

        let mut seen: std::collections::HashSet<&str> = std::collections::HashSet::new();
        let mut cycles = Vec::new();

        for p in &self.participants {
            let start = p.as_str();
            if seen.contains(start) {
                continue;
            }
            let mut cycle = Vec::new();
            let mut cur = start;
            while !seen.contains(cur) {
                seen.insert(cur);
                cycle.push(cur.to_string());
                match next.get(cur) {
                    Some(n) => cur = n,
                    // Malformed snapshot (partial data); stop rather than loop.
                    None => break,
                }
            }
            if !cycle.is_empty() {
                cycles.push(cycle);
            }
        }

        cycles
    }
}

/// Everything the pool page needs in one round trip.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PoolDetail {
    pub pool: Pool,
    pub members: Vec<Participant>,
    pub excluded_letters: Vec<char>,
    pub exchanges: Vec<Exchange>,
}

/// Turns a display name into a URL-safe slug.
pub fn slugify(name: &str) -> String {
    let mut slug = String::new();
    let mut last_dash = true;

    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
            last_dash = false;
        } else if !last_dash {
            slug.push('-');
            last_dash = true;
        }
    }

    let trimmed = slug.trim_matches('-').to_string();
    if trimmed.is_empty() {
        "pool".to_string()
    } else {
        trimmed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugify_handles_real_pool_names() {
        assert_eq!(slugify("Island Life"), "island-life");
        assert_eq!(slugify("Grabergishimazureson"), "grabergishimazureson");
        assert_eq!(slugify("Pets"), "pets");
        assert_eq!(slugify("  Spaces   Everywhere  "), "spaces-everywhere");
        assert_eq!(slugify("K-Lee's Crew!"), "k-lee-s-crew");
        assert_eq!(slugify("???"), "pool");
    }

    #[test]
    fn cycles_decomposes_a_snapshot() {
        let exchange = Exchange {
            id: 1,
            pool_id: 1,
            pool_name: "Test".into(),
            pool_slug: "test".into(),
            year: 2026,
            revision: 1,
            letter: Some('K'),
            participants: vec!["A".into(), "B".into(), "C".into(), "D".into(), "E".into(), "F".into()],
            exclusions: vec![],
            // Two rings: A→B→C→A and D→E→F→D
            pairings: vec![
                Pairing {
                    giver: "A".into(),
                    receiver: "B".into(),
                },
                Pairing {
                    giver: "B".into(),
                    receiver: "C".into(),
                },
                Pairing {
                    giver: "C".into(),
                    receiver: "A".into(),
                },
                Pairing {
                    giver: "D".into(),
                    receiver: "E".into(),
                },
                Pairing {
                    giver: "E".into(),
                    receiver: "F".into(),
                },
                Pairing {
                    giver: "F".into(),
                    receiver: "D".into(),
                },
            ],
            config: None,
            seed: None,
            adjusted_from: None,
            adjustment_note: None,
        };

        let cycles = exchange.cycles();
        assert_eq!(cycles.len(), 2);
        assert!(cycles.iter().all(|c| c.len() == 3));
    }

    #[test]
    fn cycles_survives_a_malformed_snapshot() {
        let exchange = Exchange {
            id: 1,
            pool_id: 1,
            pool_name: "Test".into(),
            pool_slug: "test".into(),
            year: 2026,
            revision: 1,
            letter: None,
            participants: vec!["A".into(), "B".into(), "C".into()],
            // C has no outgoing edge.
            pairings: vec![
                Pairing {
                    giver: "A".into(),
                    receiver: "B".into(),
                },
                Pairing {
                    giver: "B".into(),
                    receiver: "C".into(),
                },
            ],
            exclusions: vec![],
            config: None,
            seed: None,
            adjusted_from: None,
            adjustment_note: None,
        };

        let cycles = exchange.cycles();
        assert_eq!(cycles.len(), 1);
        assert_eq!(cycles[0], vec!["A", "B", "C"]);
    }

    fn revision_of(pool_id: i32, year: i32, revision: i32) -> Exchange {
        Exchange {
            id: pool_id * 1000 + year * 10 + revision,
            pool_id,
            pool_name: format!("Pool {pool_id}"),
            pool_slug: format!("pool-{pool_id}"),
            year,
            revision,
            letter: None,
            participants: vec![],
            exclusions: vec![],
            pairings: vec![],
            config: None,
            seed: None,
            adjusted_from: None,
            adjustment_note: None,
        }
    }

    #[test]
    fn revisions_are_grouped_by_pool_and_year() {
        // As the query returns them: year then revision, both descending, with
        // two pools interleaved within a year.
        let rows = vec![
            revision_of(1, 2026, 3),
            revision_of(1, 2026, 2),
            revision_of(2, 2026, 1),
            revision_of(1, 2026, 1),
            revision_of(1, 2025, 1),
        ];

        let histories = revision_histories(rows);
        let shape: Vec<(i32, i32, usize)> = histories
            .iter()
            .map(|h| (h.pool_id, h.year, h.revisions.len()))
            .collect();
        assert_eq!(shape, vec![(1, 2026, 3), (2, 2026, 1), (1, 2025, 1)]);
    }

    /// The head of each group is what the restore UI marks as current, so it has
    /// to be the live revision however the rows arrived.
    #[test]
    fn the_newest_revision_leads_its_group() {
        let rows = vec![
            revision_of(1, 2026, 1),
            revision_of(1, 2026, 3),
            revision_of(1, 2026, 2),
        ];

        let histories = revision_histories(rows);
        let revisions: Vec<i32> = histories[0].revisions.iter().map(|e| e.revision).collect();
        assert_eq!(revisions, vec![3, 2, 1]);
    }

    #[test]
    fn nothing_drawn_means_nothing_to_group() {
        assert!(revision_histories(vec![]).is_empty());
    }
}
