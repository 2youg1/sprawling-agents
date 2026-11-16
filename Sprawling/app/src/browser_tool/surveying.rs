// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use browser::survey;
use kernel::{AxCode, AxError};
use serde_json::Value;

pub(super) fn surveyed(result: &Value) -> Result<String, AxError> {
    let read: survey::probe::Read =
        serde_json::from_value(browser::read_json(result)?).map_err(|source| {
            AxError::failure(
                AxCode::ToolOutcomeUnknown,
                "survey a page",
                format!("the page answered a measurement with {source}"),
            )
            .with_recovery(
                "report this against browser::survey::probe: the probe and its reader disagree",
            )
        })?;

    let painted_by = match read.conditions() {
        Some(("1", _)) => survey::PaintSource::TheSystem,
        _ => survey::PaintSource::ThePage,
    };
    let page = read.page(painted_by);
    let found = survey::judge(&page);
    Ok(survey::written(
        survey::Survey {
            page: &page,
            at: "",
            found: &found,
            sources: &survey::Sources::default(),
        },
        survey::Shape::Tagged,
    ))
}
