// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Which directory entries a search walk descends into (runtime-SPEC,
//! the `search` section).

use std::path::PathBuf;

use kernel::{Address, ReadVerdict};

/// What the walk does with one directory entry.
pub(super) enum Entry {
    Descend(String),
    /// Left out by policy: no address, the reserved subtree, or a
    /// confidential building.
    Passed,
    /// A building closed because its rules did not read: a failure a
    /// person has to fix, so it is named rather than passed over.
    Unread {
        child: String,
        why: String,
    },
}

/// One item on the walk's stack. A building left unread waits on the
/// stack beside the directories, so `unread` names it where the walk
/// meets it: the SPEC lists `unread` in walk order.
pub(super) enum Step {
    Visit(PathBuf, String),
    Unread { child: String, why: String },
}

/// The child address the walk may descend to, or why it may not.
///
/// Reservedness is asked of `kernel::Address`, the same primitive
/// `read` reaches through `chosen_path`; a name that cannot be an
/// address at all — one holding a backslash, a colon, a control
/// character — is left alone, because this city cannot say where it is.
/// Git's own metadata is inside that predicate too (kernel-SPEC 8-73),
/// and scanning an object store yields hits nobody can act on.
///
/// The read bound is asked only at the city root: it answers for a
/// building, a building is a top-level address, and all below one
/// shares its answer, so no file costs another read of the rules.
pub(super) fn admissible(rel: &str, name: &str, bound: &dyn Fn(&Address) -> ReadVerdict) -> Entry {
    let child = if rel.is_empty() {
        name.to_owned()
    } else {
        format!("{rel}/{name}")
    };
    let Ok(addr) = Address::parse(&child) else {
        return Entry::Passed;
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
