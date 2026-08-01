// This crate is built out of immutable value types whose methods return an
// updated copy, so nearly every one trips `return_self_not_must_use`; annotating
// all of them adds noise without catching anything. The other two are deliberate
// local style: `use Enum::*` inside a function that matches on every variant, and
// exhaustive truth tables whose arms repeat a body on purpose to stay readable.
#![allow(clippy::return_self_not_must_use)]
#![allow(clippy::enum_glob_use)]
#![allow(clippy::match_same_arms)]

mod baseball;

pub use baseball::*;
