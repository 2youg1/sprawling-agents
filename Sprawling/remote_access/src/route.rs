// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::net::SocketAddr;
use std::process::{Child, ChildStdout, Command, ExitStatus};

use kernel::{AxCode, AxError};

pub mod cloudflare;
pub mod command;
pub mod scripted;
#[cfg(test)]
mod tests;

pub trait Route {
    fn open(&mut self, local: SocketAddr) -> Result<Opened, AxError>;

    fn close(&mut self) -> Result<(), AxError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Opened {
    pub url: PublicUrl,
    pub permanence: Permanence,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicUrl(String);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Permanence {
    Fixed,
    PerStart,
}

impl PublicUrl {
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

pub(crate) struct Running {
    program: String,
    child: Option<Child>,
}

impl Running {
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

    pub(crate) fn stdout(&mut self) -> Option<ChildStdout> {
        self.child.as_mut().and_then(|child| child.stdout.take())
    }

    pub(crate) fn exited(&mut self) -> Result<Option<ExitStatus>, AxError> {
        match self.child.as_mut() {
            Some(child) => child
                .try_wait()
                .map_err(|err| lost(&self.program, "ask after", &err)),
            None => Ok(None),
        }
    }

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
    fn drop(&mut self) {
        match self.stop() {
            Ok(()) => {}
            Err(_unheard) => {}
        }
    }
}
