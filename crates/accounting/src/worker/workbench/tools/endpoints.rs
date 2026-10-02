// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The tools that hand bytes this run may read to an endpoint the book
//! names: `transcribe`, then `ocr` (`crates/sprawling/Spec.lean` §8-131, §8-142).
//!
//! They are one phase of laying out the bench because they share one
//! door: a `runtime::BoundReader` over the read bound `read` and
//! `search` ask, the tree the run reads in and the city's block store,
//! so what one tool may open the others may send (`crates/runtime/Spec.lean` §8-59).

use std::sync::Arc;

use kernel::AxError;

use crate::worker::workbench::{Laying, Site};

impl Laying {
    /// The tools of this phase that the book offers this building, in
    /// their order on the table.
    ///
    /// # Errors
    /// Propagates a facility that cannot be built for its chosen
    /// endpoint.
    pub(super) fn endpoint_tools(
        &self,
        site: &Site,
        bound: &runtime::ReadBound,
    ) -> Result<Vec<Box<dyn kernel::Tool>>, AxError> {
        let reader = runtime::BoundReader::new(
            &site.write_root,
            Arc::clone(bound),
            &kernel::layout::CityLayout::new(&self.city_root).cas(),
        );
        let mut offered: Vec<Box<dyn kernel::Tool>> = Vec::new();
        if let Some(transcribe) = self.transcription_tool(site, reader.clone())? {
            offered.push(Box::new(transcribe));
        }
        if let Some(ocr) = self.ocr_tool(site, reader)? {
            offered.push(Box::new(ocr));
        }
        Ok(offered)
    }
}
