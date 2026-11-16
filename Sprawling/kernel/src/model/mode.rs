// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use serde::{Deserialize, Serialize};

use crate::write_domain::WriteLimit;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Mode {

    Chat,

    Work,
}

impl Mode {

    pub const ALL: [Mode; 2] = [Mode::Chat, Mode::Work];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Mode::Chat => "chat",
            Mode::Work => "work",
        }
    }
}

impl std::fmt::Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum AdmissionRequirement {

    Standing,

    Tested,

    ContractKept,

    DoubleValidated,
}

impl AdmissionRequirement {

    pub const ALL: [AdmissionRequirement; 4] = [
        AdmissionRequirement::Standing,
        AdmissionRequirement::Tested,
        AdmissionRequirement::ContractKept,
        AdmissionRequirement::DoubleValidated,
    ];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            AdmissionRequirement::Standing => "standing",
            AdmissionRequirement::Tested => "tested",
            AdmissionRequirement::ContractKept => "contract_kept",
            AdmissionRequirement::DoubleValidated => "double_validated",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum LandingPolicy {

    Ordinary,

    Experiment,
}

impl LandingPolicy {

    pub const ALL: [LandingPolicy; 2] = [LandingPolicy::Ordinary, LandingPolicy::Experiment];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            LandingPolicy::Ordinary => "ordinary",
            LandingPolicy::Experiment => "experiment",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct RunPolicy {
    pub mode: Mode,

    pub write: WriteLimit,

    pub admit: AdmissionRequirement,

    pub landing: LandingPolicy,
}

impl RunPolicy {

    #[must_use]
    pub const fn of(mode: Mode) -> RunPolicy {
        RunPolicy {
            mode,
            write: WriteLimit::Full,
            admit: AdmissionRequirement::Standing,
            landing: LandingPolicy::Ordinary,
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, reason = "test code")]
mod tests {
    use super::*;

    #[test]
    fn every_value_reads_back_from_the_word_it_writes() {
        fn round_trips<T>(all: &[T], word: fn(T) -> &'static str)
        where
            T: Copy + PartialEq + std::fmt::Debug + Serialize + for<'de> Deserialize<'de>,
        {
            for value in all {
                let text = serde_json::to_string(value).unwrap();
                assert_eq!(text, format!("\"{}\"", word(*value)));
                assert_eq!(serde_json::from_str::<T>(&text).unwrap(), *value);
            }
        }
        round_trips(&Mode::ALL, Mode::as_str);
        round_trips(&AdmissionRequirement::ALL, AdmissionRequirement::as_str);
        round_trips(&LandingPolicy::ALL, LandingPolicy::as_str);
        round_trips(&WriteLimit::ALL, WriteLimit::as_str);
    }

    #[test]
    fn a_word_outside_the_set_is_refused() {
        assert!(serde_json::from_str::<Mode>("\"a-mode-we-have-never-heard-of\"").is_err());
        assert!(serde_json::from_str::<Mode>("\"plan_goal\"").is_err());
        assert!(serde_json::from_str::<AdmissionRequirement>("\"none\"").is_err());
        assert!(serde_json::from_str::<LandingPolicy>("\"merge\"").is_err());
    }

    #[test]
    fn a_policy_missing_a_value_is_refused() {
        let whole: RunPolicy = serde_json::from_str(
            r#"{"mode":"work","write":"create","admit":"tested","landing":"experiment"}"#,
        )
        .unwrap();
        assert_eq!(
            whole,
            RunPolicy {
                mode: Mode::Work,
                write: WriteLimit::Create,
                admit: AdmissionRequirement::Tested,
                landing: LandingPolicy::Experiment,
            }
        );
        assert!(
            serde_json::from_str::<RunPolicy>(r#"{"mode":"work","write":"full","admit":"tested"}"#)
                .is_err()
        );
    }
}
