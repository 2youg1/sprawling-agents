// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Slash {
    Help,
    Room,
    New,
    Stop,
    Halt,
    Release,
    Model,
    Effort,
    Approve,
    Deny,
    Web,
    Quit,
    Serving,
    Remote,
    Acp,
    Wire,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Offered {
    Cli,
    WebUi,
    Both,
}

impl Offered {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Cli => "cli",
            Self::WebUi => "webui",
            Self::Both => "both",
        }
    }

    #[must_use]
    pub const fn in_the_cli(self) -> bool {
        match self {
            Self::Cli | Self::Both => true,
            Self::WebUi => false,
        }
    }
}

impl Slash {
    pub const ALL: [Slash; 16] = [
        Self::Help,
        Self::Room,
        Self::New,
        Self::Stop,
        Self::Halt,
        Self::Release,
        Self::Model,
        Self::Effort,
        Self::Approve,
        Self::Deny,
        Self::Web,
        Self::Quit,
        Self::Serving,
        Self::Remote,
        Self::Acp,
        Self::Wire,
    ];

    #[must_use]
    pub const fn spelling(self) -> &'static str {
        match self {
            Self::Help => "/help",
            Self::Room => "/room",
            Self::New => "/new",
            Self::Stop => "/stop",
            Self::Halt => "/halt",
            Self::Release => "/release",
            Self::Model => "/model",
            Self::Effort => "/effort",
            Self::Approve => "/approve",
            Self::Deny => "/deny",
            Self::Web => "/web",
            Self::Quit => "/quit",
            Self::Serving => "/serving",
            Self::Remote => "/remote",
            Self::Acp => "/acp",
            Self::Wire => "/wire",
        }
    }

    #[must_use]
    pub const fn takes(self) -> &'static str {
        match self {
            Self::Help | Self::New | Self::Stop | Self::Web | Self::Quit | Self::Serving => "",
            Self::Room => "<addr>",
            Self::Halt | Self::Release => "[--all]",
            Self::Model | Self::Approve | Self::Deny => "[<id>]",
            Self::Effort => "[<level>]",
            Self::Remote => "<verb>",
            Self::Acp => "[<text>]",
            Self::Wire => "<verb> [<json>]",
        }
    }

    #[must_use]
    pub const fn offered(self) -> Offered {
        match self {
            Self::Help
            | Self::Room
            | Self::New
            | Self::Stop
            | Self::Halt
            | Self::Release
            | Self::Model
            | Self::Effort
            | Self::Quit
            | Self::Acp => Offered::Both,
            Self::Approve | Self::Deny | Self::Web | Self::Serving | Self::Remote | Self::Wire => {
                Offered::Cli
            }
        }
    }

    #[must_use]
    pub const fn says(self) -> &'static str {
        match self {
            Self::Help => "list these verbs",
            Self::Room => "choose the room plain lines go to",
            Self::New => "start a new session in this room",
            Self::Stop => "stop the run working in this room",
            Self::Halt => "hold this room's building, or with --all the whole city",
            Self::Release => "let this room's building work again, or with --all the whole city",
            Self::Model => "show the model, or choose one the city's endpoints offer",
            Self::Effort => "show the thinking level, or choose one the model offers",
            Self::Approve => "approve the waiting request, or the one with this id",
            Self::Deny => "deny the waiting request, or the one with this id",
            Self::Web => "open the WebUI and quiet this terminal; Esc comes back",
            Self::Quit => "close the city",
            Self::Serving => "where this city listens, and what is running in it",
            Self::Remote => {
                "open|pair|close|devices|revoke: the door a device reaches the city through"
            }
            Self::Acp => "find an ACP agent, or read a pasted launch line",
            Self::Wire => "send any wire command or question, its body as JSON",
        }
    }

    #[must_use]
    pub fn parse(word: &str) -> Option<Slash> {
        let bare = word.strip_prefix('/').unwrap_or(word);
        Self::ALL
            .into_iter()
            .find(|verb| verb.spelling().strip_prefix('/') == Some(bare))
    }
}
