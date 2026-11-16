// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use winreg::enums::HKEY_CURRENT_USER;

use super::super::target::RawValue;
use super::registry::{self, Reading, WriteFault};

const USER_ENVIRONMENT: &str = "Environment";

pub(in crate::privacy) enum Written {

    Announced,

    Unannounced(kernel::AxError),
}

pub(in crate::privacy) fn read(name: &str) -> Reading {
    registry::read(HKEY_CURRENT_USER, USER_ENVIRONMENT, name)
}

pub(in crate::privacy) fn write(name: &str, value: &RawValue) -> Result<Written, WriteFault> {
    registry::write(HKEY_CURRENT_USER, USER_ENVIRONMENT, name, value)?;
    Ok(match crate::environment_broadcast::announce() {
        Ok(()) => Written::Announced,
        Err(failure) => Written::Unannounced(failure),
    })
}
