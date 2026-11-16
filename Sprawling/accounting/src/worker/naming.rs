// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{Address, AxCode, AxError};

pub(super) fn governed_of(which: wire::GovernedDocument) -> city::Governed {
    match which {
        wire::GovernedDocument::Mayor => city::Governed::Mayor,
        wire::GovernedDocument::Clerk => city::Governed::Clerk,
        wire::GovernedDocument::Preferences => city::Governed::Preferences,
    }
}

pub(super) fn naming_edit_of(card: &wire::IdentityCard) -> city::NamingEdit {
    match card {
        wire::IdentityCard::Person {
            user_id,
            imported_from,
            about,
        } => city::NamingEdit::Person {
            user_id: user_id.clone(),
            imported_from: imported_from.clone(),
            about: about.clone(),
        },
        wire::IdentityCard::Mayor { name } => city::NamingEdit::Mayor { name: name.clone() },
    }
}

pub(super) fn not_built(action: &'static str, subject: String, instead: &'static str) -> AxError {
    AxError::failure(AxCode::WireMismatch, action, subject).with_recovery(instead)
}

pub(super) fn building_of(addr: &Address) -> Option<Address> {
    Address::parse(addr.as_str().split('/').next()?).ok()
}

pub(super) fn scope_of(scope: &wire::HaltScope) -> kernel::event::Scope {
    match scope {
        wire::HaltScope::City => kernel::event::Scope::City,
        wire::HaltScope::Building(addr) => kernel::event::Scope::Building(addr.clone()),
        wire::HaltScope::Workshop(addr) => kernel::event::Scope::Workshop(addr.clone()),
    }
}
