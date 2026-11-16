// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::PathBuf;

use kernel::{Address, ReadVerdict};

pub(super) enum Entry {
    Descend(String),

    Passed,

    Unread {
        child: String,
        why: String,
    },
}

pub(super) enum Step {
    Visit(PathBuf, String),
    Unread { child: String, why: String },
}

pub(super) fn admissible(rel: &str, name: &str, bound: &dyn Fn(&Address) -> ReadVerdict) -> Entry {
    let child = if rel.is_empty() {
        name.to_owned()
    } else {
        format!("{rel}/{name}")
    };
    let addr = match Address::parse(&child) {
        Ok(addr) => addr,
        Err(refused) => {
            return Entry::Unread {
                why: format!("its name is not an address: {}", refused.recovery()),
                child,
            };
        }
    };
    if addr.is_reserved() {
        return Entry::Passed;
    }
    if !rel.is_empty() {
        return Entry::Descend(child);
    }
    match bound(&addr) {
        ReadVerdict::Open => Entry::Descend(child),
        ReadVerdict::Confidential => Entry::Passed,
        ReadVerdict::RulesUnreadable(unread) => Entry::Unread {
            why: format!("its rules did not read: {}", unread.subject()),
            child,
        },
    }
}
