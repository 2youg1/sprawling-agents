// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;

fn addr(raw: &str) -> Address {
    Address::parse(raw).unwrap()
}

fn write(path: &Path, text: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(path, text).unwrap();
}

#[test]
fn a_city_with_no_configuration_at_all_runs_on_the_policy_defaults() {
    let dir = tempfile::tempdir().unwrap();
    let frozen = load(dir.path(), &addr("lab/room1")).unwrap();
    assert_eq!(
        frozen.effort, None,
        "an unstated effort stays unstated: the provider's default is not ours to name"
    );
}

#[test]
fn the_lower_layer_wins_and_the_upper_one_is_what_it_falls_back_to() {
    let dir = tempfile::tempdir().unwrap();
    let room = addr("lab/room1");
    write(
        &path(dir.path(), &room, Layer::City).unwrap(),
        "[model]\neffort = \"low\"\n",
    );
    assert_eq!(load(dir.path(), &room).unwrap().effort, Some(Effort::Low));

    write(
        &path(dir.path(), &room, Layer::Building).unwrap(),
        "[model]\neffort = \"high\"\n",
    );
    assert_eq!(load(dir.path(), &room).unwrap().effort, Some(Effort::High));

    write(
        &path(dir.path(), &room, Layer::Resident).unwrap(),
        "[model]\neffort = \"max\"\n",
    );
    assert_eq!(load(dir.path(), &room).unwrap().effort, Some(Effort::Max));

    // A room next door reads the building's value, not its neighbour's.
    assert_eq!(
        load(dir.path(), &addr("lab/room2")).unwrap().effort,
        Some(Effort::High)
    );
}

/// Every layer, not only the city's: the file that says how much a
/// run may think, which servers it reaches and what its sandbox
/// allows must not be writable by that run. The building layer used
/// to sit at `<building>/CONFIG.toml`, which is inside the default
/// write domain of every run in that building.
#[test]
fn no_layer_of_the_ladder_is_writable_by_what_it_governs() {
    let dir = tempfile::tempdir().unwrap();
    let room = addr("lab/room1");
    for layer in [Layer::City, Layer::Building, Layer::Resident] {
        let file = path(dir.path(), &room, layer).unwrap();
        assert!(file.ends_with(CONFIG_FILE), "{layer:?}: {}", file.display());
        let relative = file
            .strip_prefix(dir.path())
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        assert!(
            Address::parse(&relative).unwrap().is_reserved(),
            "{layer:?} is reachable from a write domain: {relative}"
        );
    }
    assert!(Address::parse(RESERVED_PREFIX).unwrap().is_reserved());
}

#[test]
fn an_address_that_is_its_own_building_has_two_layers_not_three() {
    let dir = tempfile::tempdir().unwrap();
    let lab = addr("lab");
    assert_eq!(
        path(dir.path(), &lab, Layer::Building).unwrap(),
        path(dir.path(), &lab, Layer::Resident).unwrap()
    );
    write(
        &path(dir.path(), &lab, Layer::Building).unwrap(),
        "[model]\neffort = \"medium\"\n",
    );
    assert_eq!(load(dir.path(), &lab).unwrap().effort, Some(Effort::Medium));
}

/// A session picks an effort once, and every run in that room reads
/// it afterwards through the ladder that was already there.
#[test]
fn a_chosen_effort_is_written_where_the_ladder_reads_it() {
    let dir = tempfile::tempdir().unwrap();
    let room = addr("lab/refactor");
    write_effort(dir.path(), &room, Effort::High).unwrap();
    assert_eq!(load(dir.path(), &room).unwrap().effort, Some(Effort::High));

    // Choosing again replaces the value and nothing else: a person
    // may have written other keys into the same file.
    let file = path(dir.path(), &room, Layer::Resident).unwrap();
    let text = std::fs::read_to_string(&file).unwrap();
    std::fs::write(&file, format!("{text}\n[sandbox]\nshell = true\n")).unwrap();
    write_effort(dir.path(), &room, Effort::Low).unwrap();
    let after = load(dir.path(), &room).unwrap();
    assert_eq!(after.effort, Some(Effort::Low));
    assert!(
        std::fs::read_to_string(&file).unwrap().contains("shell"),
        "the rest of the file did not survive the choice"
    );

    // And it lands where a run in that room cannot rewrite it.
    let relative = file
        .strip_prefix(dir.path())
        .unwrap()
        .to_string_lossy()
        .replace('\\', "/");
    assert!(
        Address::parse(&relative).unwrap().is_reserved(),
        "{relative}"
    );
}

#[test]
fn a_key_this_version_does_not_read_is_refused_rather_than_ignored() {
    let err = ConfigLayer::parse("[model]\nefort = \"high\"\n").unwrap_err();
    assert_eq!(err.code(), &AxCode::ConfigInvalid);
    assert!(err.recovery().contains("[model]"));

    let err = ConfigLayer::parse("[clock]\nstamp = \"minute\"\n").unwrap_err();
    assert_eq!(err.code(), &AxCode::ConfigInvalid);
}

#[test]
fn an_effort_level_no_provider_offers_is_refused_at_the_file() {
    let err = ConfigLayer::parse("[model]\neffort = \"maximum\"\n").unwrap_err();
    assert_eq!(err.code(), &AxCode::ConfigInvalid);
    assert_eq!(
        ConfigLayer::parse("[model]\neffort = \"xhigh\"\n")
            .unwrap()
            .effort(),
        Some(Effort::XHigh)
    );
}

#[test]
fn a_broken_file_names_itself_in_the_refusal() {
    let dir = tempfile::tempdir().unwrap();
    let room = addr("lab/room1");
    write(
        &path(dir.path(), &room, Layer::Building).unwrap(),
        "[model\neffort = \"high\"\n",
    );
    let err = load(dir.path(), &room).unwrap_err();
    assert_eq!(err.code(), &AxCode::ConfigInvalid);
    assert!(
        err.subject().contains("lab"),
        "three files can fail; the refusal says which one did: {}",
        err.subject()
    );
}

#[test]
fn a_building_states_which_servers_it_reaches_and_replaces_the_citys_table() {
    let dir = tempfile::tempdir().unwrap();
    let room = addr("lab/room1");
    write(
        &path(dir.path(), &room, Layer::City).unwrap(),
        "[[mcp]]\nlabel = \"apps\"\ncommand = \"mcp-apps\"\nargs = [\"--stdio\"]\n\n\
         [[mcp]]\nlabel = \"mail\"\ncommand = \"mcp-mail\"\n",
    );
    let frozen = load(dir.path(), &room).unwrap();
    assert_eq!(frozen.mcp.len(), 2);
    assert_eq!(frozen.mcp[0].label.as_str(), "apps");
    assert_eq!(
        frozen.mcp[0].transport,
        McpTransport::Stdio {
            command: "mcp-apps".to_owned(),
            args: vec!["--stdio".to_owned()],
            env: Vec::new(),
        }
    );

    write(
        &path(dir.path(), &room, Layer::Building).unwrap(),
        "[[mcp]]\nlabel = \"mail\"\ncommand = \"mcp-mail\"\n",
    );
    let frozen = load(dir.path(), &room).unwrap();
    assert_eq!(
        frozen.mcp.len(),
        1,
        "a layer that speaks about servers speaks about all of them"
    );
    assert_eq!(frozen.mcp[0].label.as_str(), "mail");
}

#[test]
fn a_server_table_that_could_not_route_a_call_is_refused_at_the_file() {
    let dir = tempfile::tempdir().unwrap();
    let room = addr("lab/room1");
    write(
        &path(dir.path(), &room, Layer::Building).unwrap(),
        "[[mcp]]\nlabel = \"Apps\"\ncommand = \"mcp-apps\"\n",
    );
    let err = load(dir.path(), &room).unwrap_err();
    assert_eq!(err.code(), &AxCode::ConfigInvalid);
    assert!(
        err.subject().contains("lab"),
        "the refusal says which file: {}",
        err.subject()
    );

    let err = ConfigLayer::parse(
        "[[mcp]]\nlabel = \"apps\"\ncommand = \"one\"\n\n\
         [[mcp]]\nlabel = \"apps\"\ncommand = \"two\"\n",
    )
    .unwrap_err();
    assert!(
        err.subject().contains("named the same"),
        "{}",
        err.subject()
    );

    let err = ConfigLayer::parse("[[mcp]]\nlabel = \"apps\"\ncommand = \"\"\n").unwrap_err();
    assert!(err.subject().contains("command"), "{}", err.subject());

    let err =
        ConfigLayer::parse("[[mcp]]\nlabel = \"apps\"\nprogram = \"mcp-apps\"\n").unwrap_err();
    assert_eq!(err.code(), &AxCode::ConfigInvalid);
    assert!(err.recovery().contains("[[mcp]]"));
}

#[test]
fn the_reserved_subtree_has_no_configuration_layers() {
    let dir = tempfile::tempdir().unwrap();
    let err = load(dir.path(), &addr(".sprawling/ledger")).unwrap_err();
    assert_eq!(err.code(), &AxCode::InvalidArgs);
}

/// The names a building declares its runs may inherit are read here, and
/// a name shaped like a credential is refused here too — at the moment
/// the configuration enters the city, not at the moment a child process
/// would have been given it.
#[test]
fn declared_environment_names_are_read_and_credential_shaped_ones_refused() {
    let layer = ConfigLayer::parse(
        "[sandbox]\nshell = false\nenv_passthrough = [\"ProgramFiles(x86)\", \"VSINSTALLDIR\"]\n",
    )
    .unwrap();
    let names: Vec<&str> = layer
        .sandbox()
        .unwrap()
        .env_passthrough
        .iter()
        .map(kernel::EnvVarName::as_str)
        .collect();
    assert_eq!(names, vec!["ProgramFiles(x86)", "VSINSTALLDIR"]);

    let err = ConfigLayer::parse("[sandbox]\nenv_passthrough = [\"AWS_SECRET_ACCESS_KEY\"]\n")
        .unwrap_err();
    assert_eq!(err.code(), &AxCode::ConfigInvalid);
    assert!(
        err.subject().contains("AWS_SECRET_ACCESS_KEY"),
        "the refusal names which one: {}",
        err.subject()
    );

    // A layer that says nothing about the sandbox declares no names.
    assert!(ConfigLayer::parse("").unwrap().sandbox().is_none());
}

/// A building that declares nothing inherits nothing beyond the floor,
/// which is what makes the declaration meaningful.
#[test]
fn a_building_that_declares_no_names_resolves_to_an_empty_list() {
    let dir = tempfile::tempdir().unwrap();
    let room = addr("lab/room1");
    let frozen = load(dir.path(), &room).unwrap();
    assert!(frozen.sandbox.env_passthrough.is_empty());
}

/// The three shapes a `[[mcp]]` row may take, read back whole. The
/// environment table and the header table are what `claude mcp add -e`
/// and `--header` write, and `transport` is the only thing that tells a
/// stream apart from a posted message.
#[test]
fn a_server_row_carries_its_environment_its_headers_and_which_way_a_url_answers() {
    let layer = ConfigLayer::parse(
        "[[mcp]]\nlabel = \"apps\"\ncommand = \"mcp-apps\"\nargs = [\"--stdio\"]\n\
         env = { API_KEY = \"secret:mcp/apps\", REGION = \"eu\" }\n\n\
         [[mcp]]\nlabel = \"hosted\"\nurl = \"https://example.test/mcp\"\n\
         headers = { Authorization = \"secret:mcp/hosted\", X-Account = \"acme\" }\n\n\
         [[mcp]]\nlabel = \"streamed\"\nurl = \"https://example.test/sse\"\ntransport = \"sse\"\n",
    )
    .unwrap();
    let servers = layer.mcp().unwrap();
    assert_eq!(
        servers.iter().map(|s| &s.transport).collect::<Vec<_>>(),
        vec![
            &McpTransport::Stdio {
                command: "mcp-apps".to_owned(),
                args: vec!["--stdio".to_owned()],
                env: vec![
                    ("API_KEY".to_owned(), "secret:mcp/apps".to_owned()),
                    ("REGION".to_owned(), "eu".to_owned()),
                ],
            },
            &McpTransport::Http {
                url: "https://example.test/mcp".to_owned(),
                headers: vec![
                    ("Authorization".to_owned(), "secret:mcp/hosted".to_owned()),
                    ("X-Account".to_owned(), "acme".to_owned()),
                ],
            },
            &McpTransport::Sse {
                url: "https://example.test/sse".to_owned(),
                headers: Vec::new(),
            },
        ]
    );
}

/// A command answers on its own pipes, so naming a stream beside one is
/// a row stating two transports rather than a row with a spare key.
#[test]
fn a_command_row_that_also_names_a_transport_is_refused_where_it_is_written() {
    let err = ConfigLayer::parse(
        "[[mcp]]\nlabel = \"apps\"\ncommand = \"mcp-apps\"\ntransport = \"sse\"\n",
    )
    .unwrap_err();
    assert_eq!(err.code(), &AxCode::ConfigInvalid);
    assert!(err.subject().contains("apps"), "{}", err.subject());
}

/// What the writer emits is what the reader reads: the file's grammar
/// has one authority, and a round trip is how that stays true.
#[test]
fn every_transport_the_writer_emits_is_read_back_as_the_same_transport() {
    let dir = tempfile::tempdir().unwrap();
    let room = addr("lab/room1");
    let servers = vec![
        McpServer {
            label: kernel::ServerLabel::parse("apps").unwrap(),
            transport: McpTransport::Stdio {
                command: "mcp-apps".to_owned(),
                args: vec!["--stdio".to_owned()],
                env: vec![("API_KEY".to_owned(), "secret:mcp/apps".to_owned())],
            },
        },
        McpServer {
            label: kernel::ServerLabel::parse("hosted").unwrap(),
            transport: McpTransport::Http {
                url: "https://example.test/mcp".to_owned(),
                headers: vec![("Authorization".to_owned(), "Bearer x".to_owned())],
            },
        },
        McpServer {
            label: kernel::ServerLabel::parse("streamed").unwrap(),
            transport: McpTransport::Sse {
                url: "https://example.test/sse".to_owned(),
                headers: Vec::new(),
            },
        },
    ];
    write_mcp(dir.path(), &room, Layer::City, &servers).unwrap();
    assert_eq!(load(dir.path(), &room).unwrap().mcp, servers);
}
