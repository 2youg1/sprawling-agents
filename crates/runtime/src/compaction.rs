// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! When to shorten something, and what to keep.
//!
//! The mechanism half of this — moving a big result out of the window
//! and leaving a reference — is `offload`'s and was frozen first. This
//! is the policy half: given a piece of text, what kind of thing is it,
//! and which end of it carries the information.
//!
//! Three rules hold the whole module up.
//!
//! **The result is never longer than the input.** Shortening that
//! lengthens is not a corner case, it is the failure that makes a
//! context budget meaningless, so it is an invariant with a test rather
//! than a property people remember.
//!
//! **No regular expressions on this path.** Detection is a handful of
//! prefix and count checks over the first few lines; a pattern engine
//! here would put backtracking between a model and its next turn.
//!
//! **Cuts land on character boundaries.** A budget in bytes and a cut in
//! the middle of a character is how a compactor produces text nothing
//! downstream can read. The boundary rule and the marker that announces
//! the loss both belong to `elision`, which every shortening path here
//! shares.

use kernel::ByteLen;

use crate::elision::{self, Cut, Elided};

/// What a piece of text is, as far as shortening is concerned. Seven,
/// and `Unknown` is one of them: a compactor that had to guess would
/// guess wrong on exactly the material nobody anticipated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Content {
    /// Sentences. The beginning says what it is about.
    Prose,
    /// Source. Both ends matter and the middle repeats.
    Code,
    /// A patch. The hunk headers are the information.
    Diff,
    /// Lines with a shape, most of them the same. The end is the news.
    Log,
    /// Structured data. Shortening it makes it unparseable, so it goes
    /// out of the window whole instead.
    Structured,
    /// Rows and columns. The header row is load-bearing.
    Table,
    /// Anything else, including binary-looking material.
    Unknown,
}

/// Which end of a text survives a shortening.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Strategy {
    /// Keep the front. Prose and tables read from the top.
    Head,
    /// Keep both ends and mark the gap. Code and diffs are read from
    /// the edges inward.
    Ends,
    /// Keep the back. A log's news is at the end.
    Tail,
}

/// What has to happen to one piece of text: decided here, carried out
/// by the caller.
///
/// Exhaustive, and `MustOffload` is why it exists. This module used to
/// answer "do not shorten it" with a strategy the caller was free to
/// read as "shorten it plainly, then", and the caller did - which left
/// one crate holding two answers to whether structured data may be
/// truncated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shrink {
    /// It is within budget and goes to the model unchanged.
    Keep,
    /// Shorten it, keeping this end.
    Cut(Strategy),
    /// Do not shorten it at all — move it out of the window and leave a
    /// reference. Truncated structured data is worse than absent
    /// structured data, because it looks like it can be parsed.
    MustOffload,
}

/// Detection, in priority order, over the first lines only.
///
/// Order is the design: a diff is also code, a table is also prose, and
/// whichever check runs first wins. Cheapest and most specific first.
#[must_use]
pub fn detect(text: &str) -> Content {
    let trimmed = text.trim_start();
    if trimmed.is_empty() {
        return Content::Unknown;
    }
    if trimmed.starts_with("diff --git")
        || trimmed.starts_with("--- ")
        || trimmed.starts_with("@@ ")
    {
        return Content::Diff;
    }
    if trimmed.starts_with('{') || trimmed.starts_with('[') || trimmed.starts_with("<?xml") {
        return Content::Structured;
    }
    let head: Vec<&str> = trimmed.lines().take(8).collect();
    if head.len() >= 2 && head.iter().filter(|line| line.contains('|')).count() >= 2 {
        return Content::Table;
    }
    if head
        .iter()
        .filter(|line| looks_like_a_log_line(line))
        .count()
        >= 2
    {
        return Content::Log;
    }
    if head.iter().any(|line| {
        let line = line.trim_start();
        line.starts_with("fn ")
            || line.starts_with("def ")
            || line.starts_with("class ")
            || line.starts_with("import ")
            || line.starts_with("use ")
            || line.ends_with('{')
            || line.ends_with(';')
    }) {
        return Content::Code;
    }
    if head.iter().any(|line| line.split_whitespace().count() >= 5) {
        return Content::Prose;
    }
    Content::Unknown
}

/// A log line is one that starts with something that is not prose: a
/// timestamp, a level, or a bracket. Counted rather than matched, so
/// one stray line does not decide.
fn looks_like_a_log_line(line: &str) -> bool {
    let line = line.trim_start();
    let starts_numeric = line.chars().next().is_some_and(|c| c.is_ascii_digit());
    starts_numeric
        || line.starts_with('[')
        || line.starts_with("INFO")
        || line.starts_with("WARN")
        || line.starts_with("ERROR")
        || line.starts_with("DEBUG")
}

/// What happens to that kind of content, at that size.
///
/// The size comparison lives here rather than at the call site, so that
/// "does it fit" and "what is done when it does not" are one decision
/// with one home.
#[must_use]
pub fn plan(content: Content, size: ByteLen, budget: ByteLen) -> Shrink {
    if size.get() <= budget.get() {
        return Shrink::Keep;
    }
    match content {
        Content::Structured => Shrink::MustOffload,
        Content::Log => Shrink::Cut(Strategy::Tail),
        Content::Code | Content::Diff => Shrink::Cut(Strategy::Ends),
        Content::Prose | Content::Table => Shrink::Cut(Strategy::Head),
        // Nothing is known about it, so nothing is thrown away on a
        // guess: it leaves the window whole and keeps its reference.
        Content::Unknown => Shrink::MustOffload,
    }
}

/// Carries out a shortening the caller has already decided on.
///
/// The answer carries the text, the source bytes it lost and which end
/// lost them, so no caller has to recover the loss by subtracting two
/// lengths — a subtraction that counted the inserted marker as surviving
/// text and under-reported the loss by its width.
///
/// Taking the strategy rather than deciding it again is what keeps this
/// module out of the business of what leaves the window: a caller
/// holding [`Shrink::MustOffload`] has nothing to call here.
#[must_use]
pub fn shorten(text: &str, strategy: Strategy, budget: ByteLen) -> Cut {
    let limit = usize::try_from(budget.get()).unwrap_or(usize::MAX);
    let cut = match strategy {
        Strategy::Head => keep_head(text, limit),
        Strategy::Tail => keep_tail(text, limit),
        Strategy::Ends => keep_ends(text, limit),
    };
    // The invariant, checked here rather than trusted: if a strategy
    // ever produced something longer, the input is returned instead. A
    // compactor that grows its input makes every budget meaningless.
    if cut.text.len() >= text.len() {
        return Cut::whole(text);
    }
    cut
}

fn keep_head(text: &str, limit: usize) -> Cut {
    let room = elision::marker_room(text.len());
    let front = elision::boundary_before(text, limit.saturating_sub(room));
    elision::splice(text, front, text.len(), Elided::Tail)
}

fn keep_tail(text: &str, limit: usize) -> Cut {
    let room = elision::marker_room(text.len());
    let keep = limit.saturating_sub(room);
    let back = elision::boundary_after(text, text.len().saturating_sub(keep));
    elision::splice(text, 0, back, Elided::Head)
}

fn keep_ends(text: &str, limit: usize) -> Cut {
    let room = elision::gap_marker_room(text.len());
    let each = limit.saturating_sub(room) / 2;
    let front = elision::boundary_before(text, each);
    let back = elision::boundary_after(text, text.len().saturating_sub(each));
    if back <= front {
        return keep_head(text, limit);
    }
    elision::splice(text, front, back, Elided::Middle)
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]
mod tests {
    use super::*;

    /// Both halves of one caller's work: decide, then carry it out. The
    /// invariants below hold of the pair, which is how a caller uses
    /// them, and a text this module declines to shorten comes back
    /// whole.
    fn compacted(text: &str, budget: ByteLen) -> Cut {
        let size = ByteLen::new(u64::try_from(text.len()).unwrap_or(u64::MAX));
        match plan(detect(text), size, budget) {
            Shrink::Keep | Shrink::MustOffload => Cut::whole(text),
            Shrink::Cut(strategy) => shorten(text, strategy, budget),
        }
    }

    #[test]
    fn the_same_input_is_dispatched_the_same_way_twice() {
        let samples = [
            "diff --git a/x b/x\n@@ -1 +1 @@\n-a\n+b\n",
            "{\"a\":1}",
            "| head | row |\n| --- | --- |\n| a | b |\n",
            "12:00:01 INFO started\n12:00:02 INFO finished\n",
            "fn main() {\n    let x = 1;\n}\n",
            "This is an ordinary sentence with more than five words in it.",
            "??",
        ];
        for sample in samples {
            assert_eq!(detect(sample), detect(sample));
        }
        assert_eq!(detect(samples[0]), Content::Diff);
        assert_eq!(detect(samples[1]), Content::Structured);
        assert_eq!(detect(samples[2]), Content::Table);
        assert_eq!(detect(samples[3]), Content::Log);
        assert_eq!(detect(samples[4]), Content::Code);
        assert_eq!(detect(samples[5]), Content::Prose);
        assert_eq!(detect(samples[6]), Content::Unknown);
    }

    #[test]
    fn shortening_never_lengthens() {
        let inputs = [
            String::new(),
            "a".to_owned(),
            "short".to_owned(),
            "x".repeat(4_000),
            "12:00:01 INFO a\n".repeat(300),
            "fn f() {\n  body();\n}\n".repeat(300),
            "{\"deep\":[1,2,3]}".repeat(300),
            "字".repeat(1_000),
        ];
        for budget in [0u64, 1, 8, 64, 512, 4_096] {
            for input in &inputs {
                let cut = compacted(input, ByteLen::new(budget));
                assert!(
                    cut.text.len() <= input.len(),
                    "budget {budget} grew an input of {} to {}",
                    input.len(),
                    cut.text.len()
                );
            }
        }
    }

    #[test]
    fn a_cut_lands_on_a_character_boundary() {
        // Every character here is three bytes, so a byte budget lands
        // mid-character unless the cut is moved.
        let text = format!("fn f() {{\n{}\n}}\n", "字".repeat(400));
        for budget in [7u64, 11, 100, 301] {
            let cut = compacted(&text, ByteLen::new(budget));
            assert!(
                std::str::from_utf8(cut.text.as_bytes()).is_ok(),
                "a cut mid-character produces bytes nothing downstream can read"
            );
        }
    }

    #[test]
    fn what_a_cut_reports_losing_is_what_the_reader_lost() {
        let log = (0..400)
            .map(|n| format!("12:00:{n:02} INFO line {n}\n"))
            .collect::<String>();
        let cut = compacted(&log, ByteLen::new(200));
        assert_eq!(cut.place, Elided::Head, "a log keeps its end");
        let marker = elision::marker(cut.dropped);
        let carried = cut.text.len().saturating_sub(marker.len());
        let dropped = usize::try_from(cut.dropped.get()).unwrap();
        assert_eq!(
            carried + dropped,
            log.len(),
            "the reported loss plus the surviving bytes is the input: {}",
            cut.text
        );
    }

    #[test]
    fn a_log_keeps_its_end_and_prose_keeps_its_start() {
        let log = (0..400)
            .map(|n| format!("12:00:{n:02} INFO line {n}\n"))
            .collect::<String>();
        let cut = compacted(&log, ByteLen::new(200));
        assert_eq!(cut.place, Elided::Head);
        assert!(
            cut.text.contains("line 399"),
            "the news is at the end of a log"
        );

        let prose = "The first sentence says what this is about. ".repeat(200);
        let cut = compacted(&prose, ByteLen::new(200));
        assert_eq!(cut.place, Elided::Tail);
        assert!(cut.text.starts_with("The first sentence"));
    }

    #[test]
    fn structured_and_unknown_are_never_truncated() {
        let json = format!("{{\"a\":[{}]}}", "1,".repeat(2_000));
        let size = ByteLen::new(u64::try_from(json.len()).unwrap());
        assert_eq!(
            plan(detect(&json), size, ByteLen::new(64)),
            Shrink::MustOffload,
            "truncated structured data looks parseable"
        );
        assert_eq!(
            plan(Content::Unknown, ByteLen::new(9_000), ByteLen::new(64)),
            Shrink::MustOffload,
            "nothing is thrown away on a guess"
        );
    }

    #[test]
    fn something_within_budget_is_left_exactly_alone() {
        let text = "a short result";
        let cut = compacted(text, ByteLen::new(4_096));
        assert_eq!(cut.place, Elided::Nothing);
        assert_eq!(cut.text, text);
    }
}
