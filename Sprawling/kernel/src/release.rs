// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Which release a binary is, and how two releases order.
//! The part `crates/kernel/spec/Release.lean` specifies this module.
//!
//! **One release is spelled two ways, and this is the only place that
//! knows both.** A pre-alpha git tag reads `v0.0.5-Pre-alpha-260912`; npm accepts
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
//! **The maturity is written once.** A tag carries it between the
//! version and the date, `-Alpha-` today; [`MATURITY`] is the one
//! place it is decided, and the tag, the first line of `sprawling status`
//! and the documents render it (kernel D18). The npm spelling keeps
//! `-pre.` whatever the maturity, because semver sorts pre-release
//! identifiers as text and `alpha` sorts before `pre`.
//!
//! No clock and no socket. What the registry currently offers is the
//! caller's to fetch; this judges only what it is handed
//! (ARCHITECTURE.md paragraph 1).

use serde::{Deserialize, Serialize};

use crate::error::{AxCode, AxError};

mod number;
pub use number::Version;

/// The package identity shared by the AUR definition and installed-origin reader.
pub const AUR_PACKAGE_NAME: &str = "sprawling-bin";

/// The POSIX directory relative to the installation root, including bundled resources.
#[must_use]
pub fn aur_install_directory() -> String {
    format!("usr/lib/{AUR_PACKAGE_NAME}")
}

/// Whether the executable occupies the directory emitted by the AUR packager.
#[must_use]
pub fn is_aur_install(exe: &std::path::Path) -> bool {
    exe.parent()
        .is_some_and(|directory| directory.ends_with(aur_install_directory()))
}

/// How far this project stands, as every release it cuts names it
/// (kernel D18).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Maturity {
    /// Runs end to end, and nothing is promised.
    PreAlpha,
    /// Usable.
    Alpha,
}

/// The maturity of every release this tree cuts.
///
/// The one place it is written: the tag infix, the first line of
/// `sprawling status` and the documents' `maturity` spans are rendered
/// from it, so entering alpha moves this constant and nothing else.
pub const MATURITY: Maturity = Maturity::Alpha;

impl Maturity {
    /// `pre-alpha` or `alpha`, the word as a sentence writes it.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Maturity::PreAlpha => "pre-alpha",
            Maturity::Alpha => "alpha",
        }
    }

    /// `Pre-alpha` or `Alpha`, the word as a tag and a heading write it.
    #[must_use]
    pub const fn titled(self) -> &'static str {
        match self {
            Maturity::PreAlpha => "Pre-alpha",
            Maturity::Alpha => "Alpha",
        }
    }

    /// `-Pre-alpha-`, what separates the version from the date in a tag.
    fn tag_infix(self) -> String {
        format!("-{}-", self.titled())
    }
}

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
    version: Version,
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
/// Named for what it judges rather than `Standing`, which `wire`
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
        Release::decode_tag(tag, expected_version, MATURITY)
    }

    /// The release a tag names, read as a build of `maturity` reads it.
    fn decode_tag(
        tag: &str,
        expected_version: &str,
        maturity: Maturity,
    ) -> Result<Release, AxError> {
        let refused = |msg: &str| {
            AxError::failure(AxCode::ConfigInvalid, "read a release tag", tag.to_owned())
                .with_recovery(msg.to_owned())
        };
        let infix = maturity.tag_infix();
        let body = tag.strip_prefix('v').ok_or_else(|| {
            refused(&format!(
                "a release tag begins with `v`, as in `v0.0.5{infix}260912`"
            ))
        })?;
        let (version, date) = body.split_once(infix.as_str()).ok_or_else(|| {
            refused(&format!(
                "a release tag of this build reads `v<version>{infix}<YYMMDD>`; \
                 a tag of another shape or another maturity is not one of its releases"
            ))
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

    /// A published tag, including releases cut before the current maturity.
    ///
    /// # Errors
    /// When neither supported maturity decodes the tag's version and date.
    pub fn from_published_tag(tag: &str) -> Result<Release, AxError> {
        for maturity in [Maturity::Alpha, Maturity::PreAlpha] {
            if let Some((version, _)) = tag
                .strip_prefix('v')
                .and_then(|body| body.split_once(maturity.tag_infix().as_str()))
            {
                return Self::decode_tag(tag, version, maturity);
            }
        }
        Err(
            AxError::failure(AxCode::ConfigInvalid, "read a published tag", tag)
                .with_recovery("use a published Alpha or Pre-alpha release tag"),
        )
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

    /// `0.0.5`, the number alone, which is also the version crates.io
    /// carries.
    #[must_use]
    pub fn version(&self) -> String {
        self.version.to_string()
    }

    /// `v0.0.9-Alpha-261004`, the tag this release was cut from.
    #[must_use]
    pub fn tag(&self) -> String {
        self.tag_at(MATURITY)
    }

    /// The tag a build of `maturity` cuts for this release.
    fn tag_at(&self, maturity: Maturity) -> String {
        let Release {
            year, month, day, ..
        } = *self;
        let short = year.checked_sub(CENTURY).unwrap_or(year);
        format!(
            "v{}{}{short:02}{month:02}{day:02}",
            self.version(),
            maturity.tag_infix()
        )
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
    /// a 0.0.x version number says almost nothing about how old a
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
    verdict_of(mine.cmp(newest))
}

/// Where `mine` stands against the newest version crates.io offers,
/// judged on the version number alone (kernel D35): crates.io takes one
/// upload per version, so a re-cut that changed only the date never
/// reaches it, and for a binary cargo installed the same version is
/// current.
#[must_use]
pub fn stands_on_crates(mine: &Release, newest: &Version) -> ReleaseVerdict {
    stands_on_versions(&mine.version, newest)
}

/// Compares Cargo versions when a registry install was compiled without a release tag.
#[must_use]
pub fn stands_on_versions(mine: &Version, newest: &Version) -> ReleaseVerdict {
    verdict_of(mine.cmp(newest))
}

/// The step `stands` and `stands_on_crates` share: an order read as a
/// verdict.
fn verdict_of(order: std::cmp::Ordering) -> ReleaseVerdict {
    match order {
        std::cmp::Ordering::Less => ReleaseVerdict::Behind,
        std::cmp::Ordering::Equal => ReleaseVerdict::Current,
        std::cmp::Ordering::Greater => ReleaseVerdict::Ahead,
    }
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
    use super::{Maturity, Release, ReleaseVerdict, Version, stands, stands_on_crates};
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
            .prop_map(|(major, minor, patch, year, month, day)| {
                Release::from_npm_version(&format!(
                    "{major}.{minor}.{patch}-pre.{year:02}{month:02}{day:02}"
                ))
                .unwrap()
            })
    }

    /// Semver's rule 11 for the one shape this project publishes, written
    /// out by hand: the three numbers, then the numeric identifier after
    /// `pre`, each compared as a number.
    fn semver_key(npm: &str) -> (u32, u32, u32, u32) {
        let (core, pre) = npm.split_once("-pre.").unwrap();
        let mut numbers = core.split('.').map(|part| part.parse::<u32>().unwrap());
        let mut next = || numbers.next().unwrap();
        (next(), next(), next(), pre.parse().unwrap())
    }

    proptest! {
        /// kernel D35 over every pair of releases: crates.io and npm give
        /// one verdict when the versions differ, crates.io cannot tell two
        /// cuts of one version apart, and the npm spelling orders as
        /// `Release` does.
        #[test]
        fn the_three_spellings_give_one_order(
            a in any_release(),
            other in any_release(),
            same_version in any::<bool>(),
        ) {
            // Half the pairs are two cuts of one version, which a draw of
            // two independent releases almost never makes.
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

    /// The answer a binary cargo installed reads: crates.io names the
    /// same version, and that is current.
    #[test]
    fn a_crates_answer_of_the_same_release_reads_as_current() {
        let mine = Release::from_npm_version("0.0.8-pre.261002").unwrap();
        let newest = Version::from_crates_version("0.0.8").unwrap();
        assert_eq!(stands_on_crates(&mine, &newest), ReleaseVerdict::Current);
        assert!(Version::from_crates_version("0.0.8-pre.261002").is_err());
    }

    /// Entering alpha moves `MATURITY` and nothing else, so the tag a
    /// build cuts and the tag it reads have to follow the maturity they
    /// are given: an alpha build reads an alpha tag as its own and
    /// refuses a pre-alpha one, as it refuses a tag of another version.
    #[test]
    fn an_alpha_build_cuts_and_reads_alpha_tags() {
        let read = |tag: &str| {
            Release::decode_tag(tag, "0.1.0", Maturity::Alpha)
                .map(|cut| cut.tag_at(Maturity::Alpha))
                .map_err(|err| err.recovery().to_owned())
        };
        assert_eq!(
            (
                read("v0.1.0-Alpha-270101"),
                read("v0.1.0-Pre-alpha-270101").is_err()
            ),
            (Ok("v0.1.0-Alpha-270101".to_owned()), true)
        );
    }

    #[test]
    fn a_release_tag_becomes_the_semver_npm_accepts() {
        let cut =
            Release::decode_tag("v0.0.5-Pre-alpha-260912", "0.0.5", Maturity::PreAlpha).unwrap();
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
        let cut = Release::decode_tag(tag, "0.0.5", Maturity::PreAlpha).unwrap();
        assert_eq!(cut.tag_at(Maturity::PreAlpha), tag);
        assert_eq!(Release::from_npm_version(&cut.npm_version()).unwrap(), cut);
    }

    proptest! {
        /// The tag a build reads is the tag it cuts: the public reader and
        /// the public writer, both at this tree's maturity, are one spelling.
        #[test]
        fn a_release_reads_back_from_the_tag_it_cuts(cut in any_release()) {
            let read = Release::from_tag(&cut.tag(), &cut.version()).unwrap();
            prop_assert_eq!(read, cut);
        }
    }

    /// A maturity is one word, which a sentence writes in lower case and a
    /// tag or a heading writes with a capital.
    #[test]
    fn a_maturity_is_one_word_in_two_cases() {
        for maturity in [Maturity::PreAlpha, Maturity::Alpha] {
            assert_eq!(maturity.word(), maturity.titled().to_lowercase());
        }
    }

    /// A version part of several digits reads as its number; only a part
    /// that opens with a zero and goes on is refused.
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

    /// The check that keeps one release from having two version numbers.
    #[test]
    fn a_tag_disagreeing_with_the_build_is_refused() {
        let err = Release::decode_tag("v0.0.3-Pre-alpha-260911", "0.0.4", Maturity::PreAlpha)
            .unwrap_err();
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
                Release::decode_tag(refused, "0.0.4", Maturity::PreAlpha).is_err(),
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
