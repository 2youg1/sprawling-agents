// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The MCP servers a building's configuration names, connected for the
//! worker rather than started by it (accounting-SPEC.md 8-2).

use kernel::AxError;

/// Connects one server and hands back what it offers as tools.
///
/// The worker has already refused to start anything for a confidential
/// building by the time it asks, and it decides what a failure costs: a
/// server this returns an error for is left out of the run and named in
/// the diagnostics. An implementation decides only how the server is
/// reached and what it offers.
pub trait Connectors {
    /// Starts or reaches `server`, shakes hands, and lists its tools.
    ///
    /// `confidential` travels to `protocol::McpTool::new`, which is the
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
    ) -> Result<(Vec<protocol::McpTool>, protocol::Handshake), AxError>;
}
