// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::event::record::PromptShapeCompared;
use kernel::{AxCode, AxError};
use serde::Deserialize;

pub fn rebuild_shape(
    previous: Option<(&serde_json::Value, &serde_json::Value)>,
    assembled: &serde_json::Value,
    compared: &serde_json::Value,
) -> Result<(), AxError> {
    let (rebuilt, recorded) = rebuild_one(assembled, compared)?;
    let prior = match previous {
        Some((prior_assembled, prior_compared)) => {
            Some(rebuild_one(prior_assembled, prior_compared)?.0)
        }
        None => None,
    };
    if rebuilt.attribute(prior.as_ref()) != recorded.changed {
        return Err(shape_line_disagrees(
            "its attribution names regions that did not move",
        ));
    }
    let bound = rebuilt.request_upper_bound()?;
    if bound != recorded.upper_bound {
        return Err(shape_line_disagrees(
            "its billing bound is not what its own parts add up to",
        ));
    }
    Ok(())
}

fn rebuild_one(
    assembled: &serde_json::Value,
    compared: &serde_json::Value,
) -> Result<(crate::prefix::shape::PromptShape, PromptShapeCompared), AxError> {
    let recorded = PromptShapeCompared::deserialize(compared).map_err(|err| {
        AxError::failure(
            AxCode::CasCorrupt,
            "rebuild the prompt shape",
            format!("a prompt_shape_compared payload does not parse: {err}"),
        )
        .with_recovery(
            "replay with the build that wrote this ledger, or report the damaged line: its \
             prompt_shape_compared payload is not what this build reads",
        )
    })?;
    let parts = crate::prefix::shape::segment_parts_of(assembled)?;
    Ok((
        crate::prefix::shape::PromptShape::from_parts(parts, recorded.tools, recorded.run),
        recorded,
    ))
}

fn shape_line_disagrees(why: &str) -> AxError {
    AxError::failure(
        AxCode::CasCorrupt,
        "rebuild the prompt shape",
        format!("a prompt_shape_compared line disagrees with its own records: {why}"),
    )
    .with_recovery(
        "replay with the build that wrote this ledger, or report the damaged line: its recorded \
         facts do not add up to its recorded judgement",
    )
}
