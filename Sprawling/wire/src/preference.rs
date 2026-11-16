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
mod tests;
