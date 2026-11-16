// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ratio {
    pub num: u32,
    pub den: u32,
}

pub const STARTUP_BUDGET_TOKENS: u64 = 2000;

pub const BYTES_PER_TOKEN: u64 = 4;

pub const PREFIX_SLOTS: std::num::NonZeroU64 = match std::num::NonZeroU64::new(4) {
    Some(slots) => slots,
    None => std::num::NonZeroU64::MIN,
};

pub const CTX_REMINDER_FIRST_PERCENT: u64 = 30;
pub const CTX_REMINDER_SECOND_DEFAULT: u64 = 65;
pub const CTX_REMINDER_SECOND_MIN: u64 = match CTX_REMINDER_FIRST_PERCENT.checked_add(1) {
    Some(floor) => floor,
    None => u64::MAX,
};
pub const CTX_REMINDER_SECOND_MAX: u64 = 90;

pub const LOOP_REPEAT_THRESHOLD: u32 = 3;

pub const OFFLOAD_MIN_BYTES: u64 = 16_384;

pub const INTERVAL_CAP_BYTES: usize = 65_536;

pub const EXCHANGE_BUDGET_BYTES: u64 = match OUTPUT_CEILING_DEFAULT.checked_mul(BYTES_PER_TOKEN) {
    Some(bytes) => bytes,
    None => BYTES_PER_TOKEN,
};

pub const DRAFT_HELD_ESCALATE: u32 = 3;

pub const EDIT_WAR_FREEZE: u32 = 2;

pub const SECRET_ENTROPY_MIN: Ratio = Ratio { num: 7, den: 2 };

pub const DISCARD_FILES_MAX: u32 = 16;

pub const DISCARD_RETENTION_DAYS: u32 = 30;

pub const CLOCK_ZONES_MAX: crate::policy_limit::ClockZonesMax =
    crate::policy_limit::ClockZonesMax::new(4);

pub const SANDBOX_FUEL_DEFAULT: u64 = 200_000_000;

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

pub const WORKTREE_MAX_BYTES: u64 = 2_147_483_648;

pub const IMAGE_MAX_BYTES: crate::policy_limit::ImageMaxBytes =
    crate::policy_limit::ImageMaxBytes::new(4_194_304);

pub const IMAGES_PER_TURN: crate::policy_limit::ImagesPerTurn =
    crate::policy_limit::ImagesPerTurn::new(4);

pub const IMAGE_QUALITY: crate::policy_limit::ImageQuality =
    crate::policy_limit::ImageQuality::new(100);

pub const OUTPUT_CEILING_DEFAULT: u64 = 8_192;

pub const CLOCK_STAMP_DEFAULT: crate::config::ClockStampGranularity =
    crate::config::ClockStampGranularity::Minute;

pub const AUTONOMY_DEFAULT: crate::approval::Autonomy = crate::approval::Autonomy::Owner;

pub const DEFAULT_EFFORT: crate::model::Effort = crate::model::Effort::High;

pub const DEFAULT_AT: &str = "127.0.0.1:2501";

pub const HALL_BUILDING: &str = "hall";

pub const HALL_MAYOR: &str = "hall/mayor";

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
            crate::policy_limit::ImageMaxBytes::new(4_194_304)
        );
        assert_eq!(IMAGES_PER_TURN, crate::policy_limit::ImagesPerTurn::new(4));
        assert_eq!(OUTPUT_CEILING_DEFAULT, 8_192);
    }

    #[test]
    fn the_last_rung_of_the_ceiling_ladder_is_a_ceiling() {
        const { assert!(OUTPUT_CEILING_DEFAULT > 0) };
    }

    #[test]
    fn ratios_never_divide_by_zero() {
        const { assert!(SECRET_ENTROPY_MIN.den > 0) };
    }

    #[test]
    fn the_default_second_rung_sits_inside_its_domain() {
        const { assert!(CTX_REMINDER_SECOND_MIN <= CTX_REMINDER_SECOND_DEFAULT) };
        const { assert!(CTX_REMINDER_SECOND_DEFAULT <= CTX_REMINDER_SECOND_MAX) };
    }
}
