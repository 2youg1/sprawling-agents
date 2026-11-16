// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use serde_json::Value;

use super::entry::{AgentEntry, AgentSource};
use super::paste::server_entry;
use super::roster::{OFFICIAL, Roster, SetUpDir};

pub const CLIENT_CONFIGS: [SetUpDir; 2] = [
    SetUpDir {
        variable: None,
        under_home: &[".jetbrains", "acp.json"],
        source: "https://www.jetbrains.com/help/ai-assistant/acp.html",
    },
    SetUpDir {
        variable: None,
        under_home: &[".config", "zed", "settings.json"],
        source: "https://zed.dev/docs/ai/external-agents",
    },
];

#[must_use]
pub fn configured(text: &str) -> Vec<AgentEntry> {
    let Ok(config) = serde_json::from_str::<Value>(text) else {
        return Vec::new();
    };
    config
        .get("agent_servers")
        .and_then(Value::as_object)
        .map(|servers| {
            servers
                .iter()
                .filter_map(|(name, server)| {
                    match server_entry(name, server, AgentSource::Detected) {
                        Ok(mut entry) => {
                            entry.launch.env.clear();
                            Some(entry)
                        }

                        Err(_not_an_agent) => None,
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}

#[must_use]
pub fn detected(
    roster: &Roster,
    set_up: impl Fn(&SetUpDir) -> bool,
    configs: &[String],
) -> Vec<AgentEntry> {
    let builtins = OFFICIAL
        .iter()
        .filter(|official| official.set_up.iter().any(&set_up))
        .filter_map(|official| roster.builtin(official))
        .map(|entry| AgentEntry {
            source: AgentSource::Detected,
            ..entry
        });
    let mut found: Vec<AgentEntry> = Vec::new();
    for entry in configs
        .iter()
        .flat_map(|text| configured(text))
        .chain(builtins)
    {
        if !found.iter().any(|held| held.id == entry.id) {
            found.push(entry);
        }
    }
    found
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing, reason = "test code")]
mod tests {
    use super::*;

    #[test]
    fn evidence_offers_other_clients_agents_without_env_and_builtins_by_directory() {
        let roster = Roster::new(
            Vec::new(),
            crate::harness::catalog::Catalog::bundled().unwrap(),
        );
        let jetbrains = r#"{"agent_servers":{"My Agent":{"command":"/opt/agent","args":["acp"],"env":{"API_KEY":"k"}}}}"#;
        let found = detected(
            &roster,
            |dir| dir.variable == Some("CODEX_HOME"),
            &[jetbrains.to_owned(), "// zed comment\n{}".to_owned()],
        );
        assert_eq!(
            found
                .iter()
                .map(|entry| (entry.id.as_str(), entry.source, entry.launch.env.len()))
                .collect::<Vec<_>>(),
            [
                ("my-agent", AgentSource::Detected, 0),
                ("codex", AgentSource::Detected, 0)
            ]
        );
    }
}
