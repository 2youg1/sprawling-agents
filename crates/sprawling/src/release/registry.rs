// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
//! Registry HTTP reads and decoding (`crates/sprawling/spec/Doctor.lean` §8-68).

use kernel::release::Version;
use kernel::{AxCode, AxError, Proxying, Reach, Release};

use super::RELEASES;

const USER_AGENT: &str = concat!("sprawling/", env!("CARGO_PKG_VERSION"));
const RELEASE_PAGE_SIZE: usize = 100;
const PATIENCE: std::time::Duration = std::time::Duration::from_secs(10);

pub(super) fn npm_newest(url: &str) -> Result<Release, AxError> {
    Release::from_npm_version(&manifest_field(url, &["version"])?)
}

pub(super) fn crates_newest(url: &str) -> Result<Version, AxError> {
    Version::from_crates_version(&manifest_field(url, &["crate", "max_version"])?)
}

fn manifest(url: &str) -> Result<serde_json::Value, AxError> {
    let client = gateway::client_for(Proxying::ExceptLocal, url)
        .timeout(PATIENCE)
        .user_agent(USER_AGENT)
        .build()
        .map_err(|err| {
            AxError::failure(AxCode::ToolUnavailable, "ask the registry", err.to_string())
                .with_recovery("this machine refused to build an HTTP client")
        })?;
    let response = client
        .get(url)
        .header("accept", "application/json")
        .send()
        .map_err(|_| stopped(&client, url))?;
    let status = response.status();
    if !status.is_success() {
        return Err(
            AxError::failure(AxCode::ToolUnavailable, "ask the registry", url.to_owned())
                .with_recovery(format!(
                    "the registry answered {status}; the release page carries every \
             archive either way: {RELEASES}"
                )),
        );
    }
    response.json().map_err(|err| {
        AxError::failure(
            AxCode::ToolUnavailable,
            "read the registry's answer",
            err.to_string(),
        )
        .with_recovery(
            "the registry answered something other than a version manifest; \
             a proxy that returns a login page does this",
        )
    })
}

fn manifest_field(url: &str, path: &[&str]) -> Result<String, AxError> {
    let body = manifest(url)?;
    path.iter()
        .try_fold(&body, |at, key| at.get(key))
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| {
            AxError::failure(
                AxCode::ToolUnavailable,
                "read the registry's answer",
                url.to_owned(),
            )
            .with_recovery(format!("the manifest carried no `{}`", path.join(".")))
        })
}

pub(super) struct GithubRelease {
    pub(super) cut: Release,
    pub(super) tag: String,
}

pub(super) fn github_newest(url: &str) -> Result<GithubRelease, AxError> {
    let body = manifest(&format!("{url}?per_page={RELEASE_PAGE_SIZE}"))?;
    let releases = body
        .as_array()
        .ok_or_else(|| registry_refused(url, "expected a release list"))?;
    for release in releases {
        if release.get("draft").and_then(serde_json::Value::as_bool) == Some(true) {
            continue;
        }
        let tag = release
            .get("tag_name")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| registry_refused(url, "a published release has no tag_name"))?;
        return Release::from_published_tag(tag).map(|cut| GithubRelease {
            cut,
            tag: tag.to_owned(),
        });
    }
    Err(registry_refused(
        url,
        "no published release is available; select a fixed tag from the release page",
    ))
}

fn registry_refused(url: &str, recovery: &str) -> AxError {
    AxError::failure(AxCode::ToolUnavailable, "read the release registry", url)
        .with_recovery(recovery)
}

fn stopped(client: &reqwest::blocking::Client, url: &str) -> AxError {
    let Reach {
        host,
        named,
        connected,
        answered,
        through,
        ..
    } = gateway::reach(client, Proxying::ExceptLocal, url, 0);
    AxError::failure(AxCode::ToolUnavailable, "ask the registry", host).with_recovery(format!(
        "the call stopped there: name {named:?}, socket {connected:?}, \
         request {answered:?}, through {through:?}"
    ))
}
