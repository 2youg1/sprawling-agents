// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

mod bound_reader;
mod chosen_path;
mod edit;
mod exec;
mod path_lock;
#[cfg(windows)]
pub(crate) use exec::native_windows;
mod read;
mod search;
mod shown;
mod stamp;
mod status;
mod succeed;
mod tool;

pub use bound_reader::{BoundReader, Named, Opened};
pub use chosen_path::ReadBound;
#[cfg(test)]
pub(crate) use chosen_path::admit;
pub use edit::{EditDesk, EditTool};
pub(crate) use exec::container::ContainerLease;
pub use exec::container::run_container_guard;
pub use exec::parse_arm;
pub use exec::parse_placement;
pub(crate) use exec::program_line;
pub use exec::{ContainerEngine, ContainerLaunch, ContainerRuntime};
pub use exec::{ExecSetup, ExecTool};
pub use exec::{FailureClass, Shell, ShellCount, ShellTally};
pub use exec::{Placed, Placement};
pub use path_lock::PathLocks;
pub use stamp::ReadStamps;
#[cfg(test)]
pub(crate) use stamp::Stamp;

pub use exec::{Assurances, Confined, Confinement, Guarantee, Kept, Missing, Offerings};
pub use read::{ReadClock, ReadTool, RuleCandidate, RuleCandidates, RuleContext};
pub use search::SearchTool;
pub use shown::ReadShown;
#[cfg(test)]
pub(crate) use shown::Span;
pub use status::ChildStatus;
pub use status::ProviderMode;
pub use status::StatusSnapshot;
pub use status::StatusTool;
pub use succeed::{SucceedTool, Succession, SuccessionDesk};
pub use tool::ToolTool;
