// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Lang {
    En,
    Zh,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Lighting {
    System,
    Dark,
    Light,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Face {

    Paper,

    Maple,

    System,

    Custom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum TypeFeature {

    CodingLigatures,

    Duospace,

    NarrowSpace,

    SmallArrows,

    SingleStoryA,

    SlashedZero,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Weight {
    Light,
    Regular,
    Medium,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum ReadingFace {

    Interface,

    Libron,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Density {
    Comfortable,
    Compact,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Chroma {
    Full,
    Off,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Motion {
    System,
    On,
    Off,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Glass {
    On,
    Off,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Tier {
    Zen,
    Blend,
    Panorama,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Appearance {
    pub lighting: Lighting,
    pub sans: Face,
    pub mono: Face,

    pub sans_stack: String,

    pub mono_stack: String,

    #[serde(default)]
    pub reading: Option<ReadingFace>,

    pub body_px: Option<BodyPx>,

    #[serde(default)]
    pub fallbacks: Vec<String>,

    #[serde(default)]
    pub features: Vec<TypeFeature>,

    #[serde(default)]
    pub line_height: Option<LineHeight>,

    #[serde(default)]
    pub letter_spacing: Option<Tracking>,

    #[serde(default)]
    pub weight: Option<Weight>,
    pub density: Density,
    pub chroma: Chroma,
    pub motion: Motion,

    #[serde(default)]
    pub glass: Option<Glass>,

    #[serde(default)]
    pub blend_percent: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Chord {
    pub action: String,
    pub spelled: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct PreferencesAnswer {

    pub lang: Option<Lang>,

    pub core: CorePreferences,

    pub welcomed: bool,

    pub panel: bool,

    pub tier: Option<Tier>,
    pub appearance: Appearance,

    pub proxying: kernel::Proxying,

    pub chords: Vec<Chord>,

    pub tags: Vec<SessionTags>,

    pub theme: ThemeOverride,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ThemeOverride {

    pub tokens: BTreeMap<String, String>,

    pub css: Option<String>,
}

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

    pub fn apply(&mut self, patch: PreferencePatch) {
        match patch {
            PreferencePatch::Lang(lang) => self.lang = Some(lang),
            PreferencePatch::Welcomed(welcomed) => self.welcomed = welcomed,
            PreferencePatch::Panel(panel) => self.panel = panel,
            PreferencePatch::Tier(tier) => self.tier = Some(tier),
            PreferencePatch::Appearance(appearance) => self.appearance = appearance,
            PreferencePatch::Proxying(proxying) => self.proxying = proxying,

            PreferencePatch::CorePriority(priority) => self.core.priority = priority,
            PreferencePatch::CorePlacement(placement) => self.core.placement = placement,
            PreferencePatch::RunMemory(memory) => self.core.memory_bytes = memory,
            PreferencePatch::Tags(next) => tag::retagged(&mut self.tags, next),
            PreferencePatch::Theme(theme) => self.theme = theme,
            PreferencePatch::Chord(chord) => {
                self.chords.retain(|held| held.action != chord.action);

                if !chord.spelled.is_empty() {
                    self.chords.push(chord);

                    self.chords.sort_by(|one, two| one.action.cmp(&two.action));
                }
            }
        }
    }
}

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

    Chord(Chord),

    CorePriority(CorePriority),
    CorePlacement(CorePlacement),
    RunMemory(Option<std::num::NonZeroU64>),

    Tags(SessionTags),

    Theme(ThemeOverride),
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, reason = "test code")]
mod tests {
    use super::*;

    #[test]
    fn a_patch_travels_under_the_name_of_the_fact_it_changes() {
        let text = serde_json::to_string(&PreferencePatch::Lang(Lang::Zh)).unwrap();
        assert_eq!(text, r#"{"lang":"zh"}"#);
        let back: PreferencePatch = serde_json::from_str(&text).unwrap();
        assert_eq!(back, PreferencePatch::Lang(Lang::Zh));
    }

    #[test]
    fn a_word_no_build_offers_is_refused_rather_than_repaired() {
        assert!(serde_json::from_str::<Lighting>("\"sepia\"").is_err());
        assert!(serde_json::from_str::<PreferencePatch>(r#"{"lang":"fr"}"#).is_err());
    }

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

    #[test]
    fn a_section_that_states_one_fact_leaves_the_others_at_their_posture() {
        let held: PreferencesAnswer = serde_json::from_str(r#"{"welcomed":true}"#).unwrap();
        assert!(held.welcomed);
        assert!(held.panel);
        assert_eq!(held.lang, None);
        assert_eq!(held.appearance, Appearance::default());
    }

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

    #[test]
    fn a_key_no_field_claims_is_refused() {
        assert!(serde_json::from_str::<PreferencesAnswer>(r#"{"welcomd":true}"#).is_err());
    }
}
