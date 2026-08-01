//! What happened on a ball in play, named independently of game state.
//!
//! [`PlayOutcome`](super::baserunners::PlayOutcome) records the *end state* of a
//! play, so building one correctly requires knowing who was on base when it
//! started. Making callers supply that state is a bug factory: pass stale or
//! empty baserunners and the real ones are silently erased. `PlayResult` names
//! the play instead and lets [`HalfInning`](super::inning::HalfInning) resolve it
//! against the baserunners it already owns, so there is no second copy of the
//! truth to get wrong.

use std::fmt::Display;

use super::{baserunners::PlayOutcome, lineup::BattingPosition};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlayResult {
    Single,
    Double,
    Triple,
    /// Retired on the ground; runners hold.
    Groundout,
    /// Retired in the air; runners hold.
    Flyout,
    /// Retired on a line drive; runners hold.
    Lineout,
    /// Retired on a popup; runners hold.
    Popout,
    /// A fly deep enough for the runner on third to score after the catch.
    SacrificeFly,
    /// The forced runner is retired and the batter reaches first.
    FieldersChoice,
    /// Two outs on one ball: the forced runner and the batter.
    DoublePlay,
}

impl PlayResult {
    /// Applies this play to a specific game state.
    pub(crate) fn resolve(
        self,
        baserunners: super::baserunners::BaserunnerState,
        batter: BattingPosition,
    ) -> PlayOutcome {
        match self {
            PlayResult::Single => PlayOutcome::single(baserunners, batter),
            PlayResult::Double => PlayOutcome::double(baserunners, batter),
            PlayResult::Triple => PlayOutcome::triple(baserunners, batter),
            PlayResult::Groundout => PlayOutcome::groundout(baserunners),
            PlayResult::Flyout | PlayResult::Lineout | PlayResult::Popout => PlayOutcome::flyout(baserunners),
            PlayResult::SacrificeFly => PlayOutcome::sacrifice_fly(baserunners),
            PlayResult::FieldersChoice => PlayOutcome::fielders_choice(baserunners, batter),
            PlayResult::DoublePlay => PlayOutcome::double_play(baserunners),
        }
    }

    /// Whether the batter was retired, before any base-running is resolved.
    pub fn is_out(self) -> bool {
        matches!(
            self,
            PlayResult::Groundout
                | PlayResult::Flyout
                | PlayResult::Lineout
                | PlayResult::Popout
                | PlayResult::SacrificeFly
                | PlayResult::DoublePlay
        )
    }

    /// Whether the batter is credited with a hit.
    pub fn is_hit(self) -> bool {
        matches!(self, PlayResult::Single | PlayResult::Double | PlayResult::Triple)
    }

    /// Short all-caps label for the result banner.
    pub fn label(self) -> &'static str {
        match self {
            PlayResult::Single => "SINGLE!",
            PlayResult::Double => "DOUBLE!",
            PlayResult::Triple => "TRIPLE!",
            PlayResult::Groundout => "GROUND OUT",
            PlayResult::Flyout => "FLY OUT",
            PlayResult::Lineout => "LINE OUT",
            PlayResult::Popout => "POP OUT",
            PlayResult::SacrificeFly => "SAC FLY",
            PlayResult::FieldersChoice => "FIELDER'S CHOICE",
            PlayResult::DoublePlay => "DOUBLE PLAY!",
        }
    }
}

impl Display for PlayResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

#[cfg(test)]
mod tests {
    use super::{super::baserunners::BaserunnerState, *};

    fn loaded() -> BaserunnerState {
        BaserunnerState::empty()
            .set_first(Some(BattingPosition::First))
            .set_second(Some(BattingPosition::Second))
            .set_third(Some(BattingPosition::Third))
    }

    #[test]
    fn a_groundout_with_the_bases_loaded_leaves_every_runner_where_they_were() {
        let outcome = PlayResult::Groundout.resolve(loaded(), BattingPosition::Fourth);

        let after = outcome.baserunners();
        assert_eq!(after.first(), Some(BattingPosition::First));
        assert_eq!(after.second(), Some(BattingPosition::Second));
        assert_eq!(after.third(), Some(BattingPosition::Third));
        assert_eq!(outcome.outs(), super::super::inning::Outs::One);
        assert_eq!(outcome.runs_scored(), 0);
    }

    #[test]
    fn a_flyout_with_nobody_on_records_one_out_and_nothing_else() {
        let outcome = PlayResult::Flyout.resolve(BaserunnerState::empty(), BattingPosition::First);
        assert!(outcome.baserunners().is_empty());
        assert_eq!(outcome.outs(), super::super::inning::Outs::One);
    }

    #[test]
    fn a_sacrifice_fly_scores_the_runner_from_third_and_holds_the_rest() {
        let runners = BaserunnerState::empty()
            .set_first(Some(BattingPosition::First))
            .set_third(Some(BattingPosition::Third));
        let outcome = PlayResult::SacrificeFly.resolve(runners, BattingPosition::Fourth);

        assert_eq!(outcome.runs_scored(), 1);
        assert_eq!(outcome.baserunners().first(), Some(BattingPosition::First));
        assert_eq!(outcome.baserunners().third(), None);
        assert_eq!(outcome.outs(), super::super::inning::Outs::One);
    }

    #[test]
    fn a_sacrifice_fly_with_nobody_on_third_is_just_a_flyout() {
        let runners = BaserunnerState::empty().set_first(Some(BattingPosition::First));
        let outcome = PlayResult::SacrificeFly.resolve(runners, BattingPosition::Fourth);

        assert_eq!(outcome.runs_scored(), 0);
        assert_eq!(outcome.baserunners().first(), Some(BattingPosition::First));
    }

    #[test]
    fn a_double_play_retires_two_and_pushes_the_runner_from_second_along() {
        // The out claims second base, so the runner standing there has to move up.
        let runners = BaserunnerState::empty()
            .set_first(Some(BattingPosition::First))
            .set_second(Some(BattingPosition::Second));
        let outcome = PlayResult::DoublePlay.resolve(runners, BattingPosition::Third);

        assert_eq!(outcome.outs(), super::super::inning::Outs::Two);
        assert_eq!(outcome.baserunners().first(), None);
        assert_eq!(outcome.baserunners().second(), None);
        assert_eq!(
            outcome.baserunners().third(),
            Some(BattingPosition::Second),
            "runner from second should be pushed to third, not dropped"
        );
    }

    #[test]
    fn a_double_play_needs_a_runner_on_first_to_turn() {
        let runners = BaserunnerState::empty().set_second(Some(BattingPosition::Second));
        let outcome = PlayResult::DoublePlay.resolve(runners, BattingPosition::Third);

        assert_eq!(outcome.outs(), super::super::inning::Outs::One, "no force available");
        assert_eq!(outcome.baserunners().second(), Some(BattingPosition::Second));
    }

    #[test]
    fn a_fielders_choice_trades_the_forced_runner_for_the_batter() {
        let runners = BaserunnerState::empty().set_first(Some(BattingPosition::First));
        let outcome = PlayResult::FieldersChoice.resolve(runners, BattingPosition::Second);

        assert_eq!(outcome.outs(), super::super::inning::Outs::One);
        assert_eq!(outcome.baserunners().first(), Some(BattingPosition::Second));
        assert_eq!(outcome.baserunners().second(), None);
    }

    #[test]
    fn a_fielders_choice_with_nobody_forced_is_a_single() {
        let outcome = PlayResult::FieldersChoice.resolve(BaserunnerState::empty(), BattingPosition::First);

        assert_eq!(outcome.outs(), super::super::inning::Outs::Zero);
        assert_eq!(outcome.baserunners().first(), Some(BattingPosition::First));
    }

    #[test]
    fn a_fielders_choice_holds_a_runner_on_third_who_is_not_forced() {
        let runners = BaserunnerState::empty()
            .set_first(Some(BattingPosition::First))
            .set_third(Some(BattingPosition::Third));
        let outcome = PlayResult::FieldersChoice.resolve(runners, BattingPosition::Fourth);

        assert_eq!(outcome.runs_scored(), 0, "an unforced runner on third should hold");
        assert_eq!(outcome.baserunners().third(), Some(BattingPosition::Third));
    }

    #[test]
    fn only_force_plays_and_the_batter_suppress_runs_on_the_third_out() {
        assert!(
            PlayResult::Groundout
                .resolve(loaded(), BattingPosition::Fourth)
                .suppresses_runs_on_third_out()
        );
        assert!(
            PlayResult::DoublePlay
                .resolve(loaded(), BattingPosition::Fourth)
                .suppresses_runs_on_third_out()
        );
        assert!(
            !PlayResult::Single
                .resolve(loaded(), BattingPosition::Fourth)
                .suppresses_runs_on_third_out()
        );
    }
}
