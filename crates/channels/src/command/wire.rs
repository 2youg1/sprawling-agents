// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Everything a client may ask the city to do, and the two invariants
//! that live in the type system rather than in a check.
//!
//! - `Command` is generic over the carrier of a secret. `WireCommand`
//!   fixes that carrier to an uninhabited type, so a frame arriving from
//!   a socket cannot be a `PutSecret` — not "is rejected", but has no
//!   representation. Credentials are enrolled on the host machine, and
//!   that constraint is held by construction.
//! - Every state-changing Command owns an `IdemKey` field. There is no
//!   constructor that omits it, so "double-clicking twice opens two
//!   runs" is not reachable from this type.
//!
//! The enum is not `#[non_exhaustive]`: the wire version is the
//! versioning mechanism, so the assembly layer must handle every variant
//! and a new one fails to compile until somebody decides what it does.
//! That is what keeps a button off the client until the city can answer
//! the frame behind it.

//! The wire face: no-secret commands and back.

use kernel::IdemKey;

use super::kind::{Command, NoSecret};

/// The Command set a socket can carry. `PutSecret` is unreachable because
/// `NoSecret` has no values.
pub type WireCommand = Command<NoSecret>;

impl<Secret> Command<Secret> {
    /// Exhaustive by construction: a new variant cannot compile without
    /// choosing its name here, which is what keeps [`COMMAND_NAMES`] honest.
    #[must_use]
    pub fn name(&self) -> &'static str {
        match *self {
            Self::Dispatch { .. } => "Dispatch",
            Self::Wake { .. } => "Wake",
            Self::Login { .. } => "Login",
            Self::ConfigureBuilding { .. } => "ConfigureBuilding",
            Self::ProbeEndpoint { .. } => "ProbeEndpoint",
            Self::AttachEndpoint { .. } => "AttachEndpoint",
            Self::SelectModel { .. } => "SelectModel",
            Self::Fork { .. } => "Fork",
            Self::Attach { .. } => "Attach",
            Self::CreateBuilding { .. } => "CreateBuilding",
            Self::PutSecret { .. } => "PutSecret",
            Self::Steer { .. } => "Steer",
            Self::Cancel { .. } => "Cancel",
            Self::Takeover { .. } => "Takeover",
            Self::Rollback { .. } => "Rollback",
            Self::Halt { .. } => "Halt",
            Self::Reveal { .. } => "Reveal",
            Self::Release { .. } => "Release",
            Self::BatchByBuilding { .. } => "BatchByBuilding",
            Self::Approve { .. } => "Approve",
            Self::CreatePolicy { .. } => "CreatePolicy",
            Self::SetAutonomy { .. } => "SetAutonomy",
            Self::Pursue { .. } => "Pursue",
            Self::PutDocument { .. } => "PutDocument",
            Self::Auth { .. } => "Auth",
        }
    }

    /// The deduplication key. `None` only for the two Commands that change
    /// nothing: `Auth`, and `PutSecret` whose effect is confined to the host
    /// process and whose replay writes the same Vault entry.
    pub fn idem(&self) -> Option<&IdemKey> {
        match *self {
            Self::Dispatch { ref idem, .. }
            | Self::Wake { ref idem, .. }
            | Self::Login { ref idem, .. }
            | Self::Fork { ref idem, .. }
            | Self::ProbeEndpoint { ref idem, .. }
            | Self::ConfigureBuilding { ref idem, .. }
            | Self::Attach { ref idem, .. }
            | Self::CreateBuilding { ref idem, .. }
            | Self::Steer { ref idem, .. }
            | Self::Cancel { ref idem, .. }
            | Self::Takeover { ref idem, .. }
            | Self::Rollback { ref idem, .. }
            | Self::Halt { ref idem, .. }
            | Self::Release { ref idem, .. }
            | Self::BatchByBuilding { ref idem, .. }
            | Self::Approve { ref idem, .. }
            | Self::CreatePolicy { ref idem, .. }
            | Self::SetAutonomy { ref idem, .. }
            | Self::Pursue { ref idem, .. }
            | Self::PutDocument { ref idem, .. }
            | Self::AttachEndpoint { ref idem, .. }
            | Self::SelectModel { ref idem, .. }
            | Self::Reveal { ref idem, .. } => Some(idem),
            Self::PutSecret { .. } | Self::Auth { .. } => None,
        }
    }
}

impl From<WireCommand> for Command {
    /// Widening a wire frame into the in-process Command set. Total: the
    /// `PutSecret` arm is unreachable because its payload cannot exist.
    fn from(wire: WireCommand) -> Self {
        match wire {
            Command::Wake {
                source,
                subject,
                body,
                idem,
            } => Self::Wake {
                source,
                subject,
                body,
                idem,
            },
            Command::Dispatch {
                addr,
                task,
                goal,
                mode,
                idem,
                session,
                effort,
            } => Self::Dispatch {
                addr,
                task,
                goal,
                mode,
                idem,
                session,
                effort,
            },
            Command::Login {
                provider,
                step,
                idem,
            } => Self::Login {
                provider,
                step,
                idem,
            },
            Command::ConfigureBuilding {
                addr,
                sandbox,
                mcp,
                desktop,
                idem,
            } => Self::ConfigureBuilding {
                addr,
                sandbox,
                mcp,
                desktop,
                idem,
            },
            Command::ProbeEndpoint {
                name,
                base_url,
                dialect,
                secret,
                auth_header,
                idem,
            } => Self::ProbeEndpoint {
                name,
                base_url,
                dialect,
                secret,
                auth_header,
                idem,
            },
            Command::AttachEndpoint {
                name,
                base_url,
                dialect,
                secret,
                auth_header,
                admit,
                idem,
            } => Self::AttachEndpoint {
                name,
                base_url,
                dialect,
                secret,
                auth_header,
                admit,
                idem,
            },
            Command::SelectModel {
                endpoint,
                model,
                tag,
                context_tokens,
                max_output_tokens,
                idem,
            } => Self::SelectModel {
                endpoint,
                model,
                tag,
                context_tokens,
                max_output_tokens,
                idem,
            },
            Command::Fork {
                run,
                at_seq,
                addr,
                idem,
            } => Self::Fork {
                run,
                at_seq,
                addr,
                idem,
            },
            Command::Attach {
                upload,
                notify,
                idem,
            } => Self::Attach {
                upload,
                notify,
                idem,
            },
            Command::CreateBuilding {
                addr,
                template,
                idem,
            } => Self::CreateBuilding {
                addr,
                template,
                idem,
            },
            Command::PutSecret { value, .. } => match value {},
            Command::Steer { run, text, idem } => Self::Steer { run, text, idem },
            Command::Cancel { run, idem } => Self::Cancel { run, idem },
            Command::Takeover { run, idem } => Self::Takeover { run, idem },
            Command::Rollback { checkpoint, idem } => Self::Rollback { checkpoint, idem },
            Command::Halt { scope, idem } => Self::Halt { scope, idem },
            Command::Reveal { at, idem } => Self::Reveal { at, idem },
            Command::Release { scope, idem } => Self::Release { scope, idem },
            Command::BatchByBuilding { addr, idem } => Self::BatchByBuilding { addr, idem },
            Command::Approve {
                item,
                verdict,
                idem,
            } => Self::Approve {
                item,
                verdict,
                idem,
            },
            Command::CreatePolicy { from_item, idem } => Self::CreatePolicy { from_item, idem },
            Command::Pursue { addr, step, idem } => Self::Pursue { addr, step, idem },
            Command::SetAutonomy {
                scope,
                autonomy,
                idem,
            } => Self::SetAutonomy {
                scope,
                autonomy,
                idem,
            },
            Command::PutDocument { which, body, idem } => Self::PutDocument { which, body, idem },
            Command::Auth { token } => Self::Auth { token },
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::float_arithmetic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::string_slice,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]
mod tests {
    use super::*;
    /// The runtime half of "a credential cannot be enrolled over a
    /// connection": the compile-time half is `NoSecret` having no values,
    /// and this is what happens to bytes that try anyway.
    #[test]
    fn a_wire_frame_carrying_put_secret_fails_to_decode() {
        let json = r#"{"put_secret":{"realm":"anthropic","name":"api","value":"sk-nope"}}"#;
        let decoded: Result<WireCommand, _> = serde_json::from_str(json);
        let err = decoded.expect_err("PutSecret has no wire form");
        assert!(
            !err.to_string().contains("sk-nope"),
            "the refusal must not echo the bytes it protects"
        );
    }
}
