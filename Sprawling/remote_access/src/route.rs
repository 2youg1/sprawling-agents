// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The route seam: what makes the remote listener on this computer's
//! loopback reachable from outside, and nothing more
//! (crates/remote_access/Spec.lean §8-7).
//!
//! A route carries bytes. The handshake authenticates keys pinned at
//! pairing rather than an address, and every frame is sealed end to end,
//! so a route that sees, forwards or changes frames learns nothing and
//! can impersonate no one; the one thing a caller needs from it is an
//! address a browser treats as a secure context, which [`PublicUrl`]
//! makes the only kind there is.
//!
//! Three implementations: [`cloudflare::NamedTunnel`], the default,
//! [`command::CommandRoute`], a command the person wrote, and
//! [`scripted::ScriptedRoute`], which answers a fixed address for tests.

use std::net::SocketAddr;
use std::process::{Child, ChildStdout, Command, ExitStatus};

use kernel::{AxCode, AxError};

pub mod cloudflare;
pub mod command;
pub mod scripted;
#[cfg(test)]
mod tests;

/// Makes `local` reachable from outside, and stops.
///
/// Both methods block: `open` until the route is ready or the patience
/// its implementation was built with runs out. The caller runs them on a
/// thread of its own (remote_access D15).
pub trait Route {
    /// Opens the route to `local` and answers the address outside uses.
    /// A route that is already open is closed first.
    ///
    /// # Errors
    /// `E_TOOL_UNAVAILABLE` when the program behind the route cannot be
    /// started or stops before it is ready, `E_TIMEOUT` when it is not
    /// ready in time, and `E_CONFIG_INVALID` when the address it gives is
    /// not one a page can use.
    fn open(&mut self, local: SocketAddr) -> Result<Opened, AxError>;

    /// Closes the route. Closing a route that is not open succeeds, so a
    /// door that expires as the person closes it asks nothing of either.
    ///
    /// # Errors
    /// `E_TOOL_UNAVAILABLE` when the process holding the route open could
    /// not be ended.
    fn close(&mut self) -> Result<(), AxError>;
}

/// What an open route answers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Opened {
    pub url: PublicUrl,
    pub permanence: Permanence,
}

/// An `https://` address with a host: the only kind on which a page can
/// run the handshake and install as an app.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicUrl(String);

/// Whether a route's host name survives the route being restarted. A
/// device keeps its key and its installed page per host name, so a host
/// that changes leaves every paired device unable to read its key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Permanence {
    Fixed,
    PerStart,
}

impl PublicUrl {
    /// Reads an address a route gave, with the scheme in either case.
    ///
    /// # Errors
    /// `E_CONFIG_INVALID` for another scheme, no host, a user name before
    /// the host, a fragment, and whitespace or control characters. The
    /// invitation appends a fragment of its own, and the address is
    /// printed into a QR code.
    pub fn parse(text: &str) -> Result<Self, AxError> {
        let (scheme, rest) = text
            .split_once("://")
            .ok_or_else(|| unusable(text, "no scheme"))?;
        if !scheme.eq_ignore_ascii_case("https") {
            return Err(unusable(text, "not https"));
        }
        if text
            .chars()
            .any(|each| each.is_whitespace() || each.is_control())
        {
            return Err(unusable(text, "a blank or a control character"));
        }
        if text.contains('#') {
            return Err(unusable(text, "a fragment"));
        }
        let authority = rest
            .split(['/', '?'])
            .next()
            .ok_or_else(|| unusable(text, "no host"))?;
        if authority.is_empty() || authority.starts_with(':') {
            return Err(unusable(text, "no host"));
        }
        if authority.contains('@') {
            return Err(unusable(text, "a user name before the host"));
        }
        Ok(Self(format!("https://{rest}")))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

fn unusable(text: &str, why: &str) -> AxError {
    AxError::failure(
        AxCode::ConfigInvalid,
        "read a route's address",
        format!("{text}: {why}"),
    )
    .with_recovery(
        "give the route an https:// address with a host, no user name and no #fragment: \
         a page needs a secure context, and the invitation adds its own fragment",
    )
}

/// The process that holds a route open. Dropping it ends the process, so
/// a route dropped without being closed leaves no tunnel behind.
pub(crate) struct Running {
    program: String,
    child: Option<Child>,
}

impl Running {
    /// Starts `command`; `recovery` tells the person what to do when it
    /// cannot start.
    pub(crate) fn start(command: &mut Command, recovery: &str) -> Result<Self, AxError> {
        let program = command.get_program().to_string_lossy().into_owned();
        let child = command.spawn().map_err(|err| {
            AxError::failure(
                AxCode::ToolUnavailable,
                "open a route",
                format!("{program}: {err}"),
            )
            .with_recovery(recovery)
        })?;
        Ok(Self {
            program,
            child: Some(child),
        })
    }

    pub(crate) fn program(&self) -> &str {
        &self.program
    }

    /// The child's standard output, once; `None` when it was not piped or
    /// was already taken.
    pub(crate) fn stdout(&mut self) -> Option<ChildStdout> {
        self.child.as_mut().and_then(|child| child.stdout.take())
    }

    /// How the process ended, or `None` while it runs.
    pub(crate) fn exited(&mut self) -> Result<Option<ExitStatus>, AxError> {
        match self.child.as_mut() {
            Some(child) => child
                .try_wait()
                .map_err(|err| lost(&self.program, "ask after", &err)),
            None => Ok(None),
        }
    }

    /// Ends the process and waits for it. A process that already ended
    /// is the state this asks for.
    pub(crate) fn stop(&mut self) -> Result<(), AxError> {
        let Some(mut child) = self.child.take() else {
            return Ok(());
        };
        if child
            .try_wait()
            .map_err(|err| lost(&self.program, "ask after", &err))?
            .is_some()
        {
            return Ok(());
        }
        child
            .kill()
            .map_err(|err| lost(&self.program, "end", &err))?;
        child
            .wait()
            .map(|_status| ())
            .map_err(|err| lost(&self.program, "wait for", &err))
    }
}

fn lost(program: &str, verb: &str, err: &std::io::Error) -> AxError {
    AxError::failure(
        AxCode::ToolUnavailable,
        "close a route",
        format!("cannot {verb} {program}: {err}"),
    )
    .with_recovery("end the process by hand; until it ends, the route may still be reachable")
}

impl Drop for Running {
    /// Nothing is left to tell here: `close` reports a process that would
    /// not end, and a route dropped unclosed has no caller to hear it.
    fn drop(&mut self) {
        match self.stop() {
            Ok(()) => {}
            Err(_unheard) => {}
        }
    }
}
