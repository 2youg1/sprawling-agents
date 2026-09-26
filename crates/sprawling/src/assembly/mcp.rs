// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Reaching the MCP servers a building's configuration names: the
//! production `accounting::Connectors`, and the one door that swaps it
//! for another (accounting-SPEC.md 8-2).

use accounting::Reached;
use kernel::{Address, AxError};

use super::RunWorker;

/// Every MCP server this worker has reached, kept connected between
/// runs, so a dispatch pays for a child and a handshake only when its
/// server was not already running (sprawling-SPEC.md 8-4).
///
/// Keyed by the whole declaration and the run root the child started
/// in: a changed field is another server, and a child cannot move to
/// another working directory once it runs.
///
/// Each key has its own lock, and the table's lock is held only to find
/// or add a key: a lane still shaking hands with one server holds up
/// the lanes asking for that server and nobody else.
#[derive(Default)]
pub(crate) struct Residents {
    keys: std::sync::Mutex<Vec<std::sync::Arc<Keyed>>>,
}

/// One server this worker was asked for, connected or not.
struct Keyed {
    server: kernel::McpServer,
    root: std::path::PathBuf,
    connected: std::sync::Mutex<Option<Resident>>,
}

/// One connected server and what it offered when it connected.
struct Resident {
    link: protocol::McpLink,
    /// Each tool under both its names, in the order the server listed
    /// them.
    listed: Vec<(kernel::ToolMeta, String)>,
}

impl accounting::Connectors for Residents {
    fn connect(
        &mut self,
        server: &kernel::McpServer,
        write_root: &std::path::Path,
        confidential: bool,
        resolve: &gateway::SecretResolver,
    ) -> Result<(Vec<protocol::McpTool>, Reached), AxError> {
        self.tools(server, write_root, confidential, resolve)
    }
}

impl Residents {
    /// The tools `server` offers, over the connection an earlier run left
    /// when its child still runs, and over a new one otherwise.
    ///
    /// A child that has ended is dropped from the table and started
    /// again here, which is how a server that died between two runs
    /// comes back.
    ///
    /// Takes `&self`, so lanes preparing dispatches at once can share
    /// one table; the port's `connect` reaches the same door.
    ///
    /// # Errors
    /// Propagates the transport's refusal to open, a failed handshake or
    /// listing, and `protocol::McpTool::new`'s refusal on a confidential
    /// building.
    pub(crate) fn tools(
        &self,
        server: &kernel::McpServer,
        write_root: &std::path::Path,
        confidential: bool,
        resolve: &gateway::SecretResolver,
    ) -> Result<(Vec<protocol::McpTool>, Reached), AxError> {
        let keyed = self.keyed(server, write_root)?;
        let mut connected = super::workbench::held(&keyed.connected, "reach an mcp server")?;
        if let Some(resident) = connected
            .take()
            .filter(|resident| !resident.link.has_ended())
        {
            let tools = resident.tools(confidential);
            *connected = Some(resident);
            return Ok((tools?, Reached::Resident));
        }
        let (resident, opened) = Resident::connect(server, write_root, resolve)?;
        let tools = resident.tools(confidential)?;
        *connected = Some(resident);
        Ok((tools, Reached::Connected(opened)))
    }

    /// The entry for `server` started in `write_root`, added when this
    /// is the first time it is asked for.
    fn keyed(
        &self,
        server: &kernel::McpServer,
        write_root: &std::path::Path,
    ) -> Result<std::sync::Arc<Keyed>, AxError> {
        let mut keys = super::workbench::held(&self.keys, "find an mcp server's entry")?;
        if let Some(found) = keys
            .iter()
            .find(|keyed| keyed.server == *server && keyed.root == write_root)
        {
            return Ok(std::sync::Arc::clone(found));
        }
        let added = std::sync::Arc::new(Keyed {
            server: server.clone(),
            root: write_root.to_path_buf(),
            connected: std::sync::Mutex::new(None),
        });
        keys.push(std::sync::Arc::clone(&added));
        Ok(added)
    }
}

impl RunWorker {
    /// The same worker, reaching every MCP server through `connectors`
    /// instead of starting the ones a building's configuration names.
    ///
    /// The door citysim and the dispatch tests drive a worker through:
    /// what a server offers is theirs to script, while refusing servers
    /// to a confidential building and leaving a failed one out stay the
    /// worker's.
    #[must_use]
    pub fn with_connectors(self, connectors: Box<dyn accounting::Connectors + Send>) -> RunWorker {
        RunWorker { connectors, ..self }
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
        Ok((Resident { link, listed }, opened))
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
                model: None,
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
                model: None,
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

    /// A stdio declaration under `label`, as a building's `[[mcp]]`
    /// table would carry it.
    fn stdio_server(label: &str, (command, args): (String, Vec<String>)) -> kernel::McpServer {
        kernel::McpServer {
            label: kernel::ServerLabel::parse(label).unwrap(),
            transport: kernel::McpTransport::Stdio {
                command,
                args,
                env: Vec::new(),
            },
        }
    }

    fn no_secrets() -> gateway::SecretResolver {
        Box::new(|_| {
            Err(AxError::failure(
                kernel::AxCode::ConfigInvalid,
                "resolve a secret",
                "this server declares none",
            )
            .with_recovery("give the test server no secret"))
        })
    }

    /// How many tools `server` offered, and whether this call connected
    /// it, in a shape a lane's thread can hand back.
    fn reach(residents: &Residents, server: &kernel::McpServer, root: &Path) -> (usize, bool) {
        let (tools, reached) = residents.tools(server, root, false, &no_secrets()).unwrap();
        (tools.len(), matches!(reached, Reached::Connected(_)))
    }

    /// Two lanes reaching two servers share one table: a handshake that
    /// has not been answered yet holds up the lanes asking for its own
    /// server and nobody else (sprawling-SPEC.md 8-4).
    #[test]
    fn a_server_still_shaking_hands_keeps_no_other_server_waiting() {
        let dir = tempfile::tempdir().unwrap();
        let (starts, gate) = (dir.path().join("starts.txt"), dir.path().join("open"));
        let slow = stdio_server("slow", protocol::gated(SERVER_ANSWER, &starts, &gate));
        let quick = stdio_server("quick", protocol::echoing(SERVER_ANSWER));
        let residents = Residents::default();

        std::thread::scope(|scope| {
            let waiting = scope.spawn(|| reach(&residents, &slow, dir.path()));
            let patience = std::time::Instant::now() + std::time::Duration::from_secs(60);
            while !starts.exists() && std::time::Instant::now() < patience {
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
            assert!(starts.exists(), "the gated server started");
            let other = scope.spawn(|| reach(&residents, &quick, dir.path()));
            let patience = std::time::Instant::now() + std::time::Duration::from_secs(30);
            while !other.is_finished() && std::time::Instant::now() < patience {
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
            let answered_while_the_first_waited = other.is_finished() && !waiting.is_finished();
            std::fs::write(&gate, "").unwrap();
            assert_eq!(waiting.join().unwrap(), (1, true));
            assert_eq!(other.join().unwrap(), (1, true));
            assert!(
                answered_while_the_first_waited,
                "the second server was reached while the first still shook hands"
            );
        });
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

    /// A dispatch whose server has not answered its handshake yet leaves
    /// the desk free: the command is answered on the accounting thread,
    /// and the lane that will drive the run is the one that waits for
    /// the server (sprawling-SPEC.md 8-93).
    #[test]
    fn a_dispatch_whose_server_still_shakes_hands_leaves_the_desk_free() {
        let dir = tempfile::tempdir().unwrap();
        init_city(dir.path()).unwrap();
        let (starts, gate) = (dir.path().join("starts.txt"), dir.path().join("open"));
        let (command, args) = protocol::gated(SERVER_ANSWER, &starts, &gate);
        write_server_table(dir.path(), "lab", &command, &args);
        let (base_url, _provider) = fake_openai(&["m-local"], vec![completion("done", None)]);
        let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
        // The gate opens on its own after a while, so a desk that waits
        // for the handshake still returns and the assertion below fails
        // rather than hangs.
        let (answered, heard) = std::sync::mpsc::channel::<()>();
        let opener = {
            let gate = gate.clone();
            std::thread::spawn(move || {
                drop(heard.recv_timeout(std::time::Duration::from_secs(10)));
                std::fs::write(&gate, "").unwrap();
            })
        };

        worker.serve_one(crate::assembly::Posted {
            command: channels::Command::Dispatch {
                addr: Address::parse("lab/room1").unwrap(),
                task: "ask the outside service".to_owned(),
                goal: "one answer is enough".to_owned(),
                mode: kernel::Mode::PlanGoal,
                idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"dispatch"),
                session: None,
                effort: None,
                model: None,
            },
            reply: channels::Reply::nowhere(),
        });
        let answered_before_the_handshake = !gate.exists();
        // An opener that already gave up has no ear left, and the gate
        // it opened is all this test needs from it.
        drop(answered.send(()));
        opener.join().unwrap();
        worker.land_the_rest().unwrap();

        assert!(
            answered_before_the_handshake,
            "the desk answered the dispatch while its server still shook hands"
        );
    }
}
