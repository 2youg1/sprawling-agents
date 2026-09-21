// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! How much of a conversation one model can be given at once.

use std::num::NonZeroU64;

use serde::{Deserialize, Serialize};

/// The most tokens one model reads in a single call, prompt and reply
/// together.
///
/// **Zero is unrepresentable, and absence is not zero.** A window
/// nobody stated and a window of zero used to be the same byte, so the
/// context reminder measured a conversation against nothing and
/// reported every session as full. A figure nobody registered is
/// carried as `None` all the way to the reminder, which then stays
/// silent, and every layer in between is spared the rule that zero is
/// special.
///
/// A separate type from [`Ceiling`](crate::Ceiling) although both are
/// non-zero token counts: one bounds what the model may read and the
/// other bounds what it may write, and a call that swapped them would
/// compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Window(NonZeroU64);

impl Window {
    /// `None` for zero, which is not a window.
    #[must_use]
    pub const fn new(tokens: u64) -> Option<Window> {
        match NonZeroU64::new(tokens) {
            Some(tokens) => Some(Window(tokens)),
            None => None,
        }
    }

    #[must_use]
    pub const fn get(self) -> u64 {
        self.0.get()
    }
}

impl std::fmt::Display for Window {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.get())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, reason = "test code")]
mod tests {
    use super::*;

    #[test]
    fn a_window_of_zero_is_no_window_at_all() {
        assert_eq!(Window::new(0), None);
        assert_eq!(Window::new(200_000).unwrap().get(), 200_000);
    }

    #[test]
    fn a_window_travels_as_the_bare_number_and_absence_as_null() {
        let stated = serde_json::to_string(&Window::new(128_000)).unwrap();
        assert_eq!(stated, "128000");
        let absent = serde_json::to_string(&Option::<Window>::None).unwrap();
        assert_eq!(absent, "null");
        assert_eq!(
            serde_json::from_str::<Option<Window>>("128000").unwrap(),
            Window::new(128_000)
        );
    }

    /// Zero on the wire is refused rather than read as "no window":
    /// the encoding of absence is `null`, and a second encoding of it
    /// would be a second authority for what unstated means.
    #[test]
    fn zero_on_the_wire_is_refused_rather_than_read_as_absence() {
        assert!(serde_json::from_str::<Window>("0").is_err());
    }
}
