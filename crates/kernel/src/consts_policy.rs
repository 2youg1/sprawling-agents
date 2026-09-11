// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Policy constants: our choices. Changing one
//! changes behavior and therefore requires EVAL evidence or an
//! explicit ruling. Data only — zero branches by charter.
//!
//! All fifteen policy entries are landed; three of them carry a type
//! rather than a plain number (kernel-SPEC 8-8): AUTONOMY_DEFAULT,
//! CLOCK_STAMP_DEFAULT, SUBAGENT_CTX_LOCK_DEFAULT.

/// Exact ratio as an integer pair: kernel decision paths never touch
/// floats (determinism rule 6). Kept unreduced so the spelling mirrors the
/// value as it was decided.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ratio {
    pub num: u32,
    pub den: u32,
}

pub const STARTUP_BUDGET_TOKENS: u64 = 2000;

/// 0.5: the past-half context reminder threshold.
pub const CTX_REMINDER_RATIO: Ratio = Ratio { num: 1, den: 2 };

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
/// Derived, not chosen: the context reminders fire at 25% and 65%, and a
/// ladder means nothing if one step can clear a rung unseen - skipping
/// the first needs a jump over 40%, the second over 35%, so one result
/// stays under 35% of the window. At 2.5 bytes per token, the worst
/// realistic ratio (base64, hash-dense text), 64 KiB is 26.2K tokens:
/// 20.5% of a 128K window, with 1.7x to spare. 128 KiB would be 41% and
/// out. Measured against this repository's 928 tracked text files, the
/// bytes in their first 512 lines run p50 6,275 / p95 16,055 / p99
/// 49,373 / max 73,978, so 64 KiB binds on one of them: it is a guard
/// against input that is not line-structured - a minified bundle, a
/// lockfile, a generated table - and not a tax on ordinary reading.
pub const INTERVAL_CAP_BYTES: u64 = 65_536;

pub const DRAFT_HELD_ESCALATE: u32 = 3;

pub const EDIT_WAR_FREEZE: u32 = 2;

/// 3.5 bits/char. Second constant scheduled for re-estimation; evidence =
/// false-positive rate on repository and fixture corpora (7.1).
pub const SECRET_ENTROPY_MIN: Ratio = Ratio { num: 7, den: 2 };

pub const DISCARD_FILES_MAX: u32 = 16;

pub const DISCARD_BYTES_MAX: u64 = 1_048_576;

pub const DISCARD_RETENTION_DAYS: u32 = 30;

pub const POLICY_IDLE_DAYS: u32 = 90;

pub const CLOCK_ZONES_MAX: u32 = 4;

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
pub const IMAGE_MAX_BYTES: u64 = 2_097_152;

/// Four pictures in one turn. Past that the window is being spent on
/// pixels rather than on the work, and a limit stated once is what a
/// refusal can name.
pub const IMAGES_PER_TURN: u32 = 4;

/// Off by default: zero window bytes until a Building opts in (4.3).
pub const CLOCK_STAMP_DEFAULT: crate::config::ClockStampGranularity =
    crate::config::ClockStampGranularity::Off;

/// The human answers by default; loosening is an explicit command.
pub const AUTONOMY_DEFAULT: crate::approval::Autonomy = crate::approval::Autonomy::Owner;

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
    fn the_twelve_landed_policies_hold_their_documented_values() {
        assert_eq!(STARTUP_BUDGET_TOKENS, 2000);
        assert_eq!(CTX_REMINDER_RATIO, Ratio { num: 1, den: 2 });
        assert_eq!(LOOP_REPEAT_THRESHOLD, 3);
        assert_eq!(OFFLOAD_MIN_BYTES, 16_384);
        assert_eq!(DRAFT_HELD_ESCALATE, 3);
        assert_eq!(EDIT_WAR_FREEZE, 2);
        assert_eq!(SECRET_ENTROPY_MIN, Ratio { num: 7, den: 2 });
        assert_eq!(DISCARD_FILES_MAX, 16);
        assert_eq!(DISCARD_BYTES_MAX, 1_048_576);
        assert_eq!(DISCARD_RETENTION_DAYS, 30);
        assert_eq!(POLICY_IDLE_DAYS, 90);
        assert_eq!(CLOCK_ZONES_MAX, 4);
        assert_eq!(SANDBOX_FUEL_DEFAULT, 200_000_000);
        assert_eq!(IMAGE_MAX_BYTES, 2_097_152);
        assert_eq!(IMAGES_PER_TURN, 4);
    }

    #[test]
    fn ratios_never_divide_by_zero() {
        for ratio in [CTX_REMINDER_RATIO, SECRET_ENTROPY_MIN] {
            assert_ne!(ratio.den, 0);
        }
    }
}
