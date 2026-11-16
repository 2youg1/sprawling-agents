// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::io::Read as _;

use crate::error::{AxCode, AxError};
use crate::locator::Locator;

fn exceeds(limit: u64, observed: usize) -> bool {
    match u64::try_from(observed) {
        Ok(count) => count > limit,
        Err(_) => true,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImagesPerTurn(u64);

impl ImagesPerTurn {

    pub(crate) const fn new(max: u64) -> ImagesPerTurn {
        ImagesPerTurn(max)
    }

    pub fn admit(self, found: usize) -> Result<(), AxError> {
        let max = self.0;
        if exceeds(max, found) {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "put pictures on a provider request",
                format!("this request carries {found} distinct pictures"),
            )
            .with_recovery(format!(
                "one request carries at most {max} distinct pictures; stop and fork from \
                 before older pictures, or start a new session, to leave them out"
            )));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImageMaxBytes(u64);

impl ImageMaxBytes {

    pub(crate) const fn new(max: u64) -> ImageMaxBytes {
        ImageMaxBytes(max)
    }

    pub fn read_from(self, source: impl std::io::Read) -> std::io::Result<Vec<u8>> {
        let mut bytes = Vec::new();
        source
            .take(self.0.saturating_add(1))
            .read_to_end(&mut bytes)?;
        Ok(bytes)
    }

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImageQuality(u64);

impl ImageQuality {

    pub(crate) const fn new(max: u64) -> ImageQuality {
        ImageQuality(max)
    }

    pub fn admit(self, asked: u32) -> Result<(), AxError> {
        if u64::from(asked) <= self.0 {
            return Ok(());
        }
        Err(AxError::failure(
            AxCode::InvalidArgs,
            "encode a picture",
            format!("quality {asked} is outside 0..={}", self.0),
        )
        .with_recovery(format!(
            "pass a quality between 0 and {}; an encoder has no others",
            self.0
        )))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClockZonesMax(u64);

impl ClockZonesMax {

    pub(crate) const fn new(max: u64) -> ClockZonesMax {
        ClockZonesMax(max)
    }

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
    use crate::consts_policy::{CLOCK_ZONES_MAX, IMAGE_MAX_BYTES, IMAGE_QUALITY, IMAGES_PER_TURN};

    fn a_picture() -> Locator {
        Locator::parse(&format!("cas:b3-{}", "0".repeat(64))).unwrap()
    }

    #[test]
    fn a_request_carries_the_limit_and_one_distinct_picture_past_it_is_refused() {
        assert!(IMAGES_PER_TURN.admit(4).is_ok());
        let expected = AxError::failure(
            AxCode::InvalidArgs,
            "put pictures on a provider request",
            "this request carries 5 distinct pictures",
        )
        .with_recovery(
            "one request carries at most 4 distinct pictures; stop and fork from before older \
             pictures, or start a new session, to leave them out",
        );
        assert_eq!(IMAGES_PER_TURN.admit(5).unwrap_err(), expected);
    }

    #[test]
    fn the_quality_domain_admits_its_limit_and_refuses_one_past_it() {
        assert!(IMAGE_QUALITY.admit(100).is_ok());
        assert!(IMAGE_QUALITY.admit(0).is_ok());
        let expected = AxError::failure(
            AxCode::InvalidArgs,
            "encode a picture",
            "quality 101 is outside 0..=100",
        )
        .with_recovery("pass a quality between 0 and 100; an encoder has no others");
        assert_eq!(IMAGE_QUALITY.admit(101).unwrap_err(), expected);
    }

    #[test]
    fn a_picture_fits_at_the_ceiling_and_one_past_it_is_refused() {
        let at = a_picture();
        assert!(IMAGE_MAX_BYTES.admit(&at, 4_194_304).is_ok());
        let expected = AxError::failure(
            AxCode::InvalidArgs,
            "put a picture on a provider request",
            format!("{at} is 4194305 bytes"),
        )
        .with_recovery("one picture is at most 4194304 bytes; shrink it before attaching it");
        assert_eq!(IMAGE_MAX_BYTES.admit(&at, 4_194_305).unwrap_err(), expected);
    }

    #[test]
    fn a_picture_read_stops_one_byte_past_the_limit() {
        let source = vec![0u8; 5_000_000];
        let bytes = IMAGE_MAX_BYTES.read_from(&mut &source[..]).unwrap();
        assert_eq!(bytes.len(), 4_194_305);
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
