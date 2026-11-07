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
//! browser holding its own list of the words it would accept would keep
//! an option the city stopped offering pickable until somebody noticed;
//! the client reads these enums out of the generated schema, so an
//! option it can draw is an option this build can load.
//!
//! One named change per fact rather than one whole-record write: two
//! screens that each settle one thing must not be able to overwrite
//! each other's field on the way past.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

mod core;
pub use core::{CorePlacement, CorePreferences, CorePriority};

mod body;
pub use body::{BODY_PX_MIN, BodyPx};

mod leading;
pub use leading::{LINE_HEIGHT_MIN, LineHeight};

mod tracking;
pub use tracking::{TRACKING_MIN, Tracking};

mod tag;
pub use tag::{SessionTags, TAG_MAX, Tag};

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
    /// Paper Mono, the family this build ships first
    /// (`client/src/fonts/PaperMono[wght].woff2`, wire D123).
    Paper,
    /// Maple Mono, also shipped, and the family the doctor's NF CN build
    /// extends with Han glyphs and Nerd Font icons.
    Maple,
    /// Whatever this machine calls its interface font.
    System,
    /// The stack the person wrote out.
    Custom,
}

/// One of the shipped face's own OpenType features, spelled as that
/// face's name table spells it (`crates/wire/spec/Preference.lean`
/// D125).
///
/// The six are Paper Mono's, whose `ss01`, `ss02`, `ss03`, `ss04`,
/// `cv01` and `zero` carry these names; another bundled face spells the
/// same intents with its own tags, and the stylesheet is where the two
/// are held together. The spelling here is what a person's file holds
/// and what the page's `data-features` attribute carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum TypeFeature {
    /// `ss01`: draw `--`, `->` and the rest as the ligatures they name.
    CodingLigatures,
    /// `ss02`: the narrow `M`, `W`, `m`, `w` and their kin.
    Duospace,
    /// `ss03`: a word space narrower than the normal one.
    NarrowSpace,
    /// `ss04`: arrows a little smaller than the ones the face draws.
    SmallArrows,
    /// `cv01`: the `a` without its upper arm.
    SingleStoryA,
    /// `zero`: a zero that cannot be read as an `O`.
    SlashedZero,
}

/// How heavy running text stands: three postures, and the strong steps
/// of the type ladder stand two hundred above whichever holds
/// (`crates/wire/spec/Preference.lean` D126).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Weight {
    Light,
    Regular,
    Medium,
}

/// Which face reading text is drawn in: the messages of a conversation,
/// documents and reports. Controls, labels, figures and code keep the
/// interface's faces whichever this says
/// (`crates/wire/spec/Preference.lean` D52).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum ReadingFace {
    /// The face the interface is drawn in, whichever [`Face`] that is.
    Interface,
    /// Libron, the serif face this build ships for reading.
    Libron,
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

/// Whether the edge layer's small surfaces are drawn as glass. `On`
/// still yields to a machine that asks for less transparency, which
/// only some engines report: that gap is why the switch exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Glass {
    On,
    Off,
}

/// How much of the world layer the page draws: none of it, whole panels
/// beside the conversation, or all of it as the workspace with the
/// conversation as a band along its foot. Three layouts rather than
/// three points on one slider, so three names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Tier {
    Zen,
    Blend,
    Panorama,
}

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
    /// Absent means the person stated no opinion and reading text is
    /// drawn in the interface's face.
    #[serde(default)]
    pub reading: Option<ReadingFace>,
    /// The size of a line of body text in pixels, at least
    /// [`BODY_PX_MIN`]. Absent means the person
    /// stated no size and the stylesheet's own is drawn — which is a
    /// different statement from any number this build could pick for
    /// them.
    pub body_px: Option<BodyPx>,
    /// The families a face is followed by for the glyphs it lacks, in
    /// the order they are tried: at most two names, empty by default
    /// (`crates/wire/spec/Preference.lean` D124).
    ///
    /// The city stores what the person wrote and does not parse it: the
    /// page owns the name grammar and the stacks that read it, as it
    /// owns the stacks beside them.
    #[serde(default)]
    pub fallbacks: Vec<String>,
    /// The shipped face's own features the person turned on, in the
    /// order this crate declares them (`crates/wire/spec/Preference.lean`
    /// D125). Empty by default, and a feature no build offers is refused
    /// where it is read rather than dropped.
    #[serde(default)]
    pub features: Vec<TypeFeature>,
    /// The height of one line of running text in thousandths of the
    /// font size, at least [`LINE_HEIGHT_MIN`]. Absent means the person
    /// stated none and the stylesheet's own is drawn
    /// (`crates/wire/spec/Preference.lean` D126).
    #[serde(default)]
    pub line_height: Option<LineHeight>,
    /// The space between letters in thousandths of an em, at least
    /// [`TRACKING_MIN`]. Absent means the person stated none and the
    /// page draws its own, which differs by language.
    #[serde(default)]
    pub letter_spacing: Option<Tracking>,
    /// How heavy running text stands. Absent means the person stated no
    /// opinion and the page draws `regular`.
    #[serde(default)]
    pub weight: Option<Weight>,
    pub density: Density,
    pub chroma: Chroma,
    pub motion: Motion,
    /// Absent means the person stated no opinion and the page draws
    /// its own posture.
    #[serde(default)]
    pub glass: Option<Glass>,
    /// The world layer's opacity in the blend tier, in percent. Absent
    /// means the stylesheet's own is drawn. The domain the slider
    /// offers is the page's to state, because the page alone draws it;
    /// a value outside it is read back as absent there.
    #[serde(default)]
    pub blend_percent: Option<u32>,
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
    /// Performance settings read from `[core]`, never stored in `[ui]`.
    pub core: CorePreferences,
    /// Whether this person has walked through the welcome once. The
    /// city decides whether setting up is *needed*; this only decides
    /// whether somebody who skipped it is asked again.
    pub welcomed: bool,
    /// Whether the panel beside a conversation is open.
    pub panel: bool,
    /// The tier this person last chose, and nothing when they never
    /// chose one: the tier a first visit opens in is the page's posture,
    /// and a stated tier here would overrule it on every machine whose
    /// owner never pressed the key.
    pub tier: Option<Tier>,
    pub appearance: Appearance,
    /// Which proxy rule an endpoint attached from now on starts with.
    /// An endpoint already attached keeps the rule the city recorded
    /// for it.
    pub proxying: kernel::Proxying,
    /// The chords this person rebound, action order.
    pub chords: Vec<Chord>,
    /// The tags this person gave sessions, in session order
    /// (`crates/wire/spec/Preference.lean` §8-84).
    pub tags: Vec<SessionTags>,
    /// The person's colours laid over the built-in theme
    /// (`crates/wire/spec/Preference.lean` D29).
    pub theme: ThemeOverride,
}

/// Colours laid over the theme the page ships with. The empty value is
/// the built-in theme.
///
/// The values are CSS, and the city does not parse them: the page owns
/// both the theme and the stylesheet that reads it, and checks the keys
/// against the `@theme` block and the legibility of the result.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ThemeOverride {
    /// A CSS variable of the `@theme` block in `client/src/theme.css`,
    /// mapped to the CSS colour that replaces it.
    pub tokens: BTreeMap<String, String>,
    /// A whole stylesheet the person wrote, laid after the tokens.
    pub css: Option<String>,
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
            core: CorePreferences::default(),
            welcomed: false,
            panel: true,
            tier: None,
            appearance: Appearance::default(),
            proxying: kernel::Proxying::ExceptLocal,
            chords: Vec::new(),
            tags: Vec::new(),
            theme: ThemeOverride::default(),
        }
    }
}

impl Default for Appearance {
    fn default() -> Self {
        Appearance {
            lighting: Lighting::System,
            sans: Face::Paper,
            mono: Face::Paper,
            sans_stack: String::new(),
            mono_stack: String::new(),
            reading: None,
            body_px: None,
            fallbacks: Vec::new(),
            features: Vec::new(),
            line_height: None,
            letter_spacing: None,
            weight: None,
            density: Density::Comfortable,
            chroma: Chroma::Full,
            motion: Motion::System,
            glass: None,
            blend_percent: None,
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
            PreferencePatch::Tier(tier) => self.tier = Some(tier),
            PreferencePatch::Appearance(appearance) => self.appearance = appearance,
            PreferencePatch::Proxying(proxying) => self.proxying = proxying,
            // The writer stores these separately from `[ui]`.
            PreferencePatch::CorePriority(priority) => self.core.priority = priority,
            PreferencePatch::CorePlacement(placement) => self.core.placement = placement,
            PreferencePatch::RunMemory(memory) => self.core.memory_bytes = memory,
            PreferencePatch::Tags(next) => tag::retagged(&mut self.tags, next),
            PreferencePatch::Theme(theme) => self.theme = theme,
            PreferencePatch::Chord(chord) => {
                self.chords.retain(|held| held.action != chord.action);
                // An empty spelling returns the action to the chord this
                // build ships, which `Chord` keeps no entry for.
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
    Tier(Tier),
    Appearance(Appearance),
    Proxying(kernel::Proxying),
    /// Rebind one action, or return it to the chord this build ships by
    /// sending an empty `spelled`.
    Chord(Chord),
    /// Whether the core's threads stand above normal. Lands in the
    /// file's `[core]` section, not in `[ui]` (`crates/wire/spec/Command/Step.lean` §8-61).
    CorePriority(CorePriority),
    CorePlacement(CorePlacement),
    RunMemory(Option<std::num::NonZeroU64>),
    /// Replace one session's tags with these; an empty set removes them
    /// (`crates/wire/spec/Preference.lean` §8-84).
    Tags(SessionTags),
    /// Replace the colour override; the empty value returns to the
    /// built-in theme (`crates/wire/spec/Preference.lean` D29).
    Theme(ThemeOverride),
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

    /// No silent repair: a word no build offers is not turned into the
    /// posture the client ships with.
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

    /// An appearance section with `fields` laid over the postures a file
    /// that states nothing else holds. One spelling of the section, so a
    /// test that adds a fact states only that fact.
    fn appearance(fields: serde_json::Value) -> serde_json::Value {
        let mut stated = serde_json::json!({
            "lighting": "dark", "sans": "paper", "mono": "paper",
            "sans_stack": "", "mono_stack": "", "body_px": null,
            "density": "compact", "chroma": "full", "motion": "system",
        });
        if let (serde_json::Value::Object(extra), serde_json::Value::Object(stated)) =
            (fields, &mut stated)
        {
            stated.extend(extra);
        }
        serde_json::json!({ "appearance": stated })
    }

    /// The built-in face is Paper Mono, and neither a word of the face
    /// that left the bundle nor the word of the face that stayed is read
    /// as it: `maple` is Maple Mono, which still ships
    /// (`crates/wire/spec/Preference.lean` D123).
    #[test]
    fn the_shipped_face_is_paper_and_no_old_name_reads() {
        let held = serde_json::to_value(PreferencesAnswer::default())
            .unwrap()
            .pointer("/appearance")
            .cloned()
            .unwrap();
        assert_eq!(
            (held.get("sans"), held.get("mono")),
            (
                Some(&serde_json::json!("paper")),
                Some(&serde_json::json!("paper"))
            )
        );
        assert!(
            serde_json::from_value::<PreferencesAnswer>(appearance(
                serde_json::json!({ "sans": "geist" })
            ))
            .is_err()
        );
        let maple: PreferencesAnswer =
            serde_json::from_value(appearance(serde_json::json!({ "sans": "maple" }))).unwrap();
        assert_eq!(maple.appearance.sans, Face::Maple);
    }

    /// The fallback chain reads back as written, in its order, and a
    /// section written before it existed reads back as none: the city
    /// stores the names and does not parse them
    /// (`crates/wire/spec/Preference.lean` D124).
    #[test]
    fn the_fallback_chain_reads_back_in_its_order_and_absent_as_none() {
        let read_back = |fallbacks: Option<serde_json::Value>| {
            let mut stated = serde_json::json!({});
            if let (Some(names), serde_json::Value::Object(stated)) = (fallbacks, &mut stated) {
                stated.insert("fallbacks".to_owned(), names);
            }
            let held: PreferencesAnswer = serde_json::from_value(appearance(stated)).unwrap();
            held.appearance.fallbacks
        };
        assert_eq!(
            read_back(Some(serde_json::json!([
                "Maple Mono NF CN",
                "Noto Sans SC"
            ]))),
            vec!["Maple Mono NF CN".to_owned(), "Noto Sans SC".to_owned()]
        );
        assert_eq!(read_back(None), Vec::<String>::new());
    }

    /// The six features read back as stated and a seventh word is
    /// refused rather than dropped (`crates/wire/spec/Preference.lean`
    /// D125).
    #[test]
    fn the_six_features_read_back_and_a_word_no_bundle_offers_is_refused() {
        let held: PreferencesAnswer = serde_json::from_value(appearance(serde_json::json!({
            "features": ["duospace", "single_story_a", "slashed_zero"],
        })))
        .unwrap();
        assert_eq!(
            held.appearance.features,
            vec![
                TypeFeature::Duospace,
                TypeFeature::SingleStoryA,
                TypeFeature::SlashedZero,
            ]
        );
        assert!(
            serde_json::from_value::<PreferencesAnswer>(appearance(serde_json::json!({
                "features": ["kerning"],
            })))
            .is_err()
        );
        assert_eq!(Appearance::default().features, Vec::new());
    }

    /// A line box under the floor is refused where it is read, and any
    /// taller one reads back as the number it was
    /// (`crates/wire/spec/Preference.lean` D126).
    #[test]
    fn a_line_box_under_the_floor_is_refused_and_any_taller_one_reads() {
        let read_back = |height: u32| {
            serde_json::from_value::<PreferencesAnswer>(appearance(
                serde_json::json!({ "line_height": height }),
            ))
            .map(|held| held.appearance.line_height)
        };
        assert!(read_back(1199).is_err());
        assert_eq!(
            [1200, 1625, 3000]
                .into_iter()
                .map(|height| read_back(height).unwrap())
                .collect::<Vec<_>>(),
            vec![
                Some(LineHeight::new(1200).unwrap()),
                Some(LineHeight::new(1625).unwrap()),
                Some(LineHeight::new(3000).unwrap()),
            ]
        );
        assert_eq!(Appearance::default().line_height, None);
    }

    /// A tracking under the floor is refused where it is read, and any
    /// looser one reads back as the number it was.
    #[test]
    fn a_tracking_under_the_floor_is_refused_and_any_looser_one_reads() {
        let read_back = |spacing: i32| {
            serde_json::from_value::<PreferencesAnswer>(appearance(
                serde_json::json!({ "letter_spacing": spacing }),
            ))
            .map(|held| held.appearance.letter_spacing)
        };
        assert!(read_back(-51).is_err());
        assert_eq!(
            [-50, 0, 12, 40]
                .into_iter()
                .map(|spacing| read_back(spacing).unwrap())
                .collect::<Vec<_>>(),
            vec![
                Some(Tracking::new(-50).unwrap()),
                Some(Tracking::new(0).unwrap()),
                Some(Tracking::new(12).unwrap()),
                Some(Tracking::new(40).unwrap()),
            ]
        );
        assert_eq!(Appearance::default().letter_spacing, None);
    }

    /// The weight is the person's to state and absent until they do.
    #[test]
    fn the_weight_reads_back_as_stated_and_absent_as_never_said() {
        let read_back = |weight: &str| {
            serde_json::from_value::<PreferencesAnswer>(appearance(
                serde_json::json!({ "weight": weight }),
            ))
            .map(|held| held.appearance.weight)
        };
        assert_eq!(
            [read_back("light").unwrap(), read_back("medium").unwrap(),],
            [Some(Weight::Light), Some(Weight::Medium)]
        );
        assert!(read_back("bold").is_err());
        assert_eq!(Appearance::default().weight, None);
    }

    /// A section written before the page's glass, blend opacity and
    /// tier reached the city still reads, with the three absent: every
    /// person's file predates them.
    #[test]
    fn an_appearance_written_before_glass_and_blend_still_reads() {
        let held: PreferencesAnswer = serde_json::from_str(
            r#"{"appearance":{"lighting":"dark","sans":"maple","mono":"maple","sans_stack":"","mono_stack":"","body_px":null,"density":"compact","chroma":"full","motion":"system"}}"#,
        )
        .unwrap();
        assert_eq!(
            (
                held.tier,
                held.appearance.glass,
                held.appearance.blend_percent,
                held.appearance.line_height,
                held.appearance.letter_spacing,
                held.appearance.weight,
            ),
            (None, None, None, None, None, None)
        );
        assert_eq!(
            (
                held.appearance.fallbacks.len(),
                held.appearance.features.len(),
            ),
            (0, 0)
        );
        let mut chosen = held;
        chosen.apply(PreferencePatch::Tier(Tier::Panorama));
        assert_eq!(chosen.tier, Some(Tier::Panorama));
    }

    /// The body size has a floor and no ceiling (`crates/wire/spec/Preference.lean`
    /// D51): a size under the floor is refused where it is read, so the
    /// person's file and a frame meet the rule the page's box states,
    /// and any size above it reads back as the number it was.
    #[test]
    fn a_body_size_under_the_floor_is_refused_and_any_size_above_it_reads() {
        let read_back = |px: u32| {
            serde_json::from_value::<PreferencesAnswer>(appearance(
                serde_json::json!({ "body_px": px }),
            ))
            .map(|held| held.appearance.body_px)
        };
        assert!(read_back(11).is_err());
        for px in [12, 24, 96] {
            assert_eq!(read_back(px).unwrap(), Some(BodyPx::new(px).unwrap()));
        }
    }

    /// The reading face is the person's to state and absent until they
    /// do (`crates/wire/spec/Preference.lean` D52): a section that names
    /// it reads back the face it named, and one written before the
    /// field existed reads back as never stated.
    #[test]
    fn the_reading_face_reads_back_as_stated_and_absent_as_never_said() {
        let read_back = |reading: Option<&str>| {
            let mut fields = serde_json::json!({});
            if let (Some(face), serde_json::Value::Object(fields)) = (reading, &mut fields) {
                fields.insert("reading".to_owned(), serde_json::json!(face));
            }
            serde_json::from_value::<PreferencesAnswer>(appearance(fields))
                .map(|held| held.appearance.reading)
        };
        assert_eq!(
            [
                read_back(Some("libron")).unwrap(),
                read_back(Some("interface")).unwrap(),
                read_back(None).unwrap(),
            ],
            [
                Some(ReadingFace::Libron),
                Some(ReadingFace::Interface),
                None,
            ]
        );
        assert!(read_back(Some("georgia")).is_err());
    }

    /// A key this build does not read is refused where it is written.
    /// Ignoring it produces the one state nobody can diagnose: the
    /// setting is in the file, and nothing happens.
    #[test]
    fn a_key_no_field_claims_is_refused() {
        assert!(serde_json::from_str::<PreferencesAnswer>(r#"{"welcomd":true}"#).is_err());
    }
}
