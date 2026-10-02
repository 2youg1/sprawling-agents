// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Policy constants: our choices. Changing one
//! changes behavior and therefore requires EVAL evidence or an
//! explicit ruling. Data only — zero branches by charter.
//! The part `crates/kernel/spec/ConstsPolicy.lean` specifies this module.
//!
//! Five entries carry a type rather than a plain number (`crates/kernel/spec/ConstsPolicy.lean`
//! §8-8): AUTONOMY_DEFAULT, CLOCK_STAMP_DEFAULT, and the three limits
//! whose refusal derives from their type (`crates/kernel/spec/PolicyLimit.lean` §8-73).

/// Exact ratio as an integer pair: kernel decision paths never touch
/// floats (determinism rule 6). Kept unreduced so the spelling mirrors the
/// value as it was decided.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ratio {
    pub num: u32,
    pub den: u32,
}

pub const STARTUP_BUDGET_TOKENS: u64 = 2000;

/// Bytes per token, typical: the rate every budget that crosses the two
/// units is converted at. It is an estimate for English prose and code,
/// and it is deliberately not the worst-case ratio
/// [`INTERVAL_CAP_BYTES`] reasons from - a budget that has to hold is
/// sized by the worst case, a budget that has to be spent is sized by
/// the typical one.
pub const BYTES_PER_TOKEN: u64 = 4;

/// How many slots a whole-prefix budget divides evenly across.
/// `runtime::prefix::SegmentSlot` is the authority on how many slots
/// there are, and `runtime::prefix::tests` holds this divisor against
/// that enum so the two cannot drift apart.
pub const PREFIX_SLOTS: std::num::NonZeroU64 = match std::num::NonZeroU64::new(4) {
    Some(slots) => slots,
    None => std::num::NonZeroU64::MIN,
};

/// Where the context reminder's two rungs sit, as whole percents of the
/// window. The first is not adjustable: one number, one meaning. The
/// second is the one a layer may move, and these three numbers are the
/// whole of what it may move it between — `kernel::config::SecondThreshold`
/// enforces the domain at its one construction point.
pub const CTX_REMINDER_FIRST_PERCENT: u64 = 25;
pub const CTX_REMINDER_SECOND_DEFAULT: u64 = 65;
pub const CTX_REMINDER_SECOND_MIN: u64 = 30;
pub const CTX_REMINDER_SECOND_MAX: u64 = 90;

pub const LOOP_REPEAT_THRESHOLD: u32 = 3;

/// Floor, not threshold: the effective offload bound is derived by
/// `pipeline` from window headroom (4.4).
pub const OFFLOAD_MIN_BYTES: u64 = 16_384;

/// What one interval of a document may carry into the window: 64 KiB,
/// cut on a line end, continued by the offset the answer reports.
///
/// A different subject from a command's cap, which is why it is a
/// different number. `exec` output cannot be continued, so its ceiling
/// is where loss becomes acceptable and the remainder goes to CAS.
/// `read` and `search` hand back an interval of something the caller can
/// ask for again, so their ceiling only paces delivery.
///
/// Derived, not chosen: the context reminders sound at 25% and at the
/// second rung — `CTX_REMINDER_SECOND_DEFAULT` unless a layer moves it
/// between `CTX_REMINDER_SECOND_MIN` and `CTX_REMINDER_SECOND_MAX` — and
/// a ladder means nothing if one result can clear a rung unseen. The gaps
/// one result must not cover are the second rung minus 25 into the first
/// rung, and 100 minus the second rung past the last: 40 and 35 points at
/// the default pair, so one result stays under 35% of the window. At 2.5
/// bytes per token, the worst realistic ratio (base64, hash-dense text),
/// 64 KiB is 26.2K tokens: 20.5% of a 128K window, with 1.7x to spare.
/// 128 KiB would be 41% and out. A rung moved off the default narrows its
/// gap below that: under 60 one result can spend the first rung unseen,
/// and over 65 one result can fill the window past the second while its
/// "still enough to write a handoff" claim is still fixed text — which is
/// the arithmetic the 90 ceiling of the legal domain is priced from.
/// Measured against this repository's 928 tracked text files, the
/// bytes in their first 512 lines run p50 6,275 / p95 16,055 / p99
/// 49,373 / max 73,978, so 64 KiB binds on one of them: it is a guard
/// against input that is not line-structured - a minified bundle, a
/// lockfile, a generated table - and not a tax on ordinary reading.
///
/// A `usize` because both readers compare it with the length of a text
/// in memory: with no conversion to write, there is no failed one to
/// read as "no ceiling at all".
pub const INTERVAL_CAP_BYTES: usize = 65_536;

/// What one turn's exchange — its assistant reply and its wave's
/// results — may occupy in the window, in bytes. Shared evenly across
/// the exchange's texts when the whole does not fit; which end of a text
/// survives is `runtime::compaction`'s decision, and this file only says
/// how much there is.
///
/// Derived, not chosen: one reply at the city's default output ceiling,
/// converted at the rate every two-unit budget crosses at. A reply is
/// the one part of an exchange no landing-time mechanism has bounded
/// yet — a tool result is capped where it lands and its original is
/// pinned first — so the exchange's budget is priced at exactly the
/// largest thing the model is allowed to say. The `checked_mul` is
/// const-evaluated over two literals and cannot fail; the match is the
/// const-safe spelling of that product.
pub const EXCHANGE_BUDGET_BYTES: u64 = match OUTPUT_CEILING_DEFAULT.checked_mul(BYTES_PER_TOKEN) {
    Some(bytes) => bytes,
    None => BYTES_PER_TOKEN,
};

pub const DRAFT_HELD_ESCALATE: u32 = 3;

pub const EDIT_WAR_FREEZE: u32 = 2;

/// 3.5 bits/char. Second constant scheduled for re-estimation; evidence =
/// false-positive rate on repository and fixture corpora.
pub const SECRET_ENTROPY_MIN: Ratio = Ratio { num: 7, den: 2 };

pub const DISCARD_FILES_MAX: u32 = 16;

pub const DISCARD_RETENTION_DAYS: u32 = 30;

pub const CLOCK_ZONES_MAX: crate::policy_limit::ClockZonesMax =
    crate::policy_limit::ClockZonesMax::new(4);

/// Instruction budget for one sandboxed call when no layer states one.
/// Large enough that ordinary work finishes, small enough that a loop
/// stops rather than runs until somebody notices: the point of
/// fuel is that exhaustion is a verdict the city writes down, not a
/// machine that gets slow.
pub const SANDBOX_FUEL_DEFAULT: u64 = 200_000_000;

/// Words that make an environment variable name read as a credential
/// (11.1). Matched as a case-insensitive substring, so `AWS_SECRET_KEY`
/// and `npm_password` are both caught.
///
/// Deliberately over-inclusive: `KEYBOARD` is refused along with
/// `API_KEY`, and that is the direction to err in. A refused name comes
/// back as a three-part refusal a person can act on; an admitted one is
/// inherited by a child process that cannot be asked to forget it.
pub const CREDENTIAL_NAME_MARKERS: [&str; 11] = [
    "secret",
    "token",
    "key",
    "password",
    "passwd",
    "credential",
    "auth",
    "session",
    "cookie",
    "private",
    "signature",
];

/// 2 GiB per node's working tree. A ceiling rather than a free-space
/// probe: free space is a moving fact about one machine, while this is
/// the number a refusal can state and a person can raise. Checked
/// before a tree is created, so an over-large city is refused rather
/// than half-copied (11.4).
pub const WORKTREE_MAX_BYTES: u64 = 2_147_483_648;

/// 2 MiB per picture. A ceiling a refusal can state and a person can
/// raise, sized so a base64 body (4/3 of this) stays inside what both
/// providers accept.
pub const IMAGE_MAX_BYTES: crate::policy_limit::ImageMaxBytes =
    crate::policy_limit::ImageMaxBytes::new(2_097_152);

/// Four pictures in one turn. Past that the window is being spent on
/// pixels rather than on the work, and a limit stated once is what a
/// refusal can name.
pub const IMAGES_PER_TURN: crate::policy_limit::ImagesPerTurn =
    crate::policy_limit::ImagesPerTurn::new(4);

/// 100 is the whole domain a lossy encoder has. Refused rather than
/// clamped where a caller names one: `quality: 120` is a caller that
/// believes it asked for something better, and an answer clamped to 100
/// would agree with it.
pub const IMAGE_QUALITY: crate::policy_limit::ImageQuality =
    crate::policy_limit::ImageQuality::new(100);

/// How long one answer may be on the messages face when nobody has said:
/// 8_192 tokens.
///
/// The last rung of the output-ceiling ladder on the one face that needs
/// a figure in every request, reached only when the person stated no
/// ceiling, the provider's model list stated none, and the preset table
/// has no row for the model. The three rungs above it carry real
/// statements, so this number is never a figure that outranks one
/// somebody made. The chat and responses faces never reach it: there a
/// ceiling nobody stated is left to the provider (`crates/gateway/Spec.lean` §8-17).
///
/// Chosen at the width every provider this city calls accepts for every
/// model it serves, which is what a number used in place of knowledge
/// has to be. It truncates a long answer rather than refusing the call,
/// and `model_selected` records `ceiling_from: policy` so a truncated
/// run is read off the account rather than guessed at.
pub const OUTPUT_CEILING_DEFAULT: u64 = 8_192;

/// Every `Timestamped` result carries the clock line, and a `Timeless`
/// one at most once a minute, until a layer writes `[clock] stamp`: a
/// model that cannot tell when a command ran cannot tell a stale answer
/// from a fresh one (runtime D8).
pub const CLOCK_STAMP_DEFAULT: crate::config::ClockStampGranularity =
    crate::config::ClockStampGranularity::Minute;

/// Who reads the Approval Inbox in a city nobody has configured: the
/// person, because the inbox holds design questions and a design
/// question is the kind a person wants. What a run may *do* is settled
/// by the gates, which answer from the rules and never ask.
pub const AUTONOMY_DEFAULT: crate::approval::Autonomy = crate::approval::Autonomy::Owner;

/// Where a city listens when nobody says otherwise: the loopback
/// interface, so a city started by a double-click is reachable from the
/// browser on that machine and from nowhere else.
///
/// `up`, `serve`, the first screen, and every command that talks to a
/// served city read this one value, and the installer script and README
/// quote it, so the address a person is told is the address that binds.
pub const DEFAULT_AT: &str = "127.0.0.1:8787";

/// The building raised with every city, which holds the city's own plan
/// and the two residents that serve every other building.
pub const HALL_BUILDING: &str = "hall";

/// The city's planner. It writes Markdown and plans; it does not build.
pub const HALL_MAYOR: &str = "hall/mayor";

/// Who answers approvals when the person delegated them. Genesis writes
/// the delegation as an event rather than baking it into
/// `AUTONOMY_DEFAULT`: who answers is a decision this city made, and a
/// decision the person can change needs a line of history to change.
pub const HALL_CLERK: &str = "hall/clerk";

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests {
    use super::*;

    #[test]
    fn the_landed_policies_hold_their_documented_values() {
        assert_eq!(STARTUP_BUDGET_TOKENS, 2000);
        assert_eq!(CTX_REMINDER_FIRST_PERCENT, 30);
        assert_eq!(CTX_REMINDER_SECOND_DEFAULT, 65);
        assert_eq!(CTX_REMINDER_SECOND_MIN, 31);
        assert_eq!(CTX_REMINDER_SECOND_MAX, 90);
        assert_eq!(LOOP_REPEAT_THRESHOLD, 3);
        assert_eq!(OFFLOAD_MIN_BYTES, 16_384);
        assert_eq!(DRAFT_HELD_ESCALATE, 3);
        assert_eq!(EDIT_WAR_FREEZE, 2);
        assert_eq!(SECRET_ENTROPY_MIN, Ratio { num: 7, den: 2 });
        assert_eq!(DISCARD_FILES_MAX, 16);
        assert_eq!(DISCARD_RETENTION_DAYS, 30);
        assert_eq!(CLOCK_ZONES_MAX, crate::policy_limit::ClockZonesMax::new(4));
        assert_eq!(SANDBOX_FUEL_DEFAULT, 200_000_000);
        assert_eq!(
            IMAGE_MAX_BYTES,
            crate::policy_limit::ImageMaxBytes::new(2_097_152)
        );
        assert_eq!(IMAGES_PER_TURN, crate::policy_limit::ImagesPerTurn::new(4));
        assert_eq!(OUTPUT_CEILING_DEFAULT, 8_192);
    }

    /// Zero is not a ceiling: the Anthropic wire refuses it and the
    /// OpenAI wire sends `max_tokens: 0`, which answers with no content
    /// at all. The ladder's last rung has to be a number that calls.
    #[test]
    fn the_last_rung_of_the_ceiling_ladder_is_a_ceiling() {
        const { assert!(OUTPUT_CEILING_DEFAULT > 0) };
    }

    #[test]
    fn ratios_never_divide_by_zero() {
        const { assert!(SECRET_ENTROPY_MIN.den > 0) };
    }

    /// The default sits inside the domain the one construction point
    /// enforces, or a run with no configured layer would have no legal
    /// rung.
    #[test]
    fn the_default_second_rung_sits_inside_its_domain() {
        const { assert!(CTX_REMINDER_SECOND_MIN <= CTX_REMINDER_SECOND_DEFAULT) };
        const { assert!(CTX_REMINDER_SECOND_DEFAULT <= CTX_REMINDER_SECOND_MAX) };
    }
}
