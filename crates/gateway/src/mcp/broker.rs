// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Three calls and nothing else: what is on the shelf, an auth config
//! to connect through, and the consent page to send a person to.
//!
//! **The field names below are read defensively on purpose.** They are
//! this module's reading of somebody else's JSON, and a missing field
//! costs one row rather than the whole answer - a directory that
//! refuses to draw because one application grew a field is worse than a
//! directory with one unnamed row. What pins the reading is the fake
//! server in the tests, which is this crate's statement of what it
//! believes the broker sends.

use std::time::Duration;

use kernel::{AxCode, AxError, Proxying, Sealed};
use serde_json::{Value, json};

use crate::reach::client_for;

#[cfg(test)]
mod tests;

/// Where the broker answers.
const BASE: &str = "https://backend.composio.dev/";

/// The header the broker reads its project key from.
const KEY_HEADER: &str = "x-api-key";

/// How long one call to the broker may take.
///
/// A person is watching a page through every one of these. The bound is
/// short enough that "the broker is unreachable" arrives as an answer
/// rather than as a spinner nobody can interpret, and three calls at
/// this bound still finish inside what somebody will sit through.
const DEADLINE: Duration = Duration::from_secs(10);

/// One application on the broker's shelf.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Toolkit {
    pub slug: String,
    pub name: String,
    /// How it authenticates, in the broker's own word. Carried rather
    /// than reduced to a flag: this city is not the authority on which
    /// schemes exist, and the word decides what the person is shown.
    pub auth: String,
    pub standing: Connection,
}

/// Where one application stands for one person.
///
/// Every arm is the broker's reading rather than something remembered
/// here: a consent page opened yesterday is not evidence that anybody
/// finished with it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Connection {
    /// Nothing has connected it.
    Absent,
    /// A consent page exists and has not been completed.
    Awaiting { consent_url: String },
    /// Connected, under the account name the broker returned.
    Connected { alias: String },
    /// The broker still reports the last attempt as failed.
    Refused { refusal: AxError },
}

/// A reachable broker holding one project's key.
///
/// Constructed once per read, because the key is redeemed once per read
/// and this type is what holds it. The proxy rule is settled at
/// construction and never re-decided: `gateway::reach` is the one
/// authority on whether a call goes through the machine's proxy.
pub struct Broker {
    client: reqwest::blocking::Client,
    key: Sealed<String>,
    /// Where this broker answers. A field rather than the constant
    /// alone, because a test that cannot point the three calls at a
    /// server it controls cannot state what this module believes the
    /// broker sends.
    base: reqwest::Url,
}

impl Broker {
    /// Binds a key to a client that reaches the broker the way this
    /// machine is configured to.
    pub fn new(key: Sealed<String>, rule: Proxying) -> Result<Self, AxError> {
        let client = client_for(rule, BASE)
            .timeout(DEADLINE)
            .build()
            .map_err(|failed| {
                AxError::failure(
                    AxCode::Provider,
                    "build a client for the application broker",
                    failed.to_string(),
                )
                .with_recovery("check the proxy settings on the endpoint page")
            })?;
        Ok(Broker {
            client,
            key,
            base: parsed(BASE)?,
        })
    }

    /// The same broker, answering somewhere else. Test seam.
    #[cfg(test)]
    fn at(base: &str, key: Sealed<String>) -> Result<Self, AxError> {
        Ok(Broker {
            client: reqwest::blocking::Client::builder()
                .timeout(DEADLINE)
                .no_proxy()
                .build()
                .map_err(|failed| {
                    AxError::failure(AxCode::Provider, "build a test client", failed.to_string())
                })?,
            key,
            base: parsed(base)?,
        })
    }

    /// The shelf, joined with where each row stands for this person.
    ///
    /// Two calls rather than one: the directory is the same for
    /// everybody and the standings are this person's, and the broker
    /// publishes them separately. A row the directory names but the
    /// standings do not is `Absent`, which is the truth about a row
    /// nobody has connected.
    pub fn shelf(&self, user: &str) -> Result<Vec<Toolkit>, AxError> {
        let directory = self.get("api/v3/toolkits", &[])?;
        let standings = self.standings(user)?;
        let mut shelf = Vec::new();
        for item in items_of(&directory) {
            let Some(slug) = text_at(item, "slug") else {
                continue;
            };
            let standing = standings
                .iter()
                .find(|(named, _)| named == &slug)
                .map_or(Connection::Absent, |(_, held)| held.clone());
            shelf.push(Toolkit {
                name: text_at(item, "name").unwrap_or_else(|| slug.clone()),
                auth: scheme_of(item),
                slug,
                standing,
            });
        }
        Ok(shelf)
    }

    /// Opens a consent session and answers with the page to send the
    /// person to.
    ///
    /// The auth config is found or created first. Creating one needs no
    /// visit to the broker's own dashboard and no OAuth client of the
    /// person's: the broker keeps a managed application for exactly
    /// this, which is what makes one press enough.
    pub fn connect(&self, slug: &str, user: &str) -> Result<String, AxError> {
        let config = self.auth_config_for(slug)?;
        let opened = self.post(
            "api/v3/connected_accounts/link",
            &json!({ "auth_config_id": config, "user_id": user }),
        )?;
        text_at(&opened, "redirect_url")
            .or_else(|| text_at(&opened, "redirectUrl"))
            .ok_or_else(|| {
                AxError::failure(
                    AxCode::Provider,
                    "open a consent page for an outside application",
                    format!("the broker answered for {slug} without a page to open"),
                )
                .with_recovery("connect this application on the broker's own pages instead")
            })
    }

    /// An auth config for this application, reused when one exists.
    ///
    /// Reused rather than created every time, because each one is a
    /// lasting record on the broker's side and a city that made a fresh
    /// one per press would leave a trail nobody asked for.
    fn auth_config_for(&self, slug: &str) -> Result<String, AxError> {
        let held = self.get("api/v3/auth_configs", &[("toolkit_slug", slug)])?;
        if let Some(found) = items_of(&held).iter().find_map(|item| text_at(item, "id")) {
            return Ok(found);
        }
        let made = self.post(
            "api/v3/auth_configs",
            &json!({
                "toolkit": { "slug": slug },
                "auth_config": { "type": "use_composio_managed_auth" }
            }),
        )?;
        text_at(&made, "id")
            .or_else(|| text_at(made.get("auth_config").unwrap_or(&Value::Null), "id"))
            .ok_or_else(|| {
                AxError::failure(
                    AxCode::Provider,
                    "create an auth config for an outside application",
                    format!("the broker accepted {slug} without naming the config it made"),
                )
                .with_recovery("connect this application on the broker's own pages instead")
            })
    }

    /// Where this person's connections stand, by application slug.
    fn standings(&self, user: &str) -> Result<Vec<(String, Connection)>, AxError> {
        let held = self.get("api/v3/connected_accounts", &[("user_ids", user)])?;
        Ok(items_of(&held)
            .iter()
            .filter_map(|item| {
                let slug = text_at(item, "toolkit_slug")
                    .or_else(|| text_at(item.get("toolkit").unwrap_or(&Value::Null), "slug"))?;
                Some((slug, standing_of(item)))
            })
            .collect())
    }

    fn get(&self, path: &str, pairs: &[(&str, &str)]) -> Result<Value, AxError> {
        let target = self.url_for(path, pairs)?;
        let sent = self
            .client
            .get(target)
            .header(KEY_HEADER, self.key.expose())
            .send();
        read(sent, path)
    }

    fn post(&self, path: &str, body: &Value) -> Result<Value, AxError> {
        let target = self.url_for(path, &[])?;
        let sent = self
            .client
            .post(target)
            .header(KEY_HEADER, self.key.expose())
            .json(body)
            .send();
        read(sent, path)
    }
}

/// The broker's word for a connection, read into the four states a
/// person acts on.
///
/// An unrecognised word reads as `Absent` rather than as a failure: a
/// state this build has not heard of is a state in which nothing here
/// has connected anything, and offering the button again is the one
/// action that cannot make it worse.
fn standing_of(item: &Value) -> Connection {
    let alias = text_at(item, "alias")
        .or_else(|| text_at(item, "id"))
        .unwrap_or_else(|| "connected".to_owned());
    match text_at(item, "status").unwrap_or_default().as_str() {
        "ACTIVE" => Connection::Connected { alias },
        "INITIATED" | "PENDING" => match text_at(item, "redirect_url") {
            Some(consent_url) => Connection::Awaiting { consent_url },
            None => Connection::Absent,
        },
        "FAILED" | "EXPIRED" => Connection::Refused {
            refusal: AxError::failure(
                AxCode::Provider,
                "connect an outside application",
                text_at(item, "status_reason")
                    .unwrap_or_else(|| "the broker reported the connection as failed".to_owned()),
            )
            .with_recovery("connect it again, and complete the consent page this time"),
        },
        _ => Connection::Absent,
    }
}

/// Which authentication scheme a directory row names, in the broker's
/// own word.
fn scheme_of(item: &Value) -> String {
    item.get("auth_schemes")
        .and_then(Value::as_array)
        .and_then(|schemes| schemes.first())
        .and_then(Value::as_str)
        .map_or_else(|| "oauth2".to_owned(), str::to_lowercase)
}

/// The rows of a paged answer, or none when this is not one.
fn items_of(body: &Value) -> Vec<&Value> {
    body.get("items")
        .or_else(|| body.get("data"))
        .and_then(Value::as_array)
        .map(|rows| rows.iter().collect())
        .unwrap_or_default()
}

/// One string field, absent when it is missing or is not a string.
fn text_at(body: &Value, field: &str) -> Option<String> {
    body.get(field).and_then(Value::as_str).map(str::to_owned)
}

/// One base url, parsed once.
fn parsed(base: &str) -> Result<reqwest::Url, AxError> {
    reqwest::Url::parse(base).map_err(|bad| {
        AxError::failure(
            AxCode::Provider,
            "address the application broker",
            bad.to_string(),
        )
    })
}

impl Broker {
    /// Builds a request url with every pair percent-encoded.
    ///
    /// Encoding lives here because this is what builds the path: a slug
    /// arrives from a client frame, and the only thing standing between
    /// it and the query string is this method.
    fn url_for(&self, path: &str, pairs: &[(&str, &str)]) -> Result<reqwest::Url, AxError> {
        let mut target = self.base.join(path).map_err(|bad| {
            AxError::failure(
                AxCode::Provider,
                "address the application broker",
                bad.to_string(),
            )
        })?;
        {
            let mut query = target.query_pairs_mut();
            for (name, value) in pairs {
                query.append_pair(name, value);
            }
        }
        Ok(target)
    }
}

/// Reads one answer, turning a status into a code a person can act on.
fn read(sent: reqwest::Result<reqwest::blocking::Response>, path: &str) -> Result<Value, AxError> {
    let response = sent.map_err(|failed| {
        let refusal = AxError::failure(
            AxCode::Provider,
            "reach the application broker",
            failed.to_string(),
        )
        .with_recovery("check this machine's connection, then try again");
        if failed.is_timeout() {
            return refusal.retriable();
        }
        refusal
    })?;
    let status = response.status();
    let body = response.text().unwrap_or_default();
    if status.is_success() {
        return serde_json::from_str(&body).map_err(|bad| {
            AxError::failure(
                AxCode::Provider,
                format!("read the broker's answer to {path}"),
                bad.to_string(),
            )
            .with_recovery("try again, and report this if it keeps happening")
        });
    }
    Err(refusal_for(status, path, &body))
}

/// One refused status, as the code and recovery a person acts on.
fn refusal_for(status: reqwest::StatusCode, path: &str, body: &str) -> AxError {
    let subject = format!("{path} answered {status}");
    match status.as_u16() {
        401 | 403 => AxError::failure(AxCode::CredentialMissing, "ask the broker", subject)
            .with_recovery("store a valid project key for the broker and try again"),
        408 | 429 => AxError::failure(AxCode::Provider, "ask the broker", subject)
            .with_recovery("wait a moment and try again")
            .retriable(),
        500..=599 => AxError::failure(AxCode::Provider, "ask the broker", subject)
            .with_recovery("the broker is having trouble; try again shortly")
            .retriable(),
        _ => AxError::failure(AxCode::Provider, "ask the broker", subject)
            .with_nearby(vec![body.chars().take(200).collect()])
            .with_recovery("connect this application on the broker's own pages instead"),
    }
}
