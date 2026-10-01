// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The `[remote]` table: the route the city's own layer chooses for the
//! remote door (`crates/city/spec/ConfigLayers/Remote.lean` §8-39).
//!
//! The door opens onto the whole city, so only the city's own file may
//! choose how the outside reaches it; a building or a room that writes
//! the table is refused where the ladder reads it, by the judgement that
//! also keeps `[skills] shelves` in the city's file.
//!
//! What the table names is read here as written. Whether a tunnel name
//! or an address is one a route can use is `remote_access`'s answer,
//! and this crate sees only `kernel`, so the assembly judges those values
//! when it builds the route at each `/remote open` (city D16).

use std::path::Path;

use kernel::layout::CityLayout;
use kernel::{AxCode, AxError};
use serde::Deserialize;

use super::{Layer, ladder};

/// The table this module reads, spelled once for every refusal that
/// names it.
pub(crate) const REMOTE_KEY: &str = "[remote]";

/// Each route the table can choose, and the keys that route cannot do
/// without. The refusal for a city with no table is written from this
/// list, and the tests below write tables from it, so a key renamed in
/// [`RemoteRoute`] and left behind here turns those tests red.
const ROUTES: [(&str, [&str; 2]); 2] = [
    ("cloudflare", ["tunnel", "url"]),
    ("command", ["command", "permanence"]),
];

/// The route a city's `[remote]` table chooses, as written.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "route", rename_all = "snake_case", deny_unknown_fields)]
pub enum RemoteRoute {
    /// A Cloudflare named tunnel the person made once with `cloudflared`.
    Cloudflare {
        /// The name `cloudflared tunnel create` was given.
        tunnel: String,
        /// `https://` and the host the tunnel's DNS route points at.
        url: String,
        /// The `cloudflared` to run, when it is not the one on `PATH`.
        #[serde(default)]
        command: Option<String>,
    },
    /// A command the person wrote, which prints the address it made
    /// reachable and runs until the door closes.
    Command {
        command: String,
        #[serde(default)]
        args: Vec<String>,
        /// Stated rather than defaulted: a command cannot prove that its
        /// host name survives a restart.
        permanence: HostPermanence,
    },
}

/// Whether a command route's host name stays the same after the command
/// restarts. A device keeps its key under the host name, so on a host
/// that changes it pairs again after each restart.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HostPermanence {
    Fixed,
    PerStart,
}

/// The route the city's own layer chooses for the remote door.
///
/// The city's rung alone, not the ladder: the door opens onto the whole
/// city, and a lower rung that states the table is refused before this
/// reads it.
///
/// # Errors
/// `E_CONFIG_INVALID` naming the city's file and every key each route
/// needs when the file states no `[remote]` table; the refusals of a
/// city layer that exists and cannot be read or parsed.
pub fn remote_route(city_root: &Path) -> Result<RemoteRoute, AxError> {
    let file = CityLayout::new(city_root).city_config();
    ladder::stated(&file, Layer::City)?
        .remote()
        .cloned()
        .ok_or_else(|| unchosen(&file))
}

/// A route as one layer's file states it. A program named by an empty
/// string names nothing, and is a slip of the pen rather than a choice.
pub(super) fn stated(route: RemoteRoute) -> Result<RemoteRoute, AxError> {
    let program = match &route {
        RemoteRoute::Cloudflare { command, .. } => command.as_deref(),
        RemoteRoute::Command { command, .. } => Some(command.as_str()),
    };
    if program.is_some_and(|program| program.trim().is_empty()) {
        return Err(AxError::failure(
            AxCode::ConfigInvalid,
            "read a configuration layer",
            format!("`{REMOTE_KEY} command` is empty"),
        )
        .with_recovery("name the program the route runs, or its full path"));
    }
    Ok(route)
}

/// The refusal for a city whose own layer chooses no route: which file,
/// and the keys each route needs, written from [`ROUTES`].
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
        "write a `{REMOTE_KEY}` table there with {}; docs/operating.md shows both",
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

    /// A table for `route` with each key in `keys`, every value a word
    /// the parser takes for that key.
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

    /// Every key a route needs is refused by name when it is missing,
    /// and the keys the refusal for no table lists are the keys the
    /// parser reads.
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

    /// The door opens onto the whole city, so a building's file that
    /// chooses a route is refused where it was written, and the refusal
    /// says where the table belongs.
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
