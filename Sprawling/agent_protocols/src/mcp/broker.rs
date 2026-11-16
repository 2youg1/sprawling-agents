// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::time::Duration;

use gateway::client_for;
use kernel::{AxCode, AxError, Proxying, Sealed};
use serde_json::{Value, json};

#[cfg(test)]
mod tests;

const BASE: &str = "https://backend.composio.dev/";

const KEY_HEADER: &str = "x-api-key";

const DEADLINE: Duration = Duration::from_secs(10);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Toolkit {
    pub slug: String,
    pub name: String,

    pub auth: String,
    pub standing: Connection,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Connection {
    Absent,

    Awaiting { consent_url: String },

    Connected { alias: String },

    Refused { refusal: AxError },
}

pub struct Broker {
    client: reqwest::blocking::Client,
    key: Sealed<String>,

    base: reqwest::Url,
}

impl Broker {
    pub fn new(key: Sealed<String>, rule: Proxying) -> Result<Self, AxError> {
        Self::built(key, rule, BASE)
    }

    #[cfg(test)]
    fn at(base: &str, key: Sealed<String>) -> Result<Self, AxError> {
        Self::built(key, Proxying::ExceptLocal, base)
    }

    fn built(key: Sealed<String>, rule: Proxying, base: &str) -> Result<Self, AxError> {
        let client = client_for(rule, base)
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
            base: parsed(base)?,
        })
    }

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

fn scheme_of(item: &Value) -> String {
    item.get("auth_schemes")
        .and_then(Value::as_array)
        .and_then(|schemes| schemes.first())
        .and_then(Value::as_str)
        .map_or_else(|| "oauth2".to_owned(), str::to_lowercase)
}

fn items_of(body: &Value) -> Vec<&Value> {
    body.get("items")
        .or_else(|| body.get("data"))
        .and_then(Value::as_array)
        .map(|rows| rows.iter().collect())
        .unwrap_or_default()
}

fn text_at(body: &Value, field: &str) -> Option<String> {
    body.get(field).and_then(Value::as_str).map(str::to_owned)
}

fn parsed(base: &str) -> Result<reqwest::Url, AxError> {
    reqwest::Url::parse(base).map_err(|bad| {
        AxError::failure(
            AxCode::Provider,
            "address the application broker",
            bad.to_string(),
        )
        .with_recovery(format!(
            "give the broker an absolute url with a scheme and a host, for example \
             `https://example.com/api`; this build was given `{base}`"
        ))
    })
}

impl Broker {
    fn url_for(&self, path: &str, pairs: &[(&str, &str)]) -> Result<reqwest::Url, AxError> {
        let mut target = self.base.join(path).map_err(|bad| {
            AxError::failure(
                AxCode::Provider,
                "address the application broker",
                bad.to_string(),
            )
            .with_recovery(format!(
                "report this against {}: the path `{path}` does not join onto the \
                 broker's base url",
                module_path!()
            ))
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

fn read(sent: reqwest::Result<reqwest::blocking::Response>, path: &str) -> Result<Value, AxError> {
    let response = sent.map_err(|failed| {
        let draft = AxError::failure(
            AxCode::Provider,
            "reach the application broker",
            failed.to_string(),
        );

        let draft = match (failed.is_timeout(), failed.is_connect()) {
            (true, true) => draft.retriable(),
            (true, false) => draft.effect_unknown(),
            (false, true | false) => draft,
        };
        draft.with_recovery("check this machine's connection, then try again")
    })?;
    let status = response.status();
    let body = response.text();
    if status.is_success() {
        let body = body.map_err(|cut| {
            AxError::failure(
                AxCode::Provider,
                format!("read the broker's answer to {path}"),
                format!("the answer's body could not be read: {cut}"),
            )
            .effect_unknown()
            .with_recovery("check the broker's own pages for what this request made")
        })?;
        return serde_json::from_str(&body).map_err(|bad| {
            AxError::failure(
                AxCode::Provider,
                format!("read the broker's answer to {path}"),
                bad.to_string(),
            )
            .with_recovery("try again, and report this if it keeps happening")
        });
    }
    let nearby = body.unwrap_or_else(|cut| cut.to_string());
    Err(refusal_for(status, path, &nearby))
}

fn refusal_for(status: reqwest::StatusCode, path: &str, body: &str) -> AxError {
    let subject = format!("{path} answered {status}");
    match status.as_u16() {
        401 | 403 => AxError::failure(AxCode::CredentialMissing, "ask the broker", subject)
            .with_recovery("store a valid project key for the broker and try again"),
        408 | 429 => AxError::failure(AxCode::Provider, "ask the broker", subject)
            .retriable()
            .with_recovery("wait a moment and try again"),
        500..=599 => AxError::failure(AxCode::Provider, "ask the broker", subject)
            .retriable()
            .with_recovery("the broker is having trouble; try again shortly"),
        _ => AxError::failure(AxCode::Provider, "ask the broker", subject)
            .with_nearby(vec![body.chars().take(200).collect()])
            .with_recovery("connect this application on the broker's own pages instead"),
    }
}
