// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::net::SocketAddr;
use std::path::Path;

use kernel::layout::CityLayout;
use kernel::{AxCode, AxError};
use serde::Deserialize;

use super::{Layer, ladder};

pub(crate) const REMOTE_KEY: &str = "[remote]";

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "route", rename_all = "snake_case", deny_unknown_fields)]
pub enum RemoteRoute {
    H3 {
        url: String,
        listen: SocketAddr,
        certificate: String,
        private_key: String,
    },
}

pub fn remote_route(city_root: &Path) -> Result<RemoteRoute, AxError> {
    let file = CityLayout::new(city_root).city_config();
    ladder::stated(&file, Layer::City)?
        .remote()
        .cloned()
        .ok_or_else(|| unchosen(&file))
}

fn unchosen(file: &Path) -> AxError {
    AxError::failure(
        AxCode::ConfigInvalid,
        "choose the remote door's route",
        format!("{}: no `{REMOTE_KEY}` table chooses a route", file.display()),
    )
    .with_recovery(
        r#"in that file set [remote] route = "h3", url, listen, certificate, and private_key; see docs/operate/operating.md#choosing-the-route"#,
    )
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests {
    use super::*;
    use crate::config_layers::ConfigLayer;

    const TABLE: &str = r#"[remote]
route = "h3"
url = "https://city.example.org"
listen = "127.0.0.1:443"
certificate = "/certificate.pem"
private_key = "/key.pem"
"#;

    #[test]
    fn only_the_native_h3_route_is_read() {
        let layer = ConfigLayer::parse(TABLE).unwrap();
        assert!(matches!(layer.remote(), Some(RemoteRoute::H3 { .. })));
        for text in [
            TABLE.replace("h3", "cloudflare"),
            TABLE.replace("h3", "command"),
            format!("{TABLE}command = 'caddy'"),
        ] {
            assert_eq!(
                *ConfigLayer::parse(&text).unwrap_err().code(),
                AxCode::ConfigInvalid
            );
        }
    }

    #[test]
    fn a_remote_table_belongs_to_the_city_file() {
        let dir = tempfile::tempdir().unwrap();
        let refused = remote_route(dir.path()).unwrap_err();
        assert!(refused.subject().contains(REMOTE_KEY));
        let file = CityLayout::new(dir.path()).city_config();
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(&file, TABLE).unwrap();
        assert!(ladder::stated(&file, Layer::Building).is_err());
        assert!(remote_route(dir.path()).is_ok());
    }
}
