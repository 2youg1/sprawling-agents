// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Turning a URL and a key into a model a run can be given (web-SPEC.md section 8-12).
//!
//! Index only.

mod attach;
mod choose;
mod forms;
mod listing;
mod login;
mod page;
mod tables;
#[cfg(test)]
mod tests;

pub use forms::{
    AttachForm, AttachReadiness, SelectForm, SelectReadiness, ready, select_ready, url_is_safe,
};
pub use page::Settings;
pub use tables::{EndpointRow, TagRow, can_dispatch, endpoint_rows, tag_rows};
