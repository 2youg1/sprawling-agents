// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The MCP servers a building's configuration names, connected for the
//! worker rather than started by it (`crates/accounting/spec/Connectors.lean` §8-2).

use kernel::AxError;

/// Connects one server and hands back what it offers as tools.
///
/// The worker has already refused to start anything for a confidential
/// building by the time it asks, and it decides what a failure costs: a
/// server this returns an error for is left out of the run and named in
/// the diagnostics. An implementation decides only how the server is
/// reached, whether a connection an earlier run left is reused, and
/// what it offers.
pub trait Connectors {
    /// Starts or reaches `server`, shakes hands, and lists its tools, or
    /// hands out the tools of a connection an earlier run left running.
    ///
    /// `confidential` travels to `agent_protocols::McpTool::new`, which is the
    /// authority on whether such a tool may exist at all.
    ///
    /// # Errors
    /// Whatever starting the server, its handshake or its listing
    /// refuses.
    fn connect(
        &self,
        server: &kernel::McpServer,
        write_root: &std::path::Path,
        confidential: bool,
        resolve: &gateway::SecretResolver,
    ) -> Result<(Vec<agent_protocols::McpTool>, Reached), AxError>;

    /// Drops every resident connection whose declaration carries
    /// `reference`, so the next [`Connectors::connect`] redeems the
    /// reference again.
    ///
    /// A connection redeems its credentials once, when it opens, and
    /// sends that value for as long as it lives; the vault taking a new
    /// value under the same reference is what makes such a connection
    /// stale. The account id does not change, so the Session's account
    /// binding is untouched.
    fn invalidate(&self, reference: &kernel::SecretRef);
}

/// How one call to [`Connectors::connect`] reached its server.
pub enum Reached {
    /// Started, or opened, and shaken hands with during this call.
    Connected(agent_protocols::Handshake),
    /// Connected by an earlier call and still running.
    Resident,
}
