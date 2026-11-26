// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::io::{BufRead, BufReader, Write};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::mpsc;
use std::time::Duration;

use kernel::{AxCode, AxError, TimeoutMs};
use serde_json::{Value, json};

use super::{Opened, Permanence, PublicUrl, Route, Running};

const RECOVERY: &str = "serve the public hostname with route = \"h3\", an absolute Caddy path, and its TLS certificate and private key";

pub struct H3 {
    pub program: PathBuf,
    pub url: PublicUrl,
    pub listen: SocketAddr,
    pub certificate: PathBuf,
    pub private_key: PathBuf,
    pub storage: PathBuf,
}

pub struct Caddy {
    settings: H3,
    host: String,
    patience: TimeoutMs,
    running: Option<Running>,
    #[cfg(windows)]
    job: Option<win32job::Job>,
}

impl Caddy {
    pub fn new(settings: H3, patience: TimeoutMs) -> Result<Self, AxError> {
        let authority = settings.url.as_str().trim_start_matches("https://");
        let host = authority
            .split_once(':')
            .map_or(authority, |(host, _)| host);
        let labels_are_dns = host.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        });
        if !labels_are_dns
            || authority.contains(['/', '?', '#'])
            || authority
                .split_once(':')
                .is_some_and(|(_, port)| port.parse::<u16>().is_err())
        {
            return Err(refused(format!(
                "{} is not an HTTPS DNS origin",
                settings.url.as_str()
            )));
        }
        if let Some((field, path)) = [
            ("command", &settings.program),
            ("certificate", &settings.certificate),
            ("private_key", &settings.private_key),
        ]
        .into_iter()
        .find(|(_, path)| !path.is_absolute())
        {
            return Err(
                refused(format!("{field} = {:?}: a relative path", path.display()))
                    .rewrite_recovery(format!(
                        "set [remote] {field} to the full absolute path in the city's CONFIG.toml"
                    )),
            );
        }
        if !settings.storage.is_absolute() {
            return Err(refused(format!(
                "{}: the city storage path is relative",
                settings.storage.display()
            ))
            .rewrite_recovery("open the city from its full absolute workspace path"));
        }
        let host = host.to_owned();
        Ok(Self {
            settings,
            host,
            patience,
            running: None,
            #[cfg(windows)]
            job: None,
        })
    }

    fn config(&self, local: SocketAddr) -> Value {
        json!({
            "admin": { "disabled": true, "config": { "persist": false } },
            "storage": { "module": "file_system", "root": self.settings.storage },
            "apps": {
                "tls": { "certificates": { "load_files": [{
                    "certificate": self.settings.certificate,
                    "key": self.settings.private_key,
                }] } },
                "http": { "servers": { "sprawling": {
                    "listen": [self.settings.listen.to_string()],
                    "protocols": ["h3"],
                    "allow_0rtt": false,
                    "strict_sni_host": true,
                    "automatic_https": { "disable": true },
                    "tls_connection_policies": [{}],
                    "routes": [{
                        "match": [{ "host": [&self.host] }],
                        "handle": [{
                            "handler": "reverse_proxy",
                            "upstreams": [{ "dial": local.to_string() }],
                        }],
                        "terminal": true,
                    }],
                } } },
            },
        })
    }
}

impl Route for Caddy {
    fn open(&mut self, local: SocketAddr) -> Result<Opened, AxError> {
        self.close()?;
        if !local.ip().is_loopback() {
            return Err(refused(format!("upstream {local} is outside loopback")));
        }
        let mut running = Running::start(
            child::command(&self.settings.program)
                .args(["run", "--config", "-"])
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::piped()),
            RECOVERY,
        )?;
        #[cfg(windows)]
        let job = joined(&running)?;
        let stderr = running
            .stderr()
            .ok_or_else(|| refused("Caddy supplied no startup log"))?;
        let (ready, started) = mpsc::sync_channel(1);
        std::thread::Builder::new()
            .name("route-caddy".to_owned())
            .spawn(move || {
                let mut reported = false;
                let mut failure = None;
                for line in BufReader::new(stderr).lines() {
                    let Ok(line) = line else {
                        break;
                    };
                    if reported {
                        continue;
                    }
                    if line.starts_with("Error: ") {
                        failure = Some(line);
                        continue;
                    }
                    let Ok(event) = serde_json::from_str::<Value>(&line) else {
                        continue;
                    };
                    if event.get("msg").and_then(Value::as_str)
                        == Some("serving initial configuration")
                    {
                        reported = true;
                        drop(ready.try_send(Ok(())));
                    }
                }
                if !reported {
                    drop(ready.try_send(Err(failure.unwrap_or_else(|| {
                        "Caddy exited before confirming the HTTP/3 listener".to_owned()
                    }))));
                }
            })
            .map_err(|source| refused(source.to_string()))?;
        let mut config = running
            .stdin()
            .ok_or_else(|| refused("Caddy supplied no configuration input"))?;
        let bytes = self.config(local).to_string();
        config
            .write_all(bytes.as_bytes())
            .map_err(|source| refused(source.to_string()))?;
        drop(config);
        started
            .recv_timeout(Duration::from_millis(self.patience.0))
            .map_err(|source| {
                refused(format!(
                    "{}: startup was not confirmed: {source}",
                    self.settings.program.display()
                ))
            })?
            .map_err(refused)?;
        self.running = Some(running);
        #[cfg(windows)]
        {
            self.job = Some(job);
        }
        Ok(Opened {
            url: self.settings.url.clone(),
            permanence: Permanence::Fixed,
        })
    }

    fn close(&mut self) -> Result<(), AxError> {
        let closed = self
            .running
            .take()
            .map_or(Ok(()), |mut running| running.stop());
        #[cfg(windows)]
        drop(self.job.take());
        closed
    }
}

#[cfg(windows)]
fn joined(running: &Running) -> Result<win32job::Job, AxError> {
    use std::os::windows::io::AsRawHandle;

    let child = running
        .child
        .as_ref()
        .ok_or_else(|| refused("Caddy is already stopped"))?;
    let handle = isize::try_from(child.as_raw_handle().addr())
        .map_err(|source| refused(source.to_string()))?;
    let mut limits = win32job::ExtendedLimitInfo::default();
    limits.limit_kill_on_job_close();
    let job = win32job::Job::create_with_limit_info(&limits)
        .map_err(|source| refused(source.to_string()))?;
    job.assign_process(handle)
        .map_err(|source| refused(source.to_string()))?;
    Ok(job)
}

fn refused(subject: impl Into<String>) -> AxError {
    AxError::failure(
        AxCode::ConfigInvalid,
        "open the public HTTP/3 gateway",
        subject.into(),
    )
    .with_recovery(RECOVERY)
}
