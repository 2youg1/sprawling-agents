// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

pub mod acp;
pub mod agent;
pub mod capture;
pub mod catalog;
mod child_exit;
pub mod doctor;
pub mod error;
pub mod launch;
pub mod observation;
pub mod python;
pub mod request;
pub mod resume;
pub mod steer;
pub mod transport;
pub mod workspace;

pub use error::{Error, Result};
