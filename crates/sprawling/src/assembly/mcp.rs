// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Reaching the MCP servers a building's configuration names.

use kernel::{Address, AxError};

/// Every MCP server this worker has reached, kept connected between
/// runs, so a dispatch pays for a child and a handshake only when its
/// server was not already running (sprawling-SPEC.md 8-4).
///
/// Keyed by the whole declaration and the run root the child started
/// in: a changed field is another server, and a child cannot move to
/// another working directory once it runs.
#[derive(Default)]
pub(crate) struct Residents {
    held: Vec<Resident>,
}

/// One connected server and what it offered when it connected.
struct Resident {
    server: kernel::McpServer,
    root: std::path::PathBuf,
    link: protocol::McpLink,
    /// Each tool under both its names, in the order the server listed
    /// them.
    listed: Vec<(kernel::ToolMeta, String)>,
}

/// How this dispatch reached a server.
pub(crate) enum Reached {
    /// Started, or opened, and shaken hands with during this dispatch.
    Connected(protocol::Handshake),
    /// Connected by an earlier run and still running.
    Resident,
}

impl Residents {
    /// The tools `server` offers, over the connection an earlier run left
    /// when its child still runs, and over a new one otherwise.
    ///
    /// A child that has ended is dropped from the table and started
    /// again here, which is how a server that died between two runs
    /// comes back.
    ///
    /// # Errors
    /// Propagates the transport's refusal to open, a failed handshake or
    /// listing, and `protocol::McpTool::new`'s refusal on a confidential
    /// building.
    pub(crate) fn tools(
        &mut self,
        server: &kernel::McpServer,
        write_root: &std::path::Path,
        confidential: bool,
        resolve: &gateway::SecretResolver,
    ) -> Result<(Vec<protocol::McpTool>, Reached), AxError> {
        self.held
            .retain(|held| !(held.serves(server, write_root) && held.link.has_ended()));
        if let Some(held) = self
            .held
            .iter()
            .find(|held| held.serves(server, write_root))
        {
            return Ok((held.tools(confidential)?, Reached::Resident));
        }
        let (resident, opened) = Resident::connect(server, write_root, resolve)?;
        let tools = resident.tools(confidential)?;
        self.held.push(resident);
        Ok((tools, Reached::Connected(opened)))
    }
}

impl Resident {
    /// Starts one server and asks what it offers.
    ///
    /// The connection opens with the lifecycle the specification defines -
    /// `initialize`, then `notifications/initialized` - and only then asks
    /// what it offers. What the handshake learns is written to the
    /// diagnostics rather than branched on: negotiating a version needs a
    /// second version this build can speak before it can decide anything.
    fn connect(
        server: &kernel::McpServer,
        write_root: &std::path::Path,
        resolve: &gateway::SecretResolver,
    ) -> Result<(Resident, protocol::Handshake), AxError> {
        use protocol::Outbound as _;

        // The run's own root, which exists whether or not this building
        // lends its runs a worktree.
        let mut link = protocol::McpLink::open(&server.transport, write_root, resolve)?;
        let mut rpc = protocol::Rpc::new();
        let opened = protocol::handshake(&mut link, &mut rpc, protocol::EXTERNAL_CALL_PATIENCE)?;
        let listing = link.call(&rpc.list_tools(), protocol::EXTERNAL_CALL_PATIENCE)?;
        let listed = protocol::tools_from(&server.label, &protocol::Rpc::read(&listing)?)?
            .into_iter()
            .map(|entry| (entry.meta, entry.remote))
            .collect();
        Ok((
            Resident {
                server: server.clone(),
                root: write_root.to_path_buf(),
                link,
                listed,
            },
            opened,
        ))
    }

    fn serves(&self, server: &kernel::McpServer, write_root: &std::path::Path) -> bool {
        self.server == *server && self.root == write_root
    }

    /// One handle per tool on the one connection: two connections would
    /// be two answers to what the same label offers.
    fn tools(&self, confidential: bool) -> Result<Vec<protocol::McpTool>, AxError> {
        self.listed
            .iter()
            .map(|(meta, remote)| {
                protocol::McpTool::new(
                    meta.clone(),
                    remote.clone(),
                    Box::new(self.link.clone()),
                    confidential,
                )
            })
            .collect()
    }
}

/// Turns the configured mount list into paths under the run's write
/// root. Read-only by construction: the sandbox job carries them as
/// readable, and what may be written is the write domain's answer.
pub(super) fn mounts_under(
    write_root: &std::path::Path,
    mounts: &[Address],
) -> Vec<runtime::Mount> {
    mounts
        .iter()
        .map(|addr| runtime::Mount {
            host: write_root.join(addr.as_str()),
            guest: format!("/{}", addr.as_str()),
            writable: false,
        })
        .collect()
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests {
    use super::*;
    use crate::assembly::fixture::*;
    use crate::assembly::*;

    /// Writes a `[[mcp]]` table naming one server at the building layer.
    fn write_server_table(city_root: &Path, addr: &str, command: &str, args: &[String]) {
        let addr = Address::parse(addr).unwrap();
        let path = city::config_path(city_root, &addr, city::Layer::Building).unwrap();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(
            &path,
            format!(
                "[[mcp]]\nlabel = \"apps\"\ncommand = {}\nargs = {}\n",
                serde_json::to_string(command).unwrap(),
                serde_json::to_string(args).unwrap(),
            ),
        )
        .unwrap();
    }

    /// One line that serves as every answer this fake server gives: the
    /// negotiated version and who it is for the handshake, a listing
    /// with one tool, and content for when that tool is called.
    const SERVER_ANSWER: &str = "{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{\"protocolVersion\":\"2025-06-18\",\"capabilities\":{},\"serverInfo\":{\"name\":\"apps\",\"version\":\"1\"},\"tools\":[{\"name\":\"ping\",\"description\":\"answer with pong\",\"inputSchema\":{\"type\":\"object\"}}],\"content\":[{\"type\":\"text\",\"text\":\"pong\"}]}}";

    #[test]
    fn a_configured_server_becomes_a_tool_the_model_is_told_about_and_can_call() {
        let dir = tempfile::tempdir().unwrap();
        let report = init_city(dir.path()).unwrap();
        let (command, args) = protocol::echoing(SERVER_ANSWER);
        write_server_table(dir.path(), "lab", &command, &args);

        let (base_url, provider) = fake_openai(
            &["m-local"],
            vec![
                tool_completion("asking outside", "tu_1", "apps_ping", serde_json::json!({})),
                completion("done", None),
            ],
        );
        let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
        worker
            .handle(channels::Command::Dispatch {
                addr: Address::parse("lab/room1").unwrap(),
                task: "ask the outside service".to_owned(),
                goal: "one answer is enough".to_owned(),
                mode: kernel::Mode::PlanGoal,
                idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"dispatch"),
                session: None,
                effort: None,
            })
            .unwrap();

        let asked = provider.bodies().join("\n");
        assert!(
            asked.contains("apps_ping"),
            "the tool table the model is given carries the external tool"
        );
        let verified = runtime::replay::verify_ledger_dir(&report.ledger_dir).unwrap();
        let history: String = verified
            .raw_lines()
            .iter()
            .map(|line| String::from_utf8_lossy(line).into_owned())
            .collect::<Vec<String>>()
            .join("\n");
        assert!(
            history.contains("apps_ping"),
            "an external call is history like any other call"
        );
        assert!(
            history.contains("pong"),
            "and what the server answered came back through the tool seam"
        );
    }

    /// Two different calls to one tool in one turn are two calls. The key
    /// used to be the turn's millisecond stamp plus the tool's name, so
    /// the second came back as a duplicate of the first - and the model
    /// read that as a fault in itself.
    #[test]
    fn the_same_tool_twice_with_different_arguments_runs_twice() {
        let dir = tempfile::tempdir().unwrap();
        let report = init_city(dir.path()).unwrap();
        city::create_building(
            dir.path(),
            &Address::parse("lab").unwrap(),
            city::BuildingTemplate::Minimal,
        )
        .unwrap();
        std::fs::write(dir.path().join("lab").join("one.md"), "first\n").unwrap();
        std::fs::write(dir.path().join("lab").join("two.md"), "second\n").unwrap();

        let (base_url, _provider) = fake_openai(
            &["m-local"],
            vec![
                tool_completion(
                    "one",
                    "tu_1",
                    "read",
                    serde_json::json!({ "path": "lab/one.md" }),
                ),
                tool_completion(
                    "two",
                    "tu_2",
                    "read",
                    serde_json::json!({ "path": "lab/two.md" }),
                ),
                completion("done", None),
            ],
        );
        let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
        worker
            .handle(channels::Command::Dispatch {
                addr: Address::parse("lab/room1").unwrap(),
                task: "read both files".to_owned(),
                goal: "both read".to_owned(),
                mode: kernel::Mode::PlanGoal,
                idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"dispatch"),
                session: None,
                effort: None,
            })
            .unwrap();

        let verified = runtime::replay::verify_ledger_dir(&report.ledger_dir).unwrap();
        let history: String = verified
            .raw_lines()
            .iter()
            .map(|line| String::from_utf8_lossy(line).into_owned())
            .collect::<Vec<String>>()
            .join("\n");
        assert!(history.contains("first"), "the first read answered");
        assert!(
            history.contains("second"),
            "and so did the second: {history}"
        );
        assert!(
            !history.contains("already made"),
            "two files are two actions"
        );
    }

    #[test]
    fn a_confidential_building_starts_no_server_and_a_dead_one_is_simply_absent() {
        let dir = tempfile::tempdir().unwrap();
        init_city(dir.path()).unwrap();
        let (command, args) = protocol::echoing(SERVER_ANSWER);
        write_server_table(dir.path(), "lab", &command, &args);
        let mut worker = RunWorker::new(
            dir.path(),
            gateway::Custodian::in_memory(),
            runtime::diagnostics::Diagnostics::off(),
        )
        .unwrap();
        let config = city::load_config(dir.path(), &Address::parse("lab/room1").unwrap()).unwrap();

        let offered = worker.mcp_tools(&config, dir.path(), false);
        assert_eq!(offered.len(), 1);
        assert_eq!(kernel::Tool::meta(&offered[0]).name.as_str(), "apps_ping");
        assert_eq!(offered[0].remote(), "ping");

        assert!(
            worker.mcp_tools(&config, dir.path(), true).is_empty(),
            "a confidential building holds no outbound tool, and starts nothing to hold one"
        );

        write_server_table(dir.path(), "lab", "sprawling-no-such-server", &[]);
        let config = city::load_config(dir.path(), &Address::parse("lab/room1").unwrap()).unwrap();
        assert!(
            worker.mcp_tools(&config, dir.path(), false).is_empty(),
            "a service that is down today does not stop the building from working today"
        );
    }

    #[test]
    fn a_second_dispatch_reaches_the_server_the_first_one_started() {
        let dir = tempfile::tempdir().unwrap();
        init_city(dir.path()).unwrap();
        let starts = dir.path().join("starts.txt");
        let (command, args) = protocol::counting_starts(SERVER_ANSWER, &starts);
        write_server_table(dir.path(), "lab", &command, &args);
        let mut worker = RunWorker::new(
            dir.path(),
            gateway::Custodian::in_memory(),
            runtime::diagnostics::Diagnostics::off(),
        )
        .unwrap();
        let config = city::load_config(dir.path(), &Address::parse("lab/room1").unwrap()).unwrap();

        let first = worker.mcp_tools(&config, dir.path(), false);
        drop(first);
        let second = worker.mcp_tools(&config, dir.path(), false);

        assert_eq!(
            second.len(),
            1,
            "the resident connection still offers its tool"
        );
        assert_eq!(
            std::fs::read_to_string(&starts).unwrap().lines().count(),
            1,
            "two dispatches to one building started its server once"
        );
    }
}
