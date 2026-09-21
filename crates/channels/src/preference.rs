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
///
/// **This is the grammar of `[ui]` in the person's own file as well as
/// the shape of the answer.** The file is read through this type and
/// written by serialising it, so a key the file may hold and a field
/// the answer states are one declaration. A section that states only
/// some of them is read with the rest at the postures
/// [`PreferencesAnswer::default`] gives, and a key no field claims is
/// refused where it is written rather than ignored into a setting that
/// never takes effect.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct PreferencesAnswer {
    /// The language this person reads the interface in, and nothing
    /// when they have never said.
    ///
    /// Absent rather than a language this build picked for them: the
    /// browser's own tag is the only evidence anybody has before the
    /// first choice, and a stated `en` here would overrule it on every
    /// machine whose owner simply never opened the setting.
    pub lang: Option<Lang>,
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

/// What a person who has settled nothing is answered.
///
/// The postures this build draws before anybody chooses, stated once
/// for the file's reader and the answer's reader together. `panel` is
/// the one that is not its type's own default: the panel beside a
/// conversation is open until somebody closes it, so an unwritten file
/// has to say `true` where `welcomed` says `false`.
impl Default for PreferencesAnswer {
    fn default() -> Self {
        PreferencesAnswer {
            lang: None,
            welcomed: false,
            panel: true,
            appearance: Appearance::default(),
            proxying: kernel::Proxying::ExceptLocal,
            chords: Vec::new(),
        }
    }
}

impl Default for Appearance {
    fn default() -> Self {
        Appearance {
            lighting: Lighting::System,
            sans: Face::Geist,
            mono: Face::Geist,
            sans_stack: String::new(),
            mono_stack: String::new(),
            body_px: None,
            density: Density::Comfortable,
            chroma: Chroma::Full,
            motion: Motion::System,
        }
    }
}

impl PreferencesAnswer {
    /// Lands one named change on this record.
    ///
    /// Here rather than at the file: what a patch means is the patch's
    /// own rule, and a second application of it — in the store, in a
    /// test, in whatever keeps these next — would be a second answer
    /// to what `Chord("")` does.
    pub fn apply(&mut self, patch: PreferencePatch) {
        match patch {
            PreferencePatch::Lang(lang) => self.lang = Some(lang),
            PreferencePatch::Welcomed(welcomed) => self.welcomed = welcomed,
            PreferencePatch::Panel(panel) => self.panel = panel,
            PreferencePatch::Appearance(appearance) => self.appearance = appearance,
            PreferencePatch::Proxying(proxying) => self.proxying = proxying,
            PreferencePatch::Chord(chord) => {
                self.chords.retain(|held| held.action != chord.action);
                // An action returned to the chord this build ships has
                // no entry, so the list states what was changed rather
                // than what the keymap holds.
                if !chord.spelled.is_empty() {
                    self.chords.push(chord);
                    // Action order, so two machines that rebound the
                    // same chords in a different order hold the same
                    // file.
                    self.chords.sort_by(|one, two| one.action.cmp(&two.action));
                }
            }
        }
    }
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

    /// Rebinding one action twice leaves one entry, and returning it
    /// to the shipped chord leaves none: the list states what was
    /// changed, so an action that appears twice would make the keymap
    /// depend on which entry it read last.
    #[test]
    fn an_action_is_bound_once_and_unbinding_it_removes_the_entry() {
        let mut held = PreferencesAnswer::default();
        let bind = |spelled: &str| {
            PreferencePatch::Chord(Chord {
                action: "go.city".to_owned(),
                spelled: spelled.to_owned(),
            })
        };
        held.apply(bind("accel+2"));
        held.apply(bind("accel+3"));
        assert_eq!(held.chords.len(), 1);
        assert_eq!(
            held.chords.first().map(|held| held.spelled.as_str()),
            Some("accel+3")
        );
        held.apply(bind(""));
        assert!(held.chords.is_empty());
    }

    /// A section that states one fact is read with the rest at the
    /// postures this build draws, and the panel is the one whose
    /// absence does not mean `false`.
    #[test]
    fn a_section_that_states_one_fact_leaves_the_others_at_their_posture() {
        let held: PreferencesAnswer = serde_json::from_str(r#"{"welcomed":true}"#).unwrap();
        assert!(held.welcomed);
        assert!(held.panel);
        assert_eq!(held.lang, None);
        assert_eq!(held.appearance, Appearance::default());
    }

    /// A key this build does not read is refused where it is written.
    /// Ignoring it produces the one state nobody can diagnose: the
    /// setting is in the file, and nothing happens.
    #[test]
    fn a_key_no_field_claims_is_refused() {
        assert!(serde_json::from_str::<PreferencesAnswer>(r#"{"welcomd":true}"#).is_err());
    }
}
