// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::sync::{Arc, Mutex};

use super::offering::{apps_reference, apps_rows, assert_only_namespace_count, catalog_drifts};
use super::*;

#[test]
fn a_removed_tool_is_recorded_without_removing_its_pin() {
    let initial = listing(&[("ping", "answer with pong"), ("pong", "unchanged pong")]);
    let (dir, mut worker, listing, offered) = revision_city(&initial);
    dispatch(&mut worker, "s1");
    let pinned = apps_reference(dir.path()).tools.unwrap();
    *listing.lock().unwrap() = Some(LISTING.to_owned());

    offered.lock().unwrap().clear();
    dispatch(&mut worker, "s2");
    assert_only_namespace_count(&offered, 1);
    assert_eq!(apps_reference(dir.path()).tools.unwrap(), pinned);
    let drifts = catalog_drifts(dir.path());
    assert_eq!(drifts.len(), 1);
    assert!(matches!(
        &drifts[0],
        kernel::event::record::CatalogDrift {
            subject: kernel::event::record::CatalogSubject::Tool { name, .. },
            pinned: Some(_),
            found: None,
        } if name == "apps_pong"
    ));
    let rows = apps_rows(dir.path());
    let removed = rows.iter().find(|row| row.name == "apps_pong").unwrap();
    assert_eq!(removed.change, Some(wire::CatalogChange::Removed));
    assert_eq!(removed.current, None);
    assert_eq!(
        removed.pinned.as_ref().unwrap().description,
        "unchanged pong"
    );
}

#[test]
fn a_definition_only_change_records_server_drift_without_withholding_unchanged_tools() {
    let (dir, mut worker, _listing, offered) = revision_city(LISTING);
    dispatch(&mut worker, "s1");
    let building = Address::parse(LAB).unwrap();
    let mut server = city::load_config(dir.path(), &Address::parse("lab/room1").unwrap())
        .unwrap()
        .mcp
        .into_iter()
        .find(|server| server.label.as_str() == NAMESPACE)
        .unwrap();
    server.transport = kernel::McpTransport::Stdio {
        command: "another-absent-server".to_owned(),
        args: Vec::new(),
        env: Vec::new(),
    };
    city::write_mcp(dir.path(), &building, city::Layer::Building, &[server]).unwrap();

    offered.lock().unwrap().clear();
    dispatch(&mut worker, "s2");
    assert_only_namespace_count(&offered, 1);
    let drifts = catalog_drifts(dir.path());
    assert_eq!(drifts.len(), 1);
    assert!(matches!(
        &drifts[0],
        kernel::event::record::CatalogDrift {
            subject: kernel::event::record::CatalogSubject::Server { .. },
            pinned: Some(_),
            found: Some(_),
        }
    ));
}

#[test]
fn a_legacy_room_keeps_the_first_listing_it_adopted() {
    let (dir, mut worker, listing, offered) = revision_city(LISTING);
    dispatch(&mut worker, "s1");
    *listing.lock().unwrap() = Some(self::listing(&[("ping", "changed after admission")]));

    offered.lock().unwrap().clear();
    worker
        .handle(wire::Command::Dispatch {
            addr: Address::parse("lab/s1").unwrap(),
            task: "Continue the same room.".to_owned(),
            goal: "keep its frozen catalog".to_owned(),
            policy: kernel::RunPolicy::of(kernel::Mode::Work),
            idem: idem(b"continue-s1"),
            session: None,
            effort: None,
            model: None,
        })
        .unwrap();
    assert!(
        !offered
            .lock()
            .unwrap()
            .iter()
            .any(|name| name.starts_with(NAMESPACE)),
        "the changed tool entered the already open room"
    );
    assert_eq!(catalog_drifts(dir.path()).len(), 1);
}

#[test]
fn status_and_the_catalog_answer_read_one_frozen_count_table() {
    let seen = Arc::new(Mutex::new(Seen::default()));
    let (dir, mut worker) = lab_city(
        Box::new(Callers(Calling {
            seen: Arc::clone(&seen),
            tool: "status",
            args: serde_json::json!({}),
        })),
        Box::new(Scripted(LISTING.to_owned())),
    );
    let dispatched = dispatch(&mut worker, "s1");
    let wire::Answer::Catalog(catalog) = accounting::views::ask(
        dir.path(),
        &wire::Query::Catalog {
            building: Address::parse(LAB).unwrap(),
        },
    )
    .unwrap() else {
        panic!("the catalog question did not answer Catalog")
    };
    let expected = format!(
        "catalog: {} always, {} dormant, {} skills, {} core bytes, {} index bytes, {} reading bytes",
        catalog.always,
        catalog.dormant,
        catalog.skills,
        catalog.core_bytes,
        catalog.index_bytes,
        catalog.reading_bytes
    );
    let read = seen.lock().unwrap();
    assert!(
        read.read
            .first()
            .is_some_and(|text| text.contains(&expected)),
        "status did not carry {expected:?}: {:?}; dispatch {dispatched}",
        read.read
    );
}

#[test]
fn a_failed_listing_is_not_a_removal() {
    let (dir, mut worker, listing, _offered) = revision_city(LISTING);
    dispatch(&mut worker, "s1");
    *listing.lock().unwrap() = None;
    dispatch(&mut worker, "s2");
    assert!(catalog_drifts(dir.path()).is_empty());
}

const CALL: &str = "{\"jsonrpc\":\"2.0\",\"id\":0,\"method\":\"tools/call\",\"params\":{\"name\":\"ping\",\"arguments\":{}}}";

/// A server whose one tool answers with a long converted document.
struct Answering(String);

impl accounting::Connectors for Answering {
    fn connect(
        &self,
        server: &kernel::McpServer,
        _write_root: &std::path::Path,
        confidential: bool,
        _resolve: &gateway::SecretResolver,
    ) -> Result<(Vec<agent_protocols::McpTool>, accounting::Reached), AxError> {
        let answer = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 0,
            "result": { "content": [{ "type": "text", "text": self.0 }], "isError": false },
        })
        .to_string();
        let tools =
            agent_protocols::tools_from(&server.label, &agent_protocols::Rpc::read(LISTING)?)?
                .into_iter()
                .map(|entry| {
                    let mut outbound = agent_protocols::ScriptedOutbound::new();
                    outbound.answer(CALL, &answer)?;
                    agent_protocols::McpTool::new(
                        entry.meta,
                        entry.remote,
                        Box::new(outbound),
                        confidential,
                    )
                })
                .collect::<Result<Vec<_>, AxError>>()?;
        let opened = agent_protocols::Handshake {
            protocol_version: agent_protocols::PROTOCOL_VERSION.to_owned(),
            server: "scripted".to_owned(),
        };
        Ok((tools, accounting::Reached::Connected(opened)))
    }

    /// Keeps no connection, so there is nothing a new key makes stale.
    fn invalidate(&self, _reference: &kernel::SecretRef) {}
}

/// What a calling model saw: the dormant index of its first request,
/// and every tool result it read back.
#[derive(Default)]
pub(crate) struct Seen {
    pub(crate) index: Vec<String>,
    pub(crate) read: Vec<String>,
}

/// Makes one tool call, then records what it read back.
#[derive(Clone)]
pub(crate) struct Calling {
    pub(crate) seen: Arc<Mutex<Seen>>,
    pub(crate) tool: &'static str,
    pub(crate) args: serde_json::Value,
}

impl Model for Calling {
    fn call(&mut self, req: &ModelRequest) -> Result<ModelReturn, AxError> {
        let results: Vec<String> = req
            .chat
            .messages
            .iter()
            .flat_map(|message| &message.content)
            .filter_map(|block| match block {
                ContentBlock::ToolResult { content, .. } => Some(content.clone()),
                ContentBlock::Text { .. }
                | ContentBlock::Thinking { .. }
                | ContentBlock::RedactedThinking { .. }
                | ContentBlock::ToolUse { .. }
                | ContentBlock::Image(_) => None,
            })
            .collect();
        if !results.is_empty() {
            self.seen.lock().unwrap().read.extend(results);
            return Ok(ModelReturn::bare(
                kernel::model::message_payload(&[ContentBlock::Text {
                    text: "done".to_owned(),
                }])?,
                Vec::new(),
            ));
        }
        self.seen.lock().unwrap().index = dormant_names(req);
        let name = kernel::ToolName::parse(self.tool)?;
        let input = kernel::Payload::of(&self.args)?;
        Ok(ModelReturn::bare(
            kernel::model::message_payload(&[ContentBlock::ToolUse {
                id: "call-1".to_owned(),
                name: name.clone(),
                input: input.clone(),
            }])?,
            vec![kernel::ToolCall {
                id: "call-1".to_owned(),
                name,
                args: input,
            }],
        ))
    }
}

pub(crate) struct Callers(pub(crate) Calling);

impl accounting::ModelFactory for Callers {
    fn build(
        &self,
        _chosen: &gateway::Chosen<'_>,
        _redemption: gateway::Redemption,
    ) -> Result<Box<dyn Model + Send>, AxError> {
        Ok(Box::new(self.0.clone()))
    }
}

/// A converted document longer than the window reaches the model as
/// the pipeline's window over it, not whole (`Sprawling/runtime/Spec.lean` §8-27-10).
#[test]
fn a_long_connector_answer_reaches_the_model_packaged() {
    let document = "The quarter closed with every account reconciled. ".repeat(1_000);
    let seen = Arc::new(Mutex::new(Seen::default()));
    let dispatched = dispatch_one_run(
        Box::new(Callers(Calling {
            seen: Arc::clone(&seen),
            tool: OFFERED,
            args: serde_json::json!({}),
        })),
        Box::new(Answering(document.clone())),
    );

    let read = &seen.lock().unwrap().read;
    assert_eq!(
        read.len(),
        1,
        "one result read back; the dispatch answered {dispatched}"
    );
    assert!(
        read[0].len() < document.len(),
        "the model read {} bytes of a {}-byte document",
        read[0].len(),
        document.len()
    );
}
