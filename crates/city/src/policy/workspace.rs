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
        Ok(())
    }
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
