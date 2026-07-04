//! Local LeetCode practice harness.
//!
//! Workflow:
//!
//! 1. `cargo run -p leetcode -- fetch <id>` scaffolds `src/problems/pNNNN_<slug>.rs` with the
//!    problem description as doc comments, the official Rust signature, and best-effort example
//!    tests parsed from the problem statement.
//! 2. Solve the problem in the generated file.
//! 3. `cargo test -p leetcode pNNNN` until green.
//!
//! Notes and accepted limitations:
//!
//! - Only free-tier problems are fetchable (no auth).
//! - Examples that can't be auto-translated (linked lists, trees, in-place problems) become
//!   `#[ignore]`d stubs to fill in by hand; see [`support`] for `ListNode`/`TreeNode` builders.
//! - "Return in any order" problems generate order-sensitive assertions — edit as needed.
//! - Don't commit a scaffold you haven't solved: its example tests fail by design (red/green).

// Words like "LeetCode" and "GraphQL" appear throughout this crate's prose.
#![allow(clippy::doc_markdown)]

pub mod harness;
pub mod problems;
pub mod support;
