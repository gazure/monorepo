//! The draw engine.
//!
//! Produces a giver → receiver permutation subject to configurable structural
//! rules. Everything here is pure and deterministic: the same [`DrawConfig`]
//! seed and the same inputs always yield the same draw, so any result can be
//! replayed and audited.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Shortest cycle we ever allow. A 2-cycle is a reciprocal A→B/B→A pair, which
/// is precisely what the family wants to avoid.
pub const DEFAULT_MIN_CYCLE_LEN: usize = 3;

/// How many search steps a single draw may consume before giving up.
///
/// A step budget rather than a wall-clock deadline keeps draws reproducible —
/// a time limit would make the same seed produce different results depending on
/// machine load.
const DEFAULT_STEP_BUDGET: u64 = 2_000_000;

/// The shape the giver → receiver permutation should take.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum CycleMode {
    /// A single cycle threading every participant — one big ring.
    Grand,
    /// Any number of independent cycles, each at least `min_len` long.
    Multiple { min_len: usize },
}

impl CycleMode {
    /// The minimum cycle length this mode implies for `n` participants.
    ///
    /// `Grand` is the special case where the only permissible cycle is one that
    /// contains everybody, so it is exactly `Multiple { min_len: n }`. Both
    /// modes therefore run through the same search.
    fn min_len(self, n: usize) -> usize {
        match self {
            Self::Grand => n,
            Self::Multiple { min_len } => min_len,
        }
    }
}

/// Why a particular giver → receiver edge is forbidden.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "reason", rename_all = "snake_case")]
pub enum BlockReason {
    Spouse,
    Household,
    Manual,
    /// They gave to this person in the given year.
    RepeatOf {
        year: i32,
    },
}

impl std::fmt::Display for BlockReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Spouse => write!(f, "spouse"),
            Self::Household => write!(f, "same household"),
            Self::Manual => write!(f, "manual exclusion"),
            Self::RepeatOf { year } => write!(f, "gave to them in {year}"),
        }
    }
}

/// A forbidden directed edge. Symmetric relationships are expanded into two of
/// these by the caller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockedEdge {
    pub giver: String,
    pub receiver: String,
    pub reason: BlockReason,
}

/// Settings for a single draw. Snapshotted alongside the result so past years
/// stay explainable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DrawConfig {
    pub cycle_mode: CycleMode,
    pub exclude_spouses: bool,
    /// `None` disables the rule; `Some(n)` avoids receivers from the last `n` years.
    pub avoid_repeat_years: Option<u32>,
    pub seed: u64,
}

impl Default for DrawConfig {
    fn default() -> Self {
        Self {
            cycle_mode: CycleMode::Grand,
            exclude_spouses: true,
            avoid_repeat_years: Some(1),
            seed: 0,
        }
    }
}

/// A completed draw.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Draw {
    pub pairings: Vec<(String, String)>,
    /// The permutation decomposed into cycles, for visualization.
    pub cycles: Vec<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DrawError {
    TooFewParticipants {
        got: usize,
        need: usize,
    },
    /// Somebody has no legal receiver at all — always a constraint problem, never
    /// a search problem, so it is worth reporting separately and precisely.
    NoCandidates {
        participant: String,
        blocked_by: Vec<(String, BlockReason)>,
    },
    /// The search ran out of budget without finding a valid arrangement.
    Exhausted {
        relax_hint: String,
    },
}

impl std::fmt::Display for DrawError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooFewParticipants { got, need } => {
                write!(f, "need at least {need} participants for this cycle mode, have {got}")
            }
            Self::NoCandidates {
                participant,
                blocked_by,
            } => {
                write!(f, "{participant} has no one left to give to — blocked from ")?;
                for (i, (who, why)) in blocked_by.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{who} ({why})")?;
                }
                Ok(())
            }
            Self::Exhausted { relax_hint } => {
                write!(f, "no valid arrangement exists with these settings. {relax_hint}")
            }
        }
    }
}

impl std::error::Error for DrawError {}

/// Runs a draw.
///
/// `blocked` holds directed forbidden edges; the caller is responsible for
/// expanding symmetric relationships into both directions and for only
/// including the rules the config enables.
pub fn build_draw(participants: &[String], blocked: &[BlockedEdge], config: &DrawConfig) -> Result<Draw, DrawError> {
    let n = participants.len();
    let min_len = config.cycle_mode.min_len(n);

    let need = match config.cycle_mode {
        CycleMode::Grand => DEFAULT_MIN_CYCLE_LEN,
        CycleMode::Multiple { min_len } => min_len,
    };
    if n < need {
        return Err(DrawError::TooFewParticipants { got: n, need });
    }

    let index: HashMap<&str, usize> = participants.iter().enumerate().map(|(i, p)| (p.as_str(), i)).collect();

    // allowed[g][r] — start fully permitted, then knock out self and blocked edges.
    let mut allowed = vec![vec![true; n]; n];
    for (i, row) in allowed.iter_mut().enumerate() {
        row[i] = false;
    }

    // Retained per-giver so an impossible constraint set can name its culprits.
    let mut blocked_by: Vec<Vec<(String, BlockReason)>> = vec![Vec::new(); n];

    for edge in blocked {
        let (Some(&g), Some(&r)) = (index.get(edge.giver.as_str()), index.get(edge.receiver.as_str())) else {
            // References somebody outside this pool; harmless, skip.
            continue;
        };
        if g == r || !allowed[g][r] {
            continue;
        }
        allowed[g][r] = false;
        blocked_by[g].push((edge.receiver.clone(), edge.reason));
    }

    // Cheap feasibility pre-check: report the real problem instead of letting the
    // search flail and return a generic failure.
    for (g, row) in allowed.iter().enumerate() {
        if !row.iter().any(|&ok| ok) {
            return Err(DrawError::NoCandidates {
                participant: participants[g].clone(),
                blocked_by: blocked_by[g].clone(),
            });
        }
    }

    let mut solver = Solver {
        n,
        min_len,
        allowed: &allowed,
        next: vec![usize::MAX; n],
        prev: vec![usize::MAX; n],
        taken: vec![false; n],
        assigned: 0,
        steps: 0,
        budget: DEFAULT_STEP_BUDGET,
        rng: fastrand::Rng::with_seed(config.seed),
    };

    if !solver.solve() {
        return Err(DrawError::Exhausted {
            relax_hint: relax_hint(config),
        });
    }

    let next = solver.next.clone();
    Ok(Draw {
        pairings: (0..n)
            .map(|g| (participants[g].clone(), participants[next[g]].clone()))
            .collect(),
        cycles: decompose_cycles(&next, participants),
    })
}

/// Points the user at the specific toggle most likely to be over-constraining.
fn relax_hint(config: &DrawConfig) -> String {
    let mut candidates = Vec::new();
    if config.avoid_repeat_years.is_some() {
        candidates.push("allowing repeats of previous years' receivers");
    }
    if matches!(config.cycle_mode, CycleMode::Grand) {
        candidates.push("switching from one grand cycle to multiple cycles");
    }
    if config.exclude_spouses {
        candidates.push("allowing spouses to give to each other");
    }

    match candidates.len() {
        0 => "Try removing some manual exclusions.".to_string(),
        1 => format!("Try {}.", candidates[0]),
        _ => format!(
            "Try {}, or {}.",
            candidates[..candidates.len() - 1].join(", "),
            candidates[candidates.len() - 1]
        ),
    }
}

/// Splits a permutation into its constituent cycles.
fn decompose_cycles(next: &[usize], participants: &[String]) -> Vec<Vec<String>> {
    let mut seen = vec![false; next.len()];
    let mut cycles = Vec::new();

    for start in 0..next.len() {
        if seen[start] {
            continue;
        }
        let mut cycle = Vec::new();
        let mut cur = start;
        while !seen[cur] {
            seen[cur] = true;
            cycle.push(participants[cur].clone());
            cur = next[cur];
        }
        cycles.push(cycle);
    }

    cycles
}

struct Solver<'a> {
    n: usize,
    min_len: usize,
    allowed: &'a [Vec<bool>],
    /// `next[g]` is who g gives to, or `usize::MAX` if unassigned.
    next: Vec<usize>,
    prev: Vec<usize>,
    taken: Vec<bool>,
    assigned: usize,
    steps: u64,
    budget: u64,
    rng: fastrand::Rng,
}

impl Solver<'_> {
    fn solve(&mut self) -> bool {
        if self.assigned == self.n {
            return true;
        }
        if self.steps >= self.budget {
            return false;
        }
        self.steps += 1;

        let Some(giver) = self.select_giver() else {
            return false;
        };

        let mut candidates: Vec<usize> = (0..self.n).filter(|&r| self.can_assign(giver, r)).collect();
        self.rng.shuffle(&mut candidates);

        for receiver in candidates {
            self.next[giver] = receiver;
            self.prev[receiver] = giver;
            self.taken[receiver] = true;
            self.assigned += 1;

            if self.solve() {
                return true;
            }

            self.next[giver] = usize::MAX;
            self.prev[receiver] = usize::MAX;
            self.taken[receiver] = false;
            self.assigned -= 1;

            if self.steps >= self.budget {
                return false;
            }
        }

        false
    }

    /// Minimum-remaining-values heuristic: expand whoever is most constrained,
    /// so dead ends surface near the top of the tree instead of the bottom.
    fn select_giver(&self) -> Option<usize> {
        let mut best = None;
        let mut best_count = usize::MAX;

        for g in 0..self.n {
            if self.next[g] != usize::MAX {
                continue;
            }
            let count = (0..self.n).filter(|&r| self.can_assign(g, r)).count();
            if count == 0 {
                // Dead end — no point exploring any other branch from here.
                return Some(g);
            }
            if count < best_count {
                best_count = count;
                best = Some(g);
            }
        }

        best
    }

    fn can_assign(&self, giver: usize, receiver: usize) -> bool {
        if self.taken[receiver] || !self.allowed[giver][receiver] {
            return false;
        }

        // Assigning giver→receiver closes a cycle iff the chain already running
        // forward out of `receiver` leads back to `giver`.
        match self.chain_len(receiver, giver) {
            Some(len) => {
                let cycle_len = len + 1;
                // A closing cycle must be long enough, and it may only be short
                // of the full set if there is still work left for everyone else.
                cycle_len >= self.min_len
            }
            None => true,
        }
    }

    /// Number of hops from `start` to `target` along assigned edges, or `None`
    /// if the chain ends before reaching it.
    fn chain_len(&self, start: usize, target: usize) -> Option<usize> {
        let mut cur = start;
        let mut len = 0;
        loop {
            if cur == target {
                return Some(len);
            }
            if self.next[cur] == usize::MAX {
                return None;
            }
            cur = self.next[cur];
            len += 1;
        }
    }
}

/// Picks the letter of the year.
///
/// `excluded` is the pool's manually-disabled set; `already_used` is every
/// letter the pool has drawn before, so years do not repeat by default.
pub fn select_letter(excluded: &[char], already_used: &[char], seed: u64) -> Option<char> {
    let mut rng = fastrand::Rng::with_seed(seed);

    let fresh: Vec<char> = ('A'..='Z')
        .filter(|c| !excluded.contains(c) && !already_used.contains(c))
        .collect();

    // Once every permissible letter has been used, cycle round rather than
    // failing — the family would rather repeat a letter than have none.
    let pool: Vec<char> = if fresh.is_empty() {
        ('A'..='Z').filter(|c| !excluded.contains(c)).collect()
    } else {
        fresh
    };

    if pool.is_empty() {
        return None;
    }
    Some(pool[rng.usize(..pool.len())])
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    fn people(names: &[&str]) -> Vec<String> {
        names.iter().map(|s| (*s).to_string()).collect()
    }

    fn symmetric(a: &str, b: &str, reason: BlockReason) -> Vec<BlockedEdge> {
        vec![
            BlockedEdge {
                giver: a.to_string(),
                receiver: b.to_string(),
                reason,
            },
            BlockedEdge {
                giver: b.to_string(),
                receiver: a.to_string(),
                reason,
            },
        ]
    }

    fn cfg(mode: CycleMode, seed: u64) -> DrawConfig {
        DrawConfig {
            cycle_mode: mode,
            exclude_spouses: true,
            avoid_repeat_years: Some(1),
            seed,
        }
    }

    /// Every draw must be a permutation with no fixed points.
    fn assert_valid_permutation(draw: &Draw, participants: &[String]) {
        assert_eq!(draw.pairings.len(), participants.len());

        let givers: HashSet<&str> = draw.pairings.iter().map(|(g, _)| g.as_str()).collect();
        let receivers: HashSet<&str> = draw.pairings.iter().map(|(_, r)| r.as_str()).collect();
        assert_eq!(givers.len(), participants.len(), "every participant gives exactly once");
        assert_eq!(
            receivers.len(),
            participants.len(),
            "every participant receives exactly once"
        );

        for (g, r) in &draw.pairings {
            assert_ne!(g, r, "nobody gives to themselves");
        }
    }

    #[test]
    fn grand_mode_yields_exactly_one_cycle() {
        let participants = people(&["Alice", "Bob", "Carol", "Dave", "Eve"]);
        for seed in 0..200 {
            let draw = build_draw(&participants, &[], &cfg(CycleMode::Grand, seed)).expect("draw should succeed");
            assert_valid_permutation(&draw, &participants);
            assert_eq!(draw.cycles.len(), 1, "grand mode is a single ring");
            assert_eq!(draw.cycles[0].len(), participants.len());
        }
    }

    /// The property the old `fallback_exchange` silently violated.
    #[test]
    fn never_produces_reciprocal_pairs() {
        let participants = people(&["Alice", "Bob", "Carol", "Dave", "Eve", "Frank"]);
        let modes = [CycleMode::Grand, CycleMode::Multiple { min_len: 3 }];

        for mode in modes {
            for seed in 0..200 {
                let draw = build_draw(&participants, &[], &cfg(mode, seed)).expect("draw should succeed");
                let pairs: HashSet<(&str, &str)> =
                    draw.pairings.iter().map(|(g, r)| (g.as_str(), r.as_str())).collect();

                for (g, r) in &pairs {
                    assert!(
                        !pairs.contains(&(*r, *g)),
                        "{g} → {r} and {r} → {g} both present under {mode:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn every_cycle_meets_the_minimum_length() {
        let participants = people(&["A", "B", "C", "D", "E", "F", "G", "H", "I", "J"]);

        for min_len in [3usize, 4, 5] {
            for seed in 0..100 {
                let draw = build_draw(&participants, &[], &cfg(CycleMode::Multiple { min_len }, seed))
                    .expect("draw should succeed");
                assert_valid_permutation(&draw, &participants);

                for cycle in &draw.cycles {
                    assert!(cycle.len() >= min_len, "cycle {cycle:?} shorter than min_len {min_len}");
                }
                let total: usize = draw.cycles.iter().map(Vec::len).sum();
                assert_eq!(total, participants.len(), "cycles must cover everyone exactly once");
            }
        }
    }

    #[test]
    fn respects_blocked_edges() {
        let participants = people(&["Alice", "Bob", "Carol", "Dave", "Eve", "Frank"]);
        let mut blocked = symmetric("Alice", "Bob", BlockReason::Spouse);
        blocked.extend(symmetric("Carol", "Dave", BlockReason::Spouse));
        blocked.push(BlockedEdge {
            giver: "Eve".to_string(),
            receiver: "Frank".to_string(),
            reason: BlockReason::RepeatOf { year: 2025 },
        });

        for seed in 0..200 {
            let draw = build_draw(&participants, &blocked, &cfg(CycleMode::Grand, seed)).expect("draw should succeed");
            assert_valid_permutation(&draw, &participants);

            for (g, r) in &draw.pairings {
                for edge in &blocked {
                    assert!(
                        !(g == &edge.giver && r == &edge.receiver),
                        "{g} → {r} violates {}",
                        edge.reason
                    );
                }
            }
        }
    }

    #[test]
    fn draws_are_reproducible() {
        let participants = people(&["Alice", "Bob", "Carol", "Dave", "Eve"]);
        let first = build_draw(&participants, &[], &cfg(CycleMode::Grand, 12345)).unwrap();
        let second = build_draw(&participants, &[], &cfg(CycleMode::Grand, 12345)).unwrap();
        assert_eq!(first, second, "same seed must replay identically");
    }

    #[test]
    fn reports_the_participant_with_no_options() {
        let participants = people(&["Alice", "Bob", "Carol"]);
        let mut blocked = symmetric("Alice", "Bob", BlockReason::Spouse);
        blocked.push(BlockedEdge {
            giver: "Alice".to_string(),
            receiver: "Carol".to_string(),
            reason: BlockReason::RepeatOf { year: 2025 },
        });

        let err = build_draw(&participants, &blocked, &cfg(CycleMode::Grand, 0)).unwrap_err();
        match err {
            DrawError::NoCandidates {
                participant,
                blocked_by,
            } => {
                assert_eq!(participant, "Alice");
                assert_eq!(blocked_by.len(), 2);
            }
            other => panic!("expected NoCandidates, got {other:?}"),
        }
    }

    #[test]
    fn infeasible_sets_fail_cleanly_with_a_hint() {
        // Two couples, four people, grand cycle: A-B and C-D can't pair within
        // themselves, which leaves no Hamiltonian cycle avoiding both.
        let participants = people(&["A", "B", "C", "D"]);
        let mut blocked = symmetric("A", "B", BlockReason::Spouse);
        blocked.extend(symmetric("C", "D", BlockReason::Spouse));
        blocked.extend(symmetric("A", "C", BlockReason::Manual));
        blocked.extend(symmetric("B", "D", BlockReason::Manual));

        let err = build_draw(&participants, &blocked, &cfg(CycleMode::Grand, 0)).unwrap_err();
        match err {
            DrawError::Exhausted { relax_hint } => {
                assert!(!relax_hint.is_empty(), "failure should suggest what to relax");
            }
            DrawError::NoCandidates { .. } => {}
            DrawError::TooFewParticipants { .. } => {
                panic!("four participants is enough; this should not be a size failure")
            }
        }
    }

    #[test]
    fn too_few_participants_is_rejected() {
        let participants = people(&["Alice", "Bob"]);
        let err = build_draw(&participants, &[], &cfg(CycleMode::Grand, 0)).unwrap_err();
        assert_eq!(err, DrawError::TooFewParticipants { got: 2, need: 3 });
    }

    /// The real family shape: 14 people, five married couples, one grand cycle.
    #[test]
    fn handles_the_real_family_pool() {
        let participants = people(&[
            "Claire", "Grant", "Anne", "Duncan", "Noel", "K-Lee", "Steve", "Linda", "Chris", "Jim", "Kari", "Meaghann",
            "Alec", "Eric",
        ]);
        let couples = [
            ("Claire", "Duncan"),
            ("Anne", "Eric"),
            ("Noel", "K-Lee"),
            ("Steve", "Linda"),
            ("Jim", "Kari"),
        ];
        let blocked: Vec<BlockedEdge> = couples
            .iter()
            .flat_map(|(a, b)| symmetric(a, b, BlockReason::Spouse))
            .collect();

        for seed in 0..100 {
            let draw = build_draw(&participants, &blocked, &cfg(CycleMode::Grand, seed)).expect("draw should succeed");
            assert_valid_permutation(&draw, &participants);
            assert_eq!(draw.cycles.len(), 1);

            for (a, b) in &couples {
                for (g, r) in &draw.pairings {
                    assert!(!(g == a && r == b || g == b && r == a), "{a} and {b} are married");
                }
            }
        }
    }

    #[test]
    fn letter_avoids_previously_used() {
        let used: Vec<char> = ('A'..='X').collect();
        for seed in 0..50 {
            let letter = select_letter(&[], &used, seed).unwrap();
            assert!(
                matches!(letter, 'Y' | 'Z'),
                "should pick an unused letter, got {letter}"
            );
        }
    }

    #[test]
    fn letter_recycles_once_all_are_used() {
        let used: Vec<char> = ('A'..='Z').collect();
        let letter = select_letter(&[], &used, 0).expect("should recycle rather than fail");
        assert!(letter.is_ascii_uppercase());
    }

    #[test]
    fn letter_respects_pool_exclusions() {
        // Grabergishimazureson's historical allowed set.
        let allowed: Vec<char> = "ACDIJLMNORSTUXYZ".chars().collect();
        let excluded: Vec<char> = ('A'..='Z').filter(|c| !allowed.contains(c)).collect();

        for seed in 0..100 {
            let letter = select_letter(&excluded, &[], seed).unwrap();
            assert!(allowed.contains(&letter), "{letter} is not in the pool's letter set");
        }
    }

    #[test]
    fn letter_returns_none_when_everything_is_excluded() {
        let excluded: Vec<char> = ('A'..='Z').collect();
        assert_eq!(select_letter(&excluded, &[], 0), None);
    }
}
