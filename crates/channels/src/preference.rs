// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What one person settled about their own copy of the city: the
//! language they read, how the pages look, and the chords they rebound.
//!
//! **These are the person's layer, not the city's.** They live in
//! `~/.sprawling/config.toml` beside the components that are already
//! tied to this machine, they change nothing a run can observe, and
//! they are answered whole by [`Query::Preferences`](crate::Query) and
//! changed one named fact at a time by
//! [`Command::PutPreferences`](crate::Command).
//!
//! **Every value set a selector offers is declared once, here.** A
//! browser used to hold its own list of the words it would accept, so
//! an option the city stopped offering stayed pickable until somebody
//! noticed; the client now reads these enums out of the generated
//! schema, and an option it can draw is an option this build can load.
//!
//! One named change per fact rather than one whole-record write: two
//! screens that each settle one thing must not be able to overwrite
//! each other's field on the way past.

use serde::{Deserialize, Serialize};

/// Which language a person reads the interface in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Lang {
    En,
    Zh,
}

/// Which palette the pages are drawn in. `System` is the absence of an
/// opinion, resolved where the page is drawn rather than in the
/// stylesheet: the light palette is declared once, and a second
/// declaration of it inside a `prefers-color-scheme` block would be a
/// second authority for the same rungs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Lighting {
    System,
    Dark,
    Light,
}

/// Where one of the two typefaces comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Face {
    /// The family this build ships.
    Geist,
    /// Whatever this machine calls its interface font.
    System,
    /// The stack the person wrote out.
    Custom,
}

/// How much air the spacing steps carry. Two named postures rather than
/// a coefficient, because the coefficient is the stylesheet's to
/// choose.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Density {
    Comfortable,
    Compact,
}

/// Whether colour carries meaning on these pages, or only contrast
/// does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Chroma {
    Full,
    Off,
}

/// Whether the pages animate. `System` is the absence of an opinion,
/// which is what the stylesheet's `prefers-reduced-motion` block reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Motion {
    System,
    On,
    Off,
}

/// The smallest and the largest body size a person may ask for.
///
/// The floor is the smallest size the colour gate holds its contrast
/// tiers at, and the ceiling is where a line of body text stops being
/// body text. Stated here because both the form that offers the box
/// and the layer that writes the file have to agree, and a pair of
/// numbers typed twice is a pair that can be typed differently.
pub const BODY_PX_MIN: u32 = 12;
/// See [`BODY_PX_MIN`].
pub const BODY_PX_MAX: u32 = 20;

/// How the pages look.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Appearance {
    pub lighting: Lighting,
    pub sans: Face,
    pub mono: Face,
    /// The families this person wrote out, used when the matching
    /// [`Face`] is [`Face::Custom`] and empty otherwise.
    pub sans_stack: String,
    /// See `sans_stack`.
    pub mono_stack: String,
    /// The size of a line of body text in pixels, between
    /// [`BODY_PX_MIN`] and [`BODY_PX_MAX`]. Absent means the person
    /// stated no size and the stylesheet's own is drawn — which is a
    /// different statement from any number this build could pick for
    /// them.
    pub body_px: Option<u32>,
    pub density: Density,
    pub chroma: Chroma,
    pub motion: Motion,
}

/// One chord the person rebound: the action, and the chord as the
/// keymap spells it.
///
/// An action left at the chord this build ships has no entry, so the
/// list says what was changed rather than what the keymap holds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Chord {
    pub action: String,
    pub spelled: String,
}

/// Everything one person settled about their own copy of the city.
///
/// Whole rather than a dozen readings, because that is the record the
/// file holds and because a screen that changes two of these at once
/// must not be able to write one and drop the other.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct PreferencesAnswer {
    pub lang: Lang,
    /// Whether this person has walked through the welcome once. The
    /// city decides whether setting up is *needed*; this only decides
    /// whether somebody who skipped it is asked again.
    pub welcomed: bool,
    /// Whether the panel beside a conversation is open.
    pub panel: bool,
    pub appearance: Appearance,
    /// Which proxy rule an endpoint attached from now on starts with.
    /// An endpoint already attached keeps the rule the city recorded
    /// for it.
    pub proxying: kernel::Proxying,
    /// The chords this person rebound, action order.
    pub chords: Vec<Chord>,
}

/// One named change to [`PreferencesAnswer`].
///
/// A closed set of named changes rather than a whole record, so two
/// screens settling different facts cannot overwrite each other, and so
/// a frame that names a fact this build does not keep is refused at the
/// boundary rather than merged.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum PreferencePatch {
    Lang(Lang),
    Welcomed(bool),
    Panel(bool),
    Appearance(Appearance),
    Proxying(kernel::Proxying),
    /// Rebind one action, or return it to the chord this build ships by
    /// sending an empty `spelled`.
    Chord(Chord),
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, reason = "test code")]
mod tests {
    use super::*;

    /// A patch names one fact. The test that matters is that the tag is
    /// the fact's name, because that tag is what the person's file and
    /// the client's call site are both written against.
    #[test]
    fn a_patch_travels_under_the_name_of_the_fact_it_changes() {
        let text = serde_json::to_string(&PreferencePatch::Lang(Lang::Zh)).unwrap();
        assert_eq!(text, r#"{"lang":"zh"}"#);
        let back: PreferencePatch = serde_json::from_str(&text).unwrap();
        assert_eq!(back, PreferencePatch::Lang(Lang::Zh));
    }

    /// The silent repair this type exists to delete: a word no build
    /// offers used to become the posture the client ships with.
    #[test]
    fn a_word_no_build_offers_is_refused_rather_than_repaired() {
        assert!(serde_json::from_str::<Lighting>("\"sepia\"").is_err());
        assert!(serde_json::from_str::<PreferencePatch>(r#"{"lang":"fr"}"#).is_err());
    }
}
