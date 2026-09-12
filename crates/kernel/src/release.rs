// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Which release a binary is, and how two releases order.
//!
//! **One release is spelled two ways, and this is the only place that
//! knows both.** A git tag reads `v0.0.5-Pre-alpha-260912`; npm accepts
//! semver and nothing else, so the same release reaches the registry as
//! `0.0.5-pre.260912`. `xtask channel` converts in order to publish, and
//! a running binary converts in order to ask the registry which release
//! is newest. Either conversion written a second time would be a second
//! answer to "which release is this".
//!
//! **The order is semver's own.** Putting the date in the pre-release
//! field ranks `0.0.5-pre.260912` above `0.0.5-pre.260911` and below a
//! plain `0.0.5`, because semver compares dot-separated numeric
//! identifiers numerically. Deriving `Ord` over these fields in
//! declaration order reproduces that rule, so this crate and the
//! registry sort one pair of releases the same way. A binary that
//! compared its own bare `0.0.5` against `0.0.5-pre.260912` would read
//! itself as the newer of the two, which is the reason the date has to
//! travel with the version rather than beside it.
//!
//! No clock and no socket. What the registry currently offers is the
//! caller's to fetch; this judges only what it is handed
//! (ARCHITECTURE.md paragraph 1).

use serde::{Deserialize, Serialize};

use crate::error::{AxCode, AxError};

/// What separates the version from the date in a git tag.
const TAG_INFIX: &str = "-Pre-alpha-";

/// What separates them in the version npm carries.
const NPM_INFIX: &str = "-pre.";

/// The century the two-digit year in a tag belongs to. A pre-alpha that
/// is still being released in 2100 has larger problems than this
/// constant, and a tag cannot carry four digits without changing the
/// name of every release already published.
const CENTURY: u32 = 2000;

/// One published release of this project, in the form both spellings
/// decode to.
///
/// Field order is the comparison order, and the comparison order is
/// semver's: version first, then the date inside the pre-release field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Release {
    major: u32,
    minor: u32,
    patch: u32,
    /// Four digits. The tag carries two and this holds what they mean,
    /// so a reader is never asked whether `26` is a year or a week.
    year: u32,
    month: u32,
    day: u32,
}

/// Where one release stands against the newest one published.
///
/// Three states rather than a bool, because the third is reachable: the
/// workflow publishes the GitHub release before it publishes the npm
/// packages, so a binary downloaded from the release page is newer than
/// the registry for as long as that job takes.
///
/// Named for what it judges rather than `Standing`, which `channels`
/// already spends on where a toolkit stands: both cross the same wire,
/// and the generated client gives one name to one type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum ReleaseVerdict {
    /// The newest published release is this one.
    Current,
    /// Something newer is published.
    Behind,
    /// This release is newer than anything published.
    Ahead,
}

impl Release {
    /// The release a git tag names.
    ///
    /// `expected_version` is the version the caller was built from -
    /// `workspace.package.version` for `xtask`, `CARGO_PKG_VERSION` for
    /// the binary. Both callers hold the same number, and a tag that
    /// disagrees with it is refused here rather than producing a release
    /// whose name and whose contents state different versions.
    ///
    /// # Errors
    /// When the tag is not shaped like a release of this project, when
    /// its version disagrees with `expected_version`, or when its date
    /// is not a real day.
    pub fn from_tag(tag: &str, expected_version: &str) -> Result<Release, AxError> {
        let refused = |msg: &str| {
            AxError::failure(AxCode::ConfigInvalid, "read a release tag", tag.to_owned())
                .with_recovery(msg.to_owned())
        };
        let body = tag.strip_prefix('v').ok_or_else(|| {
            refused("a release tag begins with `v`, as in `v0.0.5-Pre-alpha-260912`")
        })?;
        let (version, date) = body.split_once(TAG_INFIX).ok_or_else(|| {
            refused(
                "a release tag reads `v<version>-Pre-alpha-<YYMMDD>`; \
                 a tag of another shape needs this rule written for it",
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

    /// The release an npm version string names.
    ///
    /// # Errors
    /// When the string is not one this project publishes.
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

    /// Both spellings decode through here, so neither can accept a
    /// version or a date the other would refuse.
    fn assemble(
        version: &str,
        date: &str,
        subject: &str,
        action: &'static str,
    ) -> Result<Release, AxError> {
        let refused = |msg: String| {
            AxError::failure(AxCode::ConfigInvalid, action, subject.to_owned()).with_recovery(msg)
        };
        let mut parts = version.split('.');
        let mut next_number = || parts.next().and_then(number);
        let (Some(major), Some(minor), Some(patch)) = (next_number(), next_number(), next_number())
        else {
            return Err(refused(format!(
                "`{version}` is not three dot-separated numbers, as in `0.0.5`"
            )));
        };
        if parts.next().is_some() {
            return Err(refused(format!(
                "`{version}` carries more than the three numbers a version of \
                 this project has"
            )));
        }
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
            major,
            minor,
            patch,
            year,
            month,
            day,
        })
    }

    /// `0.0.5`, the number alone.
    #[must_use]
    pub fn version(&self) -> String {
        let Release {
            major,
            minor,
            patch,
            ..
        } = *self;
        format!("{major}.{minor}.{patch}")
    }

    /// `v0.0.5-Pre-alpha-260912`, the tag this release was cut from.
    #[must_use]
    pub fn tag(&self) -> String {
        let Release {
            year, month, day, ..
        } = *self;
        let short = year.checked_sub(CENTURY).unwrap_or(year);
        format!("v{}{TAG_INFIX}{short:02}{month:02}{day:02}", self.version())
    }

    /// `0.0.5-pre.260912`, the version npm carries.
    #[must_use]
    pub fn npm_version(&self) -> String {
        let Release {
            year, month, day, ..
        } = *self;
        let short = year.checked_sub(CENTURY).unwrap_or(year);
        format!("{}{NPM_INFIX}{short:02}{month:02}{day:02}", self.version())
    }

    /// `2026-09-12`, the day this release was cut.
    ///
    /// The one rendering meant for a person rather than for a registry:
    /// a pre-alpha version number says almost nothing about how old a
    /// tree is, and how old it is, is what its reader most needs
    /// (CHANGELOG.md, opening note).
    #[must_use]
    pub fn released(&self) -> String {
        let Release {
            year, month, day, ..
        } = *self;
        format!("{year:04}-{month:02}-{day:02}")
    }
}

/// Where `mine` stands against the newest release published.
#[must_use]
pub fn stands(mine: &Release, newest: &Release) -> ReleaseVerdict {
    match mine.cmp(newest) {
        std::cmp::Ordering::Less => ReleaseVerdict::Behind,
        std::cmp::Ordering::Equal => ReleaseVerdict::Current,
        std::cmp::Ordering::Greater => ReleaseVerdict::Ahead,
    }
}

/// One version component: digits only, and no leading zero that would
/// make two spellings of one number.
///
/// Without the second rule `0.00.5` and `0.0.5` decode alike, and the
/// tag that round-trips back out is not the tag that came in.
fn number(text: &str) -> Option<u32> {
    let mut chars = text.chars();
    let first = chars.next()?;
    if !first.is_ascii_digit() {
        return None;
    }
    if first == '0' && chars.next().is_some() {
        return None;
    }
    text.parse().ok()
}

/// Two digits off the front of an iterator, as the number they spell.
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
    use super::{Release, ReleaseVerdict, stands};

    #[test]
    fn a_release_tag_becomes_the_semver_npm_accepts() {
        let cut = Release::from_tag("v0.0.5-Pre-alpha-260912", "0.0.5").unwrap();
        assert_eq!(cut.npm_version(), "0.0.5-pre.260912");
        assert_eq!(cut.version(), "0.0.5");
        assert_eq!(cut.released(), "2026-09-12");
    }

    /// The two spellings are one release, so each has to decode to the
    /// other. A channel that published one and checked the other would
    /// drift the first time either string changed.
    #[test]
    fn both_spellings_round_trip_through_one_another() {
        let tag = "v0.0.5-Pre-alpha-260912";
        let cut = Release::from_tag(tag, "0.0.5").unwrap();
        assert_eq!(cut.tag(), tag);
        assert_eq!(Release::from_npm_version(&cut.npm_version()).unwrap(), cut);
    }

    /// The check that keeps one release from having two version numbers.
    #[test]
    fn a_tag_disagreeing_with_the_build_is_refused() {
        let err = Release::from_tag("v0.0.3-Pre-alpha-260911", "0.0.4").unwrap_err();
        assert!(err.recovery().contains("two version numbers"), "{err:?}");
    }

    #[test]
    fn a_tag_of_another_shape_is_refused_rather_than_guessed_at() {
        for refused in [
            "0.0.4",
            "v0.0.4",
            "v0.0.4-Pre-alpha-26091",
            "v0.0.4-Pre-alpha-2609111",
            "v0.0.4-Pre-alpha-2609xx",
            "v0.0.4-Pre-alpha-261301",
            "v0.0.4-Pre-alpha-260900",
            "v0.0.4.1-Pre-alpha-260911",
            "v0.00.4-Pre-alpha-260911",
        ] {
            assert!(
                Release::from_tag(refused, "0.0.4").is_err(),
                "{refused} was accepted"
            );
        }
    }

    /// A binary reading its own bare `0.0.5` against the registry's
    /// `0.0.5-pre.260912` is the failure this whole type exists to stop:
    /// semver ranks a release above its own pre-releases, so the naive
    /// comparison reports the newest published release as the older one.
    #[test]
    fn the_date_decides_between_two_releases_of_one_version() {
        let older = Release::from_npm_version("0.0.5-pre.260911").unwrap();
        let newer = Release::from_npm_version("0.0.5-pre.260912").unwrap();
        assert_eq!(stands(&older, &newer), ReleaseVerdict::Behind);
        assert_eq!(stands(&newer, &older), ReleaseVerdict::Ahead);
        assert_eq!(stands(&newer, &newer), ReleaseVerdict::Current);
    }

    /// A larger version outranks a later date, which is what the
    /// registry does with the same two strings.
    #[test]
    fn the_version_outranks_the_date() {
        let old_version_late = Release::from_npm_version("0.0.5-pre.261231").unwrap();
        let new_version_early = Release::from_npm_version("0.0.6-pre.260101").unwrap();
        assert_eq!(
            stands(&old_version_late, &new_version_early),
            ReleaseVerdict::Behind
        );
    }

    /// The registry answering something this workflow never published is
    /// a refusal, not a release ranked against the running binary.
    #[test]
    fn an_npm_version_this_project_does_not_publish_is_refused() {
        for refused in ["0.0.5", "0.0.5-alpha.1", "0.0.5-pre.", "latest"] {
            assert!(
                Release::from_npm_version(refused).is_err(),
                "{refused} was accepted"
            );
        }
    }
}
