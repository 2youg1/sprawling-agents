// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::net::SocketAddr;
use std::path::Path;

use kernel::layout::CityLayout;
use kernel::{AxCode, AxError};
use serde::Deserialize;

use super::refuse::refuse_with;
use super::{Layer, ladder};

pub(crate) const REMOTE_KEY: &str = "[remote]";

const ROUTES: [(&str, [&str; 2]); 2] = [
    ("cloudflare", ["tunnel", "url"]),
    ("command", ["command", "permanence"]),
];

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "route", rename_all = "snake_case", deny_unknown_fields)]
pub enum RemoteRoute {
    H3 {
        command: String,
        url: String,
        listen: SocketAddr,
        certificate: String,
        private_key: String,
    },

    Cloudflare {
        tunnel: String,

        url: String,

        #[serde(default)]
        command: Option<String>,
    },

    Command {
        command: String,
        #[serde(default)]
        args: Vec<String>,

        permanence: HostPermanence,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HostPermanence {
    Fixed,
    PerStart,
}

pub fn remote_route(city_root: &Path) -> Result<RemoteRoute, AxError> {
    let file = CityLayout::new(city_root).city_config();
    ladder::stated(&file, Layer::City)?
        .remote()
        .cloned()
        .ok_or_else(|| unchosen(&file))
}

pub(super) fn stated(route: RemoteRoute) -> Result<RemoteRoute, AxError> {
    let program = match &route {
        RemoteRoute::Cloudflare { command, .. } => command.as_deref(),
        RemoteRoute::Command { command, .. } | RemoteRoute::H3 { command, .. } => {
            Some(command.as_str())
        }
    };
    if program.is_some_and(|program| program.trim().is_empty()) {
        return Err(refuse_with(
            format!("`{REMOTE_KEY} command` is empty"),
            "name the program the route runs, or its full path",
        ));
    }
    Ok(route)
}

fn unchosen(file: &Path) -> AxError {
    let routes: Vec<String> = ROUTES
        .iter()
        .map(|(route, [first, second])| {
            format!("`route = \"{route}\"` with `{first}` and `{second}`")
        })
        .collect();
    AxError::failure(
        AxCode::ConfigInvalid,
        "choose the remote door's route",
        format!(
            "{}: no `{REMOTE_KEY}` table chooses a route",
            file.display()
        ),
    )
    .with_recovery(format!(
        "write a `{REMOTE_KEY}` table there with `route = \"h3\"` for public HTTP/3; compatibility shapes are {}; docs/operate/operating.md shows the gateway settings",
        routes.join(", or ")
    ))
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests {
    use super::*;
    use crate::config_layers::{ConfigLayer, Layer, ladder};

    fn table(route: &str, keys: &[&str]) -> String {
        let lines: String = keys
            .iter()
            .map(|key| match *key {
                "permanence" => format!("{key} = \"fixed\"\n"),
                _ => format!("{key} = \"x\"\n"),
            })
            .collect();
        format!("{REMOTE_KEY}\nroute = \"{route}\"\n{lines}")
    }

    fn city_file(dir: &Path, text: &str) -> std::path::PathBuf {
        let file = CityLayout::new(dir).city_config();
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(&file, text).unwrap();
        file
    }

    #[test]
    fn each_route_reads_back_as_it_was_written() {
        let read = |text: &str| ConfigLayer::parse(text).map(|layer| layer.remote().cloned());
        assert_eq!(
            [
                read(
                    "[remote]\nroute = \"cloudflare\"\ntunnel = \"my-city\"\n\
                     url = \"https://city.example.org\"\n"
                ),
                read(
                    "[remote]\nroute = \"command\"\ncommand = \"sh\"\n\
                     args = [\"route.sh\", \"--quiet\"]\npermanence = \"per_start\"\n"
                ),
            ],
            [
                Ok(Some(RemoteRoute::Cloudflare {
                    tunnel: "my-city".to_owned(),
                    url: "https://city.example.org".to_owned(),
                    command: None,
                })),
                Ok(Some(RemoteRoute::Command {
                    command: "sh".to_owned(),
                    args: vec!["route.sh".to_owned(), "--quiet".to_owned()],
                    permanence: HostPermanence::PerStart,
                })),
            ]
        );
    }

    #[test]
    fn a_route_missing_a_key_it_needs_is_refused_naming_the_key() {
        for (route, keys) in ROUTES {
            assert!(
                ConfigLayer::parse(&table(route, &keys)).is_ok(),
                "a `{route}` route with {keys:?} is whole"
            );
            for missing in keys {
                let kept: Vec<&str> = keys.into_iter().filter(|key| *key != missing).collect();
                let refused = ConfigLayer::parse(&table(route, &kept)).unwrap_err();
                assert!(
                    *refused.code() == AxCode::ConfigInvalid
                        && refused.subject().contains(&format!("`{missing}`")),
                    "a `{route}` route without `{missing}`: {refused}"
                );
            }
        }
    }

    #[test]
    fn a_remote_table_below_the_city_layer_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let file = city_file(dir.path(), &table("command", &ROUTES[1].1));
        let below = ladder::stated(&file, Layer::Building)
            .map(|_| ())
            .unwrap_err();
        assert_eq!(
            (
                *below.code(),
                below.recovery().contains(&format!("move `{REMOTE_KEY}`")),
                ladder::stated(&file, Layer::City).is_ok()
            ),
            (AxCode::ConfigInvalid, true, true),
            "{below}: {}",
            below.recovery()
        );
    }

    #[test]
    fn a_city_with_no_remote_table_is_refused_naming_each_route_and_its_keys() {
        let dir = tempfile::tempdir().unwrap();
        city_file(dir.path(), "[clock]\nstamp = \"minute\"\n");
        let refused = remote_route(dir.path()).unwrap_err();
        let named = ROUTES.iter().all(|(route, keys)| {
            refused.recovery().contains(&format!("route = \"{route}\""))
                && keys
                    .iter()
                    .all(|key| refused.recovery().contains(&format!("`{key}`")))
        });
        assert!(
            *refused.code() == AxCode::ConfigInvalid
                && refused.subject().contains(REMOTE_KEY)
                && named,
            "{refused}: {}",
            refused.recovery()
        );
    }
}
