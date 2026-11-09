// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The optional `[workspace_models]` policy in a building's `RULES.toml`.
//!
//! The value owns structural validation; the endpoint book owns whether a
//! reference is registered and whether the model offers its effort. Keeping
//! those checks separate lets this crate remain free of provider I/O.
//! Specified by `crates/city/spec/Policy.lean` §8-35.

use serde::{Deserialize, Deserializer, Serialize};

use kernel::{AxCode, AxError, B3Hash, Difficulty, Effort, ModelRef};

/// One difficulty's model and effort recommendation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ModelRecommendation {
    #[serde(flatten)]
    model: ModelRef,
    effort: Effort,
}

/// The TOML face of a recommendation, with the model reference flattened
/// into its table beside `effort`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WrittenRecommendation {
    endpoint: String,
    model: String,
    effort: Effort,
}

impl<'de> Deserialize<'de> for ModelRecommendation {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let written = WrittenRecommendation::deserialize(deserializer)?;
        Ok(ModelRecommendation {
            model: ModelRef {
                endpoint: written.endpoint,
                model: written.model,
            },
            effort: written.effort,
        })
    }
}

impl ModelRecommendation {
    /// The complete provider/model identity this recommendation selects.
    #[must_use]
    pub fn model(&self) -> &ModelRef {
        &self.model
    }

    /// The exact effort the city asks for; it is never moved to a neighbour.
    #[must_use]
    pub fn effort(&self) -> Effort {
        self.effort
    }
}

/// All three recommendations, required together so no difficulty is left
/// to an implicit fallback.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceRecommendations {
    easy: ModelRecommendation,
    medium: ModelRecommendation,
    hard: ModelRecommendation,
}

/// The models a workspace admits and the recommendations for delegated work.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceModelPolicy {
    allowed: Vec<ModelRef>,
    max_effort: Effort,
    recommendations: WorkspaceRecommendations,
    /// The person's direct provider, model and effort. When set, a new
    /// child session uses it instead of the difficulty recommendations.
    #[serde(default)]
    choice: Option<ModelRecommendation>,
}

impl WorkspaceModelPolicy {
    /// The registered models this policy admits.
    #[must_use]
    pub fn allowed(&self) -> &[ModelRef] {
        &self.allowed
    }

    /// The highest effort any recommendation or explicit dispatch may use.
    #[must_use]
    pub fn max_effort(&self) -> Effort {
        self.max_effort
    }

    /// Every recommendation, in the tool vocabulary's order.
    #[must_use]
    pub fn recommendations(&self) -> [&ModelRecommendation; 3] {
        [
            &self.recommendations.easy,
            &self.recommendations.medium,
            &self.recommendations.hard,
        ]
    }

    /// The recommendation for one parent-supplied difficulty.
    #[must_use]
    pub fn recommendation(&self, difficulty: Difficulty) -> &ModelRecommendation {
        match difficulty {
            Difficulty::Easy => &self.recommendations.easy,
            Difficulty::Medium => &self.recommendations.medium,
            Difficulty::Hard => &self.recommendations.hard,
        }
    }

    /// The provider, model and effort the person chose directly, when
    /// they did. Absent, a new child session still follows the difficulty
    /// recommendations.
    #[must_use]
    pub fn choice(&self) -> Option<&ModelRecommendation> {
        self.choice.as_ref()
    }

    /// A stable identity for the policy a child run actually used.
    ///
    /// # Errors
    /// Reports a serialization failure rather than inventing a version.
    pub fn digest(&self) -> Result<B3Hash, AxError> {
        let bytes = serde_json::to_vec(self).map_err(|err| {
            AxError::failure(
                AxCode::ConfigInvalid,
                "digest a workspace model policy",
                err.to_string(),
            )
            .with_recovery("correct the workspace model values, then save the policy again")
        })?;
        Ok(B3Hash::digest(&bytes))
    }

    /// Checks the invariants that need no provider registry.
    pub(super) fn validate(&self) -> Result<(), AxError> {
        if self.allowed.is_empty() {
            return Err(invalid("workspace_models.allowed must not be empty"));
        }
        if self.max_effort == Effort::None {
            return Err(invalid(
                "workspace_models.max_effort must name a thinking level, not `none`",
            ));
        }
        for recommendation in self.recommendations() {
            if !self.allowed.contains(recommendation.model()) {
                return Err(invalid(format!(
                    "recommended model {} is not in workspace_models.allowed",
                    recommendation.model()
                )));
            }
            if recommendation.effort == Effort::None {
                return Err(invalid(format!(
                    "recommended model {} uses `none`, which is not a thinking level",
                    recommendation.model()
                )));
            }
            if recommendation.effort > self.max_effort {
                return Err(invalid(format!(
                    "recommended model {} uses {}, above max_effort {}",
                    recommendation.model(),
                    recommendation.effort.as_str(),
                    self.max_effort.as_str()
                )));
            }
        }
        if let Some(choice) = &self.choice {
            if !self.allowed.contains(choice.model()) {
                return Err(invalid(format!(
                    "chosen model {} is not in workspace_models.allowed",
                    choice.model()
                )));
            }
            if choice.effort == Effort::None {
                return Err(invalid(format!(
                    "chosen model {} uses `none`, which is not a thinking level",
                    choice.model()
                )));
            }
            if choice.effort > self.max_effort {
                return Err(invalid(format!(
                    "chosen model {} uses {}, above max_effort {}",
                    choice.model(),
                    choice.effort.as_str(),
                    self.max_effort.as_str()
                )));
            }
        }
        Ok(())
    }
}

/// Writes `choice` into `text` and leaves every other key. A file with
/// no `[workspace_models]` table gains one whose three recommendations
/// equal the choice, so the section the city already requires is present.
/// An existing recommendation table is left as the shortcut it is.
///
/// An empty base (no `RULES.toml` yet) opens with the ordinary
/// `confidential` / `write` pair the template uses, then the choice
/// section; a non-empty file that still fails evaluation is refused by
/// `write_rules_against`, not rewritten here.
///
/// The result is text for `write_rules_against` to evaluate. This function
/// does not decide that the choice is registered or offered.
pub fn inscribe_choice(text: &str, endpoint: &str, model: &str, effort: Effort) -> String {
    let text = if text.trim().is_empty() {
        "confidential = false\nwrite = \"everything\"\n"
    } else {
        text
    };
    let choice = choice_section(endpoint, model, effort);
    if let Some(rewritten) = replace_section(text, "[workspace_models.choice]", &choice) {
        return ensure_allowed(&rewritten, endpoint, model, effort);
    }
    if has_header(text, "[workspace_models]") {
        let with_choice = insert_before_next_header(text, "[workspace_models]", &choice);
        return ensure_allowed(&with_choice, endpoint, model, effort);
    }
    let mut out = text.to_owned();
    if !out.is_empty() && !out.ends_with('\n') {
        out.push('\n');
    }
    out.push_str(&fresh_section(endpoint, model, effort));
    out
}

fn toml_string(value: &str) -> String {
    toml::Value::String(value.to_owned()).to_string()
}

fn choice_section(endpoint: &str, model: &str, effort: Effort) -> String {
    format!(
        "[workspace_models.choice]\nendpoint = {}\nmodel = {}\neffort = \"{}\"\n",
        toml_string(endpoint),
        toml_string(model),
        effort.as_str()
    )
}

fn fresh_section(endpoint: &str, model: &str, effort: Effort) -> String {
    let word = effort.as_str();
    let endpoint = toml_string(endpoint);
    let model = toml_string(model);
    format!(
        "\n[workspace_models]\nallowed = [{{ endpoint = {endpoint}, model = {model} }}]\nmax_effort = \"{word}\"\n\n[workspace_models.choice]\nendpoint = {endpoint}\nmodel = {model}\neffort = \"{word}\"\n\n[workspace_models.recommendations.easy]\nendpoint = {endpoint}\nmodel = {model}\neffort = \"{word}\"\n\n[workspace_models.recommendations.medium]\nendpoint = {endpoint}\nmodel = {model}\neffort = \"{word}\"\n\n[workspace_models.recommendations.hard]\nendpoint = {endpoint}\nmodel = {model}\neffort = \"{word}\"\n"
    )
}

fn has_header(text: &str, header: &str) -> bool {
    text.lines().any(|line| is_header(line, header))
}

/// A `[table]` line, optionally followed by a TOML line comment.
fn is_header(line: &str, header: &str) -> bool {
    let trimmed = line.trim();
    if trimmed == header {
        return true;
    }
    trimmed
        .strip_prefix(header)
        .is_some_and(|rest| rest.trim_start().starts_with('#'))
}

/// Replaces the body of `header` through the line before the next header.
fn replace_section(text: &str, header: &str, section: &str) -> Option<String> {
    let lines: Vec<&str> = text.lines().collect();
    let start = lines.iter().position(|line| is_header(line, header))?;
    let end = next_header_index(&lines, start, |_| true);
    let mut out = String::new();
    for line in lines.iter().take(start) {
        out.push_str(line);
        out.push('\n');
    }
    out.push_str(section);
    if !section.ends_with('\n') {
        out.push('\n');
    }
    for line in lines.iter().skip(end) {
        out.push_str(line);
        out.push('\n');
    }
    Some(out)
}

fn insert_before_next_header(text: &str, header: &str, section: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let Some(start) = lines.iter().position(|line| is_header(line, header)) else {
        return text.to_owned();
    };
    let end = next_header_index(&lines, start, |trimmed| !is_header(trimmed, header));
    let mut out = String::new();
    for line in lines.iter().take(end) {
        out.push_str(line);
        out.push('\n');
    }
    if !out.ends_with("\n\n") {
        out.push('\n');
    }
    out.push_str(section);
    for line in lines.iter().skip(end) {
        out.push_str(line);
        out.push('\n');
    }
    out
}

/// Index of the next `[...]` header after `start`, or `lines.len()`.
fn next_header_index(lines: &[&str], start: usize, keep: impl Fn(&str) -> bool) -> usize {
    let after = start.saturating_add(1);
    lines
        .iter()
        .enumerate()
        .skip(after)
        .find(|(_, line)| {
            let trimmed = line.trim();
            trimmed.starts_with('[') && keep(trimmed)
        })
        .map_or(lines.len(), |(index, _)| index)
}

fn ensure_allowed(text: &str, endpoint: &str, model: &str, effort: Effort) -> String {
    let pair = format!(
        "endpoint = {}, model = {}",
        toml_string(endpoint),
        toml_string(model)
    );
    let lines: Vec<&str> = text.lines().collect();
    let Some(start) = lines
        .iter()
        .position(|line| is_header(line, "[workspace_models]"))
    else {
        return text.to_owned();
    };
    let end = next_header_index(&lines, start, |_| true);
    let Some(allowed_at) = lines
        .iter()
        .enumerate()
        .skip(start)
        .take(end.saturating_sub(start))
        .find(|(_, line)| line.trim().starts_with("allowed"))
        .map(|(index, _)| index)
    else {
        return insert_after_header(
            text,
            "[workspace_models]",
            &format!("allowed = [{{ {pair} }}]\n"),
        );
    };
    let close_at = lines
        .iter()
        .enumerate()
        .skip(allowed_at)
        .take(end.saturating_sub(allowed_at))
        .find(|(_, line)| line.contains(']'))
        .map_or(allowed_at, |(index, _)| index);
    let pool: String = lines
        .iter()
        .skip(allowed_at)
        .take(close_at.saturating_sub(allowed_at).saturating_add(1))
        .copied()
        .collect::<Vec<_>>()
        .join("\n");
    let rewritten = if pool.contains(&pair) {
        pool
    } else {
        with_pair_in_pool(&pool, &pair)
    };
    let mut out = String::new();
    for (index, line) in lines.iter().enumerate() {
        if index == allowed_at {
            out.push_str(&rewritten);
            if !rewritten.ends_with('\n') {
                out.push('\n');
            }
            continue;
        }
        if index > allowed_at && index <= close_at {
            continue;
        }
        if (start..end).contains(&index) && line.trim().starts_with("max_effort") {
            out.push_str(&raised_max(line, effort));
        } else {
            out.push_str(line);
        }
        out.push('\n');
    }
    out
}

/// Inserts `{ pair }` before the closing `]` of an `allowed` pool that
/// may span several lines (the shape the building template leaves).
fn with_pair_in_pool(pool: &str, pair: &str) -> String {
    let Some(bracket) = pool.rfind(']') else {
        return pool.to_owned();
    };
    let (head, tail) = pool.split_at(bracket);
    let trimmed = head.trim_end();
    let joiner = if trimmed.ends_with('[') {
        ""
    } else if trimmed.ends_with(',') {
        " "
    } else {
        ", "
    };
    format!("{trimmed}{joiner}{{ {pair} }}{tail}")
}

fn raised_max(line: &str, effort: Effort) -> String {
    let Some(quoted) = line.split('"').nth(1) else {
        return line.to_owned();
    };
    let current = Effort::ALL.into_iter().find(|each| each.as_str() == quoted);
    if current.is_some_and(|current| current >= effort) {
        return line.to_owned();
    }
    format!("max_effort = \"{}\"", effort.as_str())
}

fn insert_after_header(text: &str, header: &str, line: &str) -> String {
    let mut out = String::new();
    for existing in text.lines() {
        out.push_str(existing);
        out.push('\n');
        if is_header(existing, header) {
            out.push_str(line);
            if !line.ends_with('\n') {
                out.push('\n');
            }
        }
    }
    out
}

/// A structurally invalid policy, rejected before any rule bytes land.
fn invalid(subject: impl Into<String>) -> AxError {
    AxError::failure(
        AxCode::ConfigInvalid,
        "evaluate a workspace model policy",
        subject.into(),
    )
    .with_recovery(
        "correct [workspace_models] in RULES.toml: keep allowed non-empty, choose every \
             recommendation from it, and keep every effort at or below max_effort",
    )
}
