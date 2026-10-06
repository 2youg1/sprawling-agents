// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;

mod framing;
mod reconnect;

use framing::read_http_request;

use crate::worker::fixture::*;
use crate::worker::*;

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
    let report = crate::worker::fixture::init_city(dir.path()).unwrap();
    let (command, args) = agent_protocols::echoing(SERVER_ANSWER);
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
        .handle(wire::Command::Dispatch {
            addr: Address::parse("lab/room1").unwrap(),
            task: "ask the outside service".to_owned(),
            goal: "one answer is enough".to_owned(),
            policy: kernel::RunPolicy::of(kernel::Mode::Work),
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

/// Two different calls to one tool in one turn are two calls. A key
/// made of the turn's millisecond stamp and the tool's name would
/// return the second as a duplicate of the first, and the model would
/// read that as a fault in itself.
#[test]
fn the_same_tool_twice_with_different_arguments_runs_twice() {
    let dir = tempfile::tempdir().unwrap();
    let report = crate::worker::fixture::init_city(dir.path()).unwrap();
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
        .handle(wire::Command::Dispatch {
            addr: Address::parse("lab/room1").unwrap(),
            task: "read both files".to_owned(),
            goal: "both read".to_owned(),
            policy: kernel::RunPolicy::of(kernel::Mode::Work),
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
    crate::worker::fixture::init_city(dir.path()).unwrap();
    let (command, args) = agent_protocols::echoing(SERVER_ANSWER);
    write_server_table(dir.path(), "lab", &command, &args);
    let worker = RunWorker::new(
        dir.path(),
        runtime::diagnostics::Diagnostics::off(),
        crate::worker::fixture::hands(),
    )
    .unwrap();
    let config = city::load_config(dir.path(), &Address::parse("lab/room1").unwrap()).unwrap();

    let offered = worker
        .laying("")
        .unwrap()
        .mcp_tools(&config.mcp, dir.path(), false);
    assert_eq!(offered.len(), 1);
    assert_eq!(kernel::Tool::meta(&offered[0]).name.as_str(), "apps_ping");
    assert_eq!(offered[0].remote(), "ping");

    assert!(
        worker
            .laying("")
            .unwrap()
            .mcp_tools(&config.mcp, dir.path(), true)
            .is_empty(),
        "a confidential building holds no outbound tool, and starts nothing to hold one"
    );

    write_server_table(dir.path(), "lab", "sprawling-no-such-server", &[]);
    let config = city::load_config(dir.path(), &Address::parse("lab/room1").unwrap()).unwrap();
    assert!(
        worker
            .laying("")
            .unwrap()
            .mcp_tools(&config.mcp, dir.path(), false)
            .is_empty(),
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
/// server and nobody else (`crates/sprawling/Spec.lean` §8-4).
#[test]
#[allow(
    clippy::disallowed_methods,
    clippy::arithmetic_side_effects,
    reason = "test code: how long a server is waited for is read off the wall clock"
)]
fn a_server_still_shaking_hands_keeps_no_other_server_waiting() {
    let dir = tempfile::tempdir().unwrap();
    let (starts, gate) = (dir.path().join("starts.txt"), dir.path().join("open"));
    let slow = stdio_server(
        "slow",
        agent_protocols::gated(SERVER_ANSWER, &starts, &gate),
    );
    let quick = stdio_server("quick", agent_protocols::echoing(SERVER_ANSWER));
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
    crate::worker::fixture::init_city(dir.path()).unwrap();
    let starts = dir.path().join("starts.txt");
    let (command, args) = agent_protocols::counting_starts(SERVER_ANSWER, &starts);
    write_server_table(dir.path(), "lab", &command, &args);
    let worker = RunWorker::new(
        dir.path(),
        runtime::diagnostics::Diagnostics::off(),
        crate::worker::fixture::hands(),
    )
    .unwrap();
    let config = city::load_config(dir.path(), &Address::parse("lab/room1").unwrap()).unwrap();

    let first = worker
        .laying("")
        .unwrap()
        .mcp_tools(&config.mcp, dir.path(), false);
    drop(first);
    let second = worker
        .laying("")
        .unwrap()
        .mcp_tools(&config.mcp, dir.path(), false);

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
/// the server (`crates/sprawling/Spec.lean` §8-113).
#[test]
fn a_dispatch_whose_server_still_shakes_hands_leaves_the_desk_free() {
    let dir = tempfile::tempdir().unwrap();
    crate::worker::fixture::init_city(dir.path()).unwrap();
    let (starts, gate) = (dir.path().join("starts.txt"), dir.path().join("open"));
    let (command, args) = agent_protocols::gated(SERVER_ANSWER, &starts, &gate);
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
            if heard
                .recv_timeout(std::time::Duration::from_secs(10))
                .is_err()
            {
                std::fs::write(&gate, "").unwrap();
            }
        })
    };

    worker.serve_one(crate::worker::Posted {
        command: wire::Command::Dispatch {
            addr: Address::parse("lab/room1").unwrap(),
            task: "ask the outside service".to_owned(),
            goal: "one answer is enough".to_owned(),
            policy: kernel::RunPolicy::of(kernel::Mode::Work),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"dispatch"),
            session: None,
            effort: None,
            model: None,
        },
        reply: wire::Reply::nowhere(),
    });
    let answered_before_the_handshake = !gate.exists();
    std::fs::write(&gate, "").unwrap();
    drop(answered);
    opener.join().unwrap();
    worker.land_the_rest().unwrap();

    assert!(
        answered_before_the_handshake,
        "the desk answered the dispatch while its server still shook hands"
    );
}
