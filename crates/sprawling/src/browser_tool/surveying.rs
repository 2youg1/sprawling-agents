// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A page judged, for a resident that has no repository beside it.

use browser::survey;
use kernel::{AxCode, AxError};
use serde_json::Value;

/// The page's own readings, judged and written for whoever asked.
///
/// The source index is empty here and that is the honest state: a
/// resident surveying a page has no repository beside it, so every
/// finding names the box and none of them names a line.
///
/// # Errors
/// Refuses a reply that is not the three strings the probe promised,
/// because a page measured into a shape this version cannot read is not
/// a clean page.
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
    // What the page says it was painted by, because nobody asked it to
    // be painted any particular way.
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
