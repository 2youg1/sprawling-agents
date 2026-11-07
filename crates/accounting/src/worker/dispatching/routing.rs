// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Native model selection from an optional workspace policy.
//!
//! The parent supplies only a difficulty. This module resolves the complete
//! endpoint/model reference and exact effort before a child run starts, and
//! validates the same policy when a person saves it. Existing sessions and
//! new sessions in buildings without a policy retain the tag-based path.
//! Specified by `crates/sprawling/spec/Accounting/Worker.lean` §8-173.

use kernel::event::record::{ModelRouting, ModelSelectionSource};
use kernel::{AxCode, AxError, Difficulty, Effort, ModelRef};

use super::super::RunWorker;
use super::Assignment;

impl RunWorker {
    /// Selects a native model and records why it was selected.
    ///
    /// # Errors
    /// Refuses an unavailable recommendation, an explicit choice outside
    /// the workspace policy, or an effort the provider does not offer.
    pub(super) fn route_model(
        &self,
        at: &mut Assignment,
        rules: &city::BuildingRules,
        own: &city::ConfigLayer,
    ) -> Result<(gateway::Chosen<'_>, ModelRouting), AxError> {
        let opens = super::session::opens_a_room(&at.addr, at.session.as_ref());
        let frozen = (!opens).then_some(own).filter(|own| own.model().is_some());
        let workspace = rules.workspace_models().filter(|_| frozen.is_none());
        if let Some(policy) = workspace.filter(|_| at.parent.is_some() && at.model.is_none()) {
            let requested = at.difficulty;
            let difficulty = requested.unwrap_or(Difficulty::Medium);
            let recommendation = policy.recommendation(difficulty);
            let chosen = self
                .credentials
                .book
                .select_ref(recommendation.model(), rules.policy())?;
            admit_recommendation(&chosen, recommendation.effort(), difficulty)?;
            at.effort = Some(recommendation.effort());
            return Ok((
                chosen,
                routing(
                    at,
                    &chosen,
                    Some(policy.digest()?),
                    if requested.is_some() {
                        ModelSelectionSource::Difficulty
                    } else {
                        ModelSelectionSource::MediumDefault
                    },
                ),
            ));
        }

        let chosen = if let Some(own) = frozen
            && let (Some(endpoint), Some(model)) = (own.endpoint(), own.model())
            && at.model.as_deref().is_none_or(|named| named == model)
        {
            self.credentials.book.select_ref(
                &ModelRef {
                    endpoint: endpoint.to_owned(),
                    model: model.to_owned(),
                },
                rules.policy(),
            ).map_err(|error| {
                if *error.code() != AxCode::ConfigInvalid {
                    return error;
                }
                AxError::failure(*error.code(), error.action(), format!("the model changed: {}", error.subject()))
                    .with_recovery(format!("restore the frozen endpoint/model registration, or send this task to another address; {}", error.recovery()))
            })?
        } else {
            let tag = self.tag_for(
                at.model.as_deref(),
                frozen.and_then(city::ConfigLayer::model),
            )?;
            self.credentials.book.select(tag, rules.policy())?
        };
        let manually_named = at.model.is_some() || at.effort.is_some();
        let policy = if manually_named {
            workspace
                .map(|policy| {
                    let judged = if opens {
                        city::Building::of(&at.addr)?.addr().clone()
                    } else {
                        at.addr.clone()
                    };
                    let effective = match at.effort {
                        Some(effort) => Some(effort),
                        None => city::settled_effort(&self.city_root, &judged)?
                            .map(|(effort, _)| effort),
                    };
                    admit_explicit(policy, &chosen, effective)?;
                    policy.digest()
                })
                .transpose()?
        } else {
            None
        };
        let source = if manually_named {
            ModelSelectionSource::Manual
        } else {
            ModelSelectionSource::Main
        };
        Ok((chosen, routing(at, &chosen, policy, source)))
    }

    /// Validates registry-dependent policy facts before a save writes them.
    ///
    /// # Errors
    /// Refuses references not registered on an attached endpoint and known
    /// thinking offers that do not contain a recommended level.
    pub(in crate::worker) fn validate_workspace_models(
        &self,
        rules: &city::BuildingRules,
    ) -> Result<(), AxError> {
        let Some(policy) = rules.workspace_models() else {
            return Ok(());
        };
        let recommendations = policy.recommendations();
        for (difficulty, recommendation) in [Difficulty::Easy, Difficulty::Medium, Difficulty::Hard]
            .into_iter()
            .zip(recommendations)
        {
            let chosen = self
                .credentials
                .book
                .select_ref(recommendation.model(), rules.policy())?;
            admit_recommendation(&chosen, recommendation.effort(), difficulty)?;
        }
        for allowed in policy.allowed() {
            if recommendations
                .iter()
                .any(|recommendation| recommendation.model() == allowed)
            {
                continue;
            }
            self.credentials.book.select_ref(allowed, rules.policy())?;
        }
        Ok(())
    }
}

/// Applies the workspace's constraints to a person's explicit selection.
fn admit_explicit(
    policy: &city::WorkspaceModelPolicy,
    chosen: &gateway::Chosen<'_>,
    effort: Option<Effort>,
) -> Result<(), AxError> {
    let model = ModelRef {
        endpoint: chosen.endpoint.name.clone(),
        model: chosen.entry.id.clone(),
    };
    if !policy.allowed().contains(&model) {
        return Err(AxError::failure(
            AxCode::ConfigInvalid,
            "dispatch a model allowed by the workspace",
            format!("{model} is not in workspace_models.allowed"),
        )
        .with_recovery(
            "choose an allowed model, or update [workspace_models] in this building's RULES.toml",
        ));
    }
    if let Some(effort) = effort
        && effort > policy.max_effort()
    {
        return Err(AxError::failure(
            AxCode::ConfigInvalid,
            "dispatch within the workspace effort limit",
            format!(
                "{} is above workspace max_effort {}",
                effort.as_str(),
                policy.max_effort().as_str()
            ),
        )
        .with_recovery(
            "choose an effort at or below max_effort, or update the workspace policy first",
        ));
    }
    Ok(())
}

/// The ledger fact for the selection just made.
fn routing(
    at: &Assignment,
    chosen: &gateway::Chosen<'_>,
    policy: Option<kernel::B3Hash>,
    source: ModelSelectionSource,
) -> ModelRouting {
    ModelRouting {
        difficulty: at.difficulty,
        model: ModelRef {
            endpoint: chosen.endpoint.name.clone(),
            model: chosen.entry.id.clone(),
        },
        policy,
        source,
    }
}

/// Keeps provider admission authoritative while naming the policy a person can repair.
fn admit_recommendation(
    chosen: &gateway::Chosen<'_>,
    effort: Effort,
    difficulty: Difficulty,
) -> Result<(), AxError> {
    let offer = gateway::thinking_offer(chosen.endpoint, &chosen.entry.id);
    offer.admit(Some(effort), &chosen.entry.id).map_err(|error| {
        let difficulty = match difficulty {
            Difficulty::Easy => "easy",
            Difficulty::Medium => "medium",
            Difficulty::Hard => "hard",
        };
        let levels = offer.levels.iter().map(Effort::as_str).collect::<Vec<_>>();
        AxError::failure(*error.code(), error.action(), error.subject()).with_recovery(format!(
            "correct [workspace_models.recommendations.{difficulty}] in this building's RULES.toml; provider supports effort levels: {}",
            if levels.is_empty() { "none".to_owned() } else { levels.join(", ") }
        ))
    })
}
