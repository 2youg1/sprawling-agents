// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use serde::{Deserialize, Serialize};

use crate::error::{AxCode, AxError};

mod number;
pub use number::Version;

pub const AUR_PACKAGE_NAME: &str = "sprawling-bin";

#[must_use]
pub fn aur_install_directory() -> String {
    format!("usr/lib/{AUR_PACKAGE_NAME}")
}

#[must_use]
pub fn is_aur_install(exe: &std::path::Path) -> bool {
    exe.parent()
        .is_some_and(|directory| directory.ends_with(aur_install_directory()))
}

const NPM_INFIX: &str = "-pre.";

const CENTURY: u32 = 2000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Release {
    version: Version,

    year: u32,
    month: u32,
    day: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum ReleaseVerdict {
    Current,

    Behind,

    Ahead,
}

impl Release {
    pub fn from_tag(tag: &str, expected_version: &str) -> Result<Release, AxError> {
        let (version, date) = Self::current_parts(tag).ok_or_else(|| {
            AxError::failure(AxCode::ConfigInvalid, "read a release tag", tag.to_owned())
                .with_recovery(
                    "a release tag of this build reads `v<version>-<YYMMDD>`; \
                     another shape is not one of its releases",
                )
        })?;
        if version != expected_version {
            return Err(AxError::failure(
                AxCode::ConfigInvalid,
                "read a release tag",
                tag.to_owned(),
            )
            .with_recovery(format!(
                "the tag says {version} and the build says {expected_version}; \
                 one release cannot have two version numbers"
            )));
        }
        Release::assemble(version, date, tag, "read a release tag")
    }

    fn current_parts(tag: &str) -> Option<(&str, &str)> {
        tag.strip_prefix('v')?.split_once('-')
    }

    fn legacy_parts(tag: &str) -> Option<(&str, &str)> {
        let (version, rest) = tag.strip_prefix('v')?.split_once('-')?;
        if !version.starts_with("0.0.") {
            return None;
        }
        let (stage, date) = rest.rsplit_once('-')?;
        if stage.is_empty() {
            return None;
        }
        Some((version, date))
    }

    pub fn from_published_tag(tag: &str) -> Result<Release, AxError> {
        let parts = if tag.starts_with("v0.0.") {
            Self::legacy_parts(tag)
        } else {
            Self::current_parts(tag)
        };
        let Some((version, date)) = parts else {
            return Err(AxError::failure(
                AxCode::ConfigInvalid,
                "read a published tag",
                tag.to_owned(),
            )
            .with_recovery("use a published current or historical release tag"));
        };
        Release::assemble(version, date, tag, "read a published tag")
    }

    pub fn from_npm_version(text: &str) -> Result<Release, AxError> {
        let (version, date) = text.split_once(NPM_INFIX).ok_or_else(|| {
            AxError::failure(
                AxCode::ConfigInvalid,
                "read an npm version",
                text.to_owned(),
            )
            .with_recovery(
                "every release of this project is published as \
                 `<version>-pre.<YYMMDD>`; a version of another shape was \
                 published by something other than this workflow",
            )
        })?;
        Release::assemble(version, date, text, "read an npm version")
    }

    fn assemble(
        version: &str,
        date: &str,
        subject: &str,
        action: &'static str,
    ) -> Result<Release, AxError> {
        let refused = |msg: String| {
            AxError::failure(AxCode::ConfigInvalid, action, subject.to_owned()).with_recovery(msg)
        };
        let version = Version::parse(version, &refused)?;
        let mut digits = date.chars();
        let (Some(year), Some(month), Some(day)) =
            (pair(&mut digits), pair(&mut digits), pair(&mut digits))
        else {
            return Err(refused(format!(
                "the date reads `{date}`, which is not six digits"
            )));
        };
        if digits.next().is_some() {
            return Err(refused(format!(
                "the date reads `{date}`, which is longer than six digits"
            )));
        }
        let year = year
            .checked_add(CENTURY)
            .ok_or_else(|| refused(format!("the year in `{date}` does not fit")))?;
        if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
            return Err(refused(format!(
                "the date reads `{date}`, and `{month:02}-{day:02}` is not a day of any month"
            )));
        }
        Ok(Release {
            version,
            year,
            month,
            day,
        })
    }

    #[must_use]
    pub fn version(&self) -> String {
        self.version.to_string()
    }

    #[must_use]
    pub fn tag(&self) -> String {
        let Release {
            year, month, day, ..
        } = *self;
        let short = year.checked_sub(CENTURY).unwrap_or(year);
        format!("v{}-{short:02}{month:02}{day:02}", self.version())
    }

    #[must_use]
    pub fn npm_version(&self) -> String {
        let Release {
            year, month, day, ..
        } = *self;
        let short = year.checked_sub(CENTURY).unwrap_or(year);
        format!("{}{NPM_INFIX}{short:02}{month:02}{day:02}", self.version())
    }

    #[must_use]
    pub fn released(&self) -> String {
        let Release {
            year, month, day, ..
        } = *self;
        format!("{year:04}-{month:02}-{day:02}")
    }
}

#[must_use]
pub fn stands(mine: &Release, newest: &Release) -> ReleaseVerdict {
    verdict_of(mine.cmp(newest))
}

#[must_use]
pub fn stands_on_crates(mine: &Release, newest: &Version) -> ReleaseVerdict {
    stands_on_versions(&mine.version, newest)
}

#[must_use]
pub fn stands_on_versions(mine: &Version, newest: &Version) -> ReleaseVerdict {
    verdict_of(mine.cmp(newest))
}

fn verdict_of(order: std::cmp::Ordering) -> ReleaseVerdict {
    match order {
        std::cmp::Ordering::Less => ReleaseVerdict::Behind,
        std::cmp::Ordering::Equal => ReleaseVerdict::Current,
        std::cmp::Ordering::Greater => ReleaseVerdict::Ahead,
    }
}

fn pair(digits: &mut impl Iterator<Item = char>) -> Option<u32> {
    let tens = digits.next()?.to_digit(10)?;
    let ones = digits.next()?.to_digit(10)?;
    tens.checked_mul(10)?.checked_add(ones)
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "test code"
)]
mod tests {
    use super::{Release, ReleaseVerdict, Version, stands, stands_on_crates};
    use proptest::prelude::*;

    fn any_release() -> impl Strategy<Value = Release> {
        (
            0u32..=9,
            0u32..=9,
            0u32..=9,
            0u32..=99,
            1u32..=12,
            1u32..=31,
        )
            .prop_map(|(maturity, repository, package, year, month, day)| {
                Release::from_npm_version(&format!(
                    "{maturity}.{repository}.{package}-pre.{year:02}{month:02}{day:02}"
                ))
                .unwrap()
            })
    }

    fn semver_key(npm: &str) -> (u32, u32, u32, u32) {
        let (core, pre) = npm.split_once("-pre.").unwrap();
        let mut numbers = core.split('.').map(|part| part.parse::<u32>().unwrap());
        let mut next = || numbers.next().unwrap();
        (next(), next(), next(), pre.parse().unwrap())
    }

    proptest! {

        #[test]
        fn the_three_spellings_give_one_order(
            a in any_release(),
            other in any_release(),
            same_version in any::<bool>(),
        ) {

            let b = if same_version {
                Release { version: a.version, ..other }
            } else {
                other
            };
            let crates = Version::from_crates_version(&b.version()).unwrap();
            if a.version == b.version {
                prop_assert_eq!(stands_on_crates(&a, &crates), ReleaseVerdict::Current);
            } else {
                prop_assert_eq!(stands_on_crates(&a, &crates), stands(&a, &b));
            }
            prop_assert_eq!(
                semver_key(&a.npm_version()).cmp(&semver_key(&b.npm_version())),
                a.cmp(&b)
            );
        }
    }

    #[test]
    fn a_crates_answer_of_the_same_release_reads_as_current() {
        let mine = Release::from_npm_version("0.0.8-pre.261002").unwrap();
        let newest = Version::from_crates_version("0.0.8").unwrap();
        assert_eq!(stands_on_crates(&mine, &newest), ReleaseVerdict::Current);
        assert!(Version::from_crates_version("0.0.8-pre.261002").is_err());
    }

    #[test]
    fn a_build_cuts_and_reads_current_tags() {
        let cut = Release::from_tag("v0.17.0-270101", "0.17.0").unwrap();
        assert_eq!(cut.tag(), "v0.17.0-270101");
        assert!(Release::from_tag("v0.17.0-extra-270101", "0.17.0").is_err());
    }

    #[test]
    fn a_release_tag_becomes_the_semver_npm_accepts() {
        let cut = Release::from_tag("v0.17.0-261115", "0.17.0").unwrap();
        assert_eq!(cut.npm_version(), "0.17.0-pre.261115");
        assert_eq!(cut.version(), "0.17.0");
        assert_eq!(cut.released(), "2026-11-15");
    }

    #[test]
    fn both_spellings_round_trip_through_one_another() {
        let tag = "v0.17.0-261115";
        let cut = Release::from_tag(tag, "0.17.0").unwrap();
        assert_eq!(cut.tag(), tag);
        assert_eq!(Release::from_published_tag(tag).unwrap(), cut);
        assert_eq!(Release::from_npm_version(&cut.npm_version()).unwrap(), cut);
    }

    proptest! {

        #[test]
        fn a_release_reads_back_from_the_tag_it_cuts(cut in any_release()) {
            let read = Release::from_tag(&cut.tag(), &cut.version()).unwrap();
            prop_assert_eq!(read, cut);
        }
    }

    #[test]
    fn historical_tags_remain_readable_without_reusing_their_text() {
        for (tag, version, released) in [
            ("v0.0.5-Pre-alpha-260912", "0.0.5", "2026-09-12"),
            ("v0.0.17-Alpha-261115", "0.0.17", "2026-11-15"),
        ] {
            let cut = Release::from_published_tag(tag).unwrap();
            assert_eq!(cut.version(), version);
            assert_eq!(cut.released(), released);
            let date = tag.rsplit_once('-').unwrap().1;
            assert_eq!(cut.tag(), format!("v{version}-{date}"));
        }
    }

    #[test]
    fn a_version_part_of_several_digits_reads_as_its_number() {
        assert_eq!(
            Version::from_crates_version("10.12.30")
                .unwrap()
                .to_string(),
            "10.12.30"
        );
        assert!(Version::from_crates_version("0.012.0").is_err());
    }

    #[test]
    fn a_tag_disagreeing_with_the_build_is_refused() {
        let err = Release::from_tag("v0.17.0-260911", "0.17.1").unwrap_err();
        assert!(err.recovery().contains("two version numbers"), "{err:?}");
    }

    #[test]
    fn a_tag_of_another_shape_is_refused_rather_than_guessed_at() {
        for refused in [
            "0.0.4",
            "v0.0.4",
            "v0.17.0-26091",
            "v0.17.0-2609111",
            "v0.17.0-2609xx",
            "v0.17.0-261301",
            "v0.17.0-260900",
            "v0.17.0.1-260911",
            "v0.017.0-260911",
        ] {
            assert!(
                Release::from_tag(refused, "0.17.0").is_err(),
                "{refused} was accepted"
            );
        }
    }

    #[test]
    fn the_date_decides_between_two_releases_of_one_version() {
        let older = Release::from_npm_version("0.0.5-pre.260911").unwrap();
        let newer = Release::from_npm_version("0.0.5-pre.260912").unwrap();
        assert_eq!(stands(&older, &newer), ReleaseVerdict::Behind);
        assert_eq!(stands(&newer, &older), ReleaseVerdict::Ahead);
        assert_eq!(stands(&newer, &newer), ReleaseVerdict::Current);
    }

    #[test]
    fn the_version_outranks_the_date() {
        let old_version_late = Release::from_npm_version("0.0.5-pre.261231").unwrap();
        let new_version_early = Release::from_npm_version("0.0.6-pre.260101").unwrap();
        assert_eq!(
            stands(&old_version_late, &new_version_early),
            ReleaseVerdict::Behind
        );
    }

    #[test]
    fn an_npm_version_this_project_does_not_publish_is_refused() {
        for refused in ["0.17.0", "0.17.0-test.1", "0.17.0-pre.", "latest"] {
            assert!(
                Release::from_npm_version(refused).is_err(),
                "{refused} was accepted"
            );
        }
    }
}
