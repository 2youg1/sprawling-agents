// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{McpServer, McpTransport, ServerLabel};

use super::Laying;
use crate::worker::Site;

pub(in crate::worker) const DESKTOP_LABEL: &str = "desktop";

const DESKTOP_VERB: &str = "desktop";

impl Laying {
    pub(in crate::worker) fn servers(&self, site: &Site) -> Vec<McpServer> {
        let mut servers = site.config.mcp.clone();
        if !site.rules.desktop() {
            return servers;
        }
        if servers
            .iter()
            .any(|server| server.label.as_str() == DESKTOP_LABEL)
        {
            self.note(
                runtime::diagnostics::Level::Decide,
                "crate::worker::workbench::desktop",
                "this building names its own `desktop` server, so the one this binary \
                 carries is not started; remove that [[mcp]] row to use the built-in one",
            );
            return servers;
        }
        match self.desktop_server(site) {
            Ok(server) => servers.push(server),
            Err(err) => self.note(
                runtime::diagnostics::Level::Refuse,
                "crate::worker::workbench::desktop",
                &format!("the desktop is not offered: {err}; {}", err.recovery()),
            ),
        }
        servers
    }

    fn desktop_server(&self, site: &Site) -> Result<McpServer, kernel::AxError> {
        let program = (self.desktop_program)().map_err(|err| unavailable(&err.to_string()))?;
        let scope = city::desktop_scope_path(&self.city_root, site.building.addr());
        let spelled = |path: &std::path::Path| {
            path.to_str()
                .map(str::to_owned)
                .ok_or_else(|| unavailable(&format!("{} is not UTF-8", path.display())))
        };
        Ok(McpServer {
            label: ServerLabel::parse(DESKTOP_LABEL)?,
            transport: McpTransport::Stdio {
                command: spelled(&program)?,
                args: vec![DESKTOP_VERB.to_owned(), spelled(&scope)?],
                env: Vec::new(),
            },
        })
    }
}

fn unavailable(subject: &str) -> kernel::AxError {
    kernel::AxError::failure(
        kernel::AxCode::ToolUnavailable,
        "start this machine's desktop server",
        subject.to_owned(),
    )
    .with_recovery("run sprawling from a folder whose path is plain UTF-8, then dispatch again")
}
