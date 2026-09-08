// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The L0 three. Index only: no logic lives here.

mod chosen_path;
mod edit;
mod exec;
mod read;
mod search;
mod status;
mod succeed;

pub use edit::EditTool;
pub use edit::version_of;
pub use exec::parse_arm;
pub use exec::{ExecSetup, ExecTool};
pub use read::ReadTool;
pub use search::SearchTool;
pub use status::ChildStatus;
pub use status::ProviderMode;
pub use status::StatusSnapshot;
pub use status::StatusTool;
pub use succeed::{SucceedTool, Succession, SuccessionDesk};
