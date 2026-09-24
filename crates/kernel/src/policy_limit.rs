// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Policy limits as values (kernel-SPEC 8-73): each type carries one
//! ceiling's legal domain and the one refusal that says no, so a caller
//! hands over what it observed and cannot spell a second refusal.
//!
//! The numbers live in `consts_policy` and the sentences live here. A
//! limit has no public constructor, no getter, no `Display` and no
//! serde: a caller outside this crate can neither mint a looser limit
//! nor read the figure out to format a sentence of its own. The
//! compile-failure counterexample is `tests/ui/forge_policy_limit.rs`.

use crate::error::{AxCode, AxError};
use crate::locator::Locator;

/// Whether an observation is past a limit: the limit itself is
/// admitted, one more is refused. The one boundary rule, and the one
/// place an observation is narrowed to `u64`.
///
/// `usize` is at most 64 bits on every target Rust builds, so the
/// narrowing cannot fail there; the failing arm still answers "past the
/// limit", which keeps the rule off any target's pointer width.
fn exceeds(limit: u64, observed: usize) -> bool {
    match u64::try_from(observed) {
        Ok(count) => count > limit,
        Err(_) => true,
    }
}

/// How many pictures one turn may put on a provider request. The legal
/// domain is `0..=max`: a turn carrying exactly the limit is sent, and
/// one more picture is refused before any byte leaves for the provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImagesPerTurn(u64);

impl ImagesPerTurn {
    /// The only way a value of this type exists: `consts_policy` mints
    /// the policy instance with it, and no caller outside this crate
    /// can mint a looser one.
    pub(crate) const fn new(max: u64) -> ImagesPerTurn {
        ImagesPerTurn(max)
    }

    /// Whether this turn's pictures fit, refusing past the limit with
    /// the one sentence that names it.
    ///
    /// # Errors
    /// `E_INVALID_ARGS` when `found` is past the limit; the recovery
    /// names the limit and what to do with the pictures that do not fit.
    pub fn admit(self, found: usize) -> Result<(), AxError> {
        let max = self.0;
        if exceeds(max, found) {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "put pictures on a provider request",
                format!("this turn carries {found} pictures"),
            )
            .with_recovery(format!(
                "one turn carries at most {max} pictures; send the rest in a later turn"
            )));
        }
        Ok(())
    }
}

/// How many bytes one picture may carry to a provider. Sized so a
/// base64 body of it stays inside what every provider this city calls
/// accepts (the number's derivation is on the constant in
/// `consts_policy`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImageMaxBytes(u64);

impl ImageMaxBytes {
    /// The only way a value of this type exists; see [`ImagesPerTurn::new`].
    pub(crate) const fn new(max: u64) -> ImageMaxBytes {
        ImageMaxBytes(max)
    }

    /// Whether this picture's bytes fit, refusing past the limit with
    /// the one sentence that names it.
    ///
    /// # Errors
    /// `E_INVALID_ARGS` when `size` is past the limit; the recovery
    /// names the limit and what to do with the picture.
    pub fn admit(self, at: &Locator, size: usize) -> Result<(), AxError> {
        let max = self.0;
        if exceeds(max, size) {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "put a picture on a provider request",
                format!("{at} is {size} bytes"),
            )
            .with_recovery(format!(
                "one picture is at most {max} bytes; shrink it before attaching it"
            )));
        }
        Ok(())
    }
}

/// How many concern zones a clock stamp may carry, the UTC row not
/// among them. Refused rather than dropped: a configured zone that
/// silently stops rendering is a wrong stamp, and a wrong stamp is
/// worse than no stamp.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClockZonesMax(u64);

impl ClockZonesMax {
    /// The only way a value of this type exists; see [`ImagesPerTurn::new`].
    pub(crate) const fn new(max: u64) -> ClockZonesMax {
        ClockZonesMax(max)
    }

    /// Whether these configured zones fit, refusing past the limit with
    /// the one sentence that names it.
    ///
    /// # Errors
    /// `E_INVALID_ARGS` when `configured` is past the limit; the
    /// recovery names the limit and where a person edits the rows.
    pub fn admit(self, configured: usize) -> Result<(), AxError> {
        let max = self.0;
        if exceeds(max, configured) {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "format clock stamp",
                format!("{configured} zones exceed CLOCK_ZONES_MAX={max}"),
            )
            .with_recovery(format!(
                "keep at most {max} rows in `[clock] zones`; the UTC row is added on top \
                     of them and is never one of them"
            )));
        }
        Ok(())
    }
}

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
    use crate::consts_policy::{CLOCK_ZONES_MAX, IMAGE_MAX_BYTES, IMAGES_PER_TURN};

    fn a_picture() -> Locator {
        Locator::parse(&format!("cas:b3-{}", "0".repeat(64))).unwrap()
    }

    /// The boundary is the limit itself admitted and one past it
    /// refused, and the refusal is compared whole: the four parts are
    /// the contract, so a wording change has to be deliberate.
    #[test]
    fn a_turn_carries_the_limit_and_one_picture_past_it_is_refused() {
        assert!(IMAGES_PER_TURN.admit(4).is_ok());
        let expected = AxError::failure(
            AxCode::InvalidArgs,
            "put pictures on a provider request",
            "this turn carries 5 pictures",
        )
        .with_recovery("one turn carries at most 4 pictures; send the rest in a later turn");
        assert_eq!(IMAGES_PER_TURN.admit(5).unwrap_err(), expected);
    }

    #[test]
    fn a_picture_fits_at_the_ceiling_and_one_past_it_is_refused() {
        let at = a_picture();
        assert!(IMAGE_MAX_BYTES.admit(&at, 2_097_152).is_ok());
        let expected = AxError::failure(
            AxCode::InvalidArgs,
            "put a picture on a provider request",
            format!("{at} is 2097153 bytes"),
        )
        .with_recovery("one picture is at most 2097152 bytes; shrink it before attaching it");
        assert_eq!(IMAGE_MAX_BYTES.admit(&at, 2_097_153).unwrap_err(), expected);
    }

    #[test]
    fn the_zone_count_admits_the_limit_and_one_past_it_is_refused() {
        assert!(CLOCK_ZONES_MAX.admit(4).is_ok());
        let expected = AxError::failure(
            AxCode::InvalidArgs,
            "format clock stamp",
            "5 zones exceed CLOCK_ZONES_MAX=4",
        )
        .with_recovery(
            "keep at most 4 rows in `[clock] zones`; the UTC row is added on top of them and is \
             never one of them",
        );
        assert_eq!(CLOCK_ZONES_MAX.admit(5).unwrap_err(), expected);
    }
}
