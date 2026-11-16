// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Where the catalog files an admitted tool (`crates/runtime/spec/Catalog.lean`
//! §8-60): under a namespace the city keeps for its own tools, or under
//! the label of the server that offers it. The dormant index gives each
//! namespace one line, and `tool list` opens one.

use kernel::{ReservedNamespace, ServerLabel};

/// The namespace one admitted tool is filed under, said by the assembler
/// that admits it: it is the one place that knows whether a tool is the
/// city's own or a server's.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Namespace {
    /// One of the namespaces the city keeps for its own tools.
    Own(ReservedNamespace),
    /// An external server's tools, filed under its label. Every tool it
    /// offers is spelled `{label}_{tool}`, and a label holds no `_`.
    Server(ServerLabel),
}

impl Namespace {
    /// The name the index prints and `tool list` takes.
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Namespace::Own(own) => own.as_str(),
            Namespace::Server(label) => label.as_str(),
        }
    }

    /// What one line of the index says about this namespace: a sentence
    /// of the city's own for a namespace it keeps, and the members' names
    /// without the label for a server, which says nothing of its own here.
    pub(super) fn about<'a>(&self, members: impl Iterator<Item = &'a str>) -> String {
        match self {
            Namespace::Own(own) => own_about(*own).to_owned(),
            Namespace::Server(label) => members
                .map(|name| {
                    name.strip_prefix(label.as_str())
                        .and_then(|rest| rest.strip_prefix('_'))
                        .unwrap_or(name)
                })
                .collect::<Vec<_>>()
                .join(", "),
        }
    }

    /// The member's name without this namespace's prefix, when it has
    /// one: what a model asking for `create_issue` means by
    /// `github_create_issue`.
    pub(super) fn member<'a>(&self, name: &'a str) -> &'a str {
        name.strip_prefix(self.as_str())
            .and_then(|rest| rest.strip_prefix('_'))
            .unwrap_or(name)
    }
}

/// The sentence a namespace the city keeps is introduced by.
fn own_about(own: ReservedNamespace) -> &'static str {
    match own {
        ReservedNamespace::Computer => "drive a browser: the building's own, or the person's",
        ReservedNamespace::Media => "read the text in a picture or a recording",
        ReservedNamespace::Record => "look back at what this building did, and propose changes",
        ReservedNamespace::Skill => "the steps this building follows for a kind of work",
        ReservedNamespace::Sprawling => {
            "talk to other residents, split and hand off work, see your run"
        }
        ReservedNamespace::Web => "search the web through the city's own supplier",
    }
}
