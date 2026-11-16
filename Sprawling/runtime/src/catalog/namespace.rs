// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{ReservedNamespace, ServerLabel};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Namespace {

    Own(ReservedNamespace),

    Server(ServerLabel),
}

impl Namespace {

    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Namespace::Own(own) => own.as_str(),
            Namespace::Server(label) => label.as_str(),
        }
    }

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

    pub(super) fn member<'a>(&self, name: &'a str) -> &'a str {
        name.strip_prefix(self.as_str())
            .and_then(|rest| rest.strip_prefix('_'))
            .unwrap_or(name)
    }
}

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
