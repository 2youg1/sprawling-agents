// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::io::{BufRead, Write};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::time::Duration;

const PROBE_ATTEMPTS: u32 = 60;
const PROBE_INTERVAL: Duration = Duration::from_millis(200);

pub enum FirstScreen {
    Start(PathBuf),

    Use(PathBuf),
    Quit,
}

pub fn ask<R: BufRead, W: Write>(
    city: &Path,
    input: &mut R,
    out: &mut W,
) -> std::io::Result<FirstScreen> {
    writeln!(out, "sprawling  no city yet")?;
    writeln!(
        out,
        "start one at {}?   Enter yes   type a folder to use it   q quit",
        city.display()
    )?;
    writeln!(
        out,
        "  a folder you type becomes a city around your work, and your files stay as they are"
    )?;
    write!(out, "> ")?;
    out.flush()?;

    let mut answer = String::new();
    let read = input.read_line(&mut answer)?;
    if read == 0 {
        return Ok(FirstScreen::Quit);
    }
    let answer = answer.trim();
    if answer.is_empty() || answer.eq_ignore_ascii_case("y") || answer.eq_ignore_ascii_case("yes") {
        return Ok(FirstScreen::Start(city.to_path_buf()));
    }
    if answer.eq_ignore_ascii_case("q") || answer.eq_ignore_ascii_case("quit") {
        return Ok(FirstScreen::Quit);
    }

    Ok(FirstScreen::Use(PathBuf::from(strip_quotes(answer))))
}

fn strip_quotes(answer: &str) -> &str {
    answer
        .strip_prefix('"')
        .and_then(|rest| rest.strip_suffix('"'))
        .or_else(|| {
            answer
                .strip_prefix('\'')
                .and_then(|rest| rest.strip_suffix('\''))
        })
        .unwrap_or(answer)
}

fn reachable(bind: SocketAddr) -> SocketAddr {
    if bind.ip().is_unspecified() {
        let loopback = std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST);
        return SocketAddr::new(loopback, bind.port());
    }
    bind
}

pub fn open_when_ready(bind: SocketAddr, url: String, door: wire::LocalDoor) {
    let addr = reachable(bind);
    std::thread::spawn(move || {
        for _ in 0..PROBE_ATTEMPTS {
            if std::net::TcpStream::connect_timeout(&addr, PROBE_INTERVAL).is_ok() {
                if let Err(unopened) = open_paired(&url, &door) {
                    eprintln!("  {}: {}", unopened.action(), unopened.recovery());
                }
                return;
            }
            std::thread::sleep(PROBE_INTERVAL);
        }
    });
}

pub(crate) fn open_paired(url: &str, door: &wire::LocalDoor) -> Result<(), kernel::AxError> {
    let unhandled = |source: std::io::Error| {
        kernel::AxError::failure(
            kernel::AxCode::ToolUnavailable,
            "open the WebUI in a browser",
            source.to_string(),
        )
        .with_recovery("open the address the city printed, and type the pairing code it shows")
    };
    if std::env::var_os("WSL_DISTRO_NAME").is_some() {
        return open_in_browser(url, unhandled);
    }
    let wire::OpenCode { code, redeemed } = door.issue_open_code()?;
    let page = crate::serving::key_file::runtime_dir()?.join(format!(
        "{REDIRECT_PREFIX}{}{REDIRECT_SUFFIX}",
        std::process::id()
    ));
    let target = format!("{}/#open={code}", url.trim_end_matches('/'));
    let redirect = format!(
        "<!doctype html><meta charset=\"utf-8\"><meta http-equiv=\"refresh\" content=\"0;url={target}\">\
         <script>location.replace({target:?})</script>\n"
    );
    write_private(&page, &redirect).map_err(unhandled)?;
    let opened = open_in_browser(&page.to_string_lossy(), unhandled);
    std::thread::spawn(move || {
        let _waited: Result<(), std::sync::mpsc::RecvTimeoutError> =
            redeemed.recv_timeout(Duration::from_millis(wire::OPEN_CODE_LIFETIME_MS));
        if let Err(left) = std::fs::remove_file(&page) {
            eprintln!(
                "  remove the browser's redirect page {}: {left}",
                page.display()
            );
        }
    });
    opened
}

fn write_private(path: &Path, text: &str) -> std::io::Result<()> {
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.mode(0o600);
    }
    options.open(path)?.write_all(text.as_bytes())
}

const REDIRECT_PREFIX: &str = "open-";
const REDIRECT_SUFFIX: &str = ".html";

pub fn sweep_expired_redirects(now: kernel::TimeMs) -> Result<(), kernel::AxError> {
    let dir = crate::serving::key_file::runtime_dir()?;
    let unswept = |source: std::io::Error| {
        kernel::AxError::failure(
            kernel::AxCode::StorageFatal,
            "remove the expired browser redirect pages",
            format!("{}: {source}", dir.display()),
        )
        .with_recovery("delete the open-*.html files in that directory; their codes have expired")
    };
    let age = Duration::from_millis(now.value())
        .saturating_sub(Duration::from_millis(wire::OPEN_CODE_LIFETIME_MS));

    let Some(cutoff) = std::time::UNIX_EPOCH.checked_add(age) else {
        return Ok(());
    };
    for entry in std::fs::read_dir(&dir).map_err(unswept)? {
        let entry = entry.map_err(unswept)?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !(name.starts_with(REDIRECT_PREFIX) && name.ends_with(REDIRECT_SUFFIX)) {
            continue;
        }
        let written = entry
            .metadata()
            .and_then(|m| m.modified())
            .map_err(unswept)?;
        if written < cutoff {
            std::fs::remove_file(entry.path()).map_err(unswept)?;
        }
    }
    Ok(())
}

fn open_in_browser(
    target: &str,
    unhandled: impl Fn(std::io::Error) -> kernel::AxError,
) -> Result<(), kernel::AxError> {
    handler(target)?
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|_| ())
        .map_err(unhandled)
}

#[cfg(target_os = "windows")]
fn handler(target: &str) -> Result<std::process::Command, kernel::AxError> {
    let mut command = child::command(crate::privacy::windows::system32("cmd.exe")?);

    command.args(["/C", "start", "", target]);
    Ok(command)
}

#[cfg(target_os = "macos")]
fn handler(target: &str) -> Result<std::process::Command, kernel::AxError> {
    let mut command = child::command("open");
    command.arg(target);
    Ok(command)
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn handler(target: &str) -> Result<std::process::Command, kernel::AxError> {
    let mut command = child::command("xdg-open");
    command.arg(target);
    Ok(command)
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::wildcard_enum_match_arm,
    clippy::let_underscore_must_use,
    clippy::let_underscore_untyped,
    reason = "test code"
)]
mod tests {
    use super::{FirstScreen, ask};
    use std::path::PathBuf;

    fn screen(answer: &str) -> (FirstScreen, String) {
        let city = PathBuf::from("/tmp/here/city");
        let mut input = answer.as_bytes();
        let mut out: Vec<u8> = Vec::new();
        let outcome = ask(&city, &mut input, &mut out).unwrap();
        (outcome, String::from_utf8(out).unwrap())
    }

    #[test]
    fn the_path_is_on_the_screen_before_the_question_is_answered() {
        let (_, drawn) = screen("\n");
        assert!(drawn.contains("/tmp/here/city"), "drawn: {drawn}");
    }

    #[test]
    fn enter_starts_the_city_at_the_path_the_screen_showed() {
        let (outcome, _) = screen("\n");
        match outcome {
            FirstScreen::Start(path) => assert_eq!(path, PathBuf::from("/tmp/here/city")),
            _ => panic!("Enter starts the city"),
        }
    }

    #[test]
    fn a_pasted_path_is_a_folder_to_use_rather_than_a_place_to_start() {
        let (outcome, drawn) = screen("/tmp/already/work\n");
        match outcome {
            FirstScreen::Use(path) => assert_eq!(path, PathBuf::from("/tmp/already/work")),
            _ => panic!("a path is a folder to use"),
        }
        assert!(
            drawn.contains("your files stay as they are"),
            "the screen has to say what will happen to their work: {drawn}"
        );
    }

    #[test]
    fn a_pasted_path_keeps_its_spaces_and_loses_its_quotes() {
        let (outcome, _) = screen("\"/tmp/already/my work\"\n");
        match outcome {
            FirstScreen::Use(path) => assert_eq!(path, PathBuf::from("/tmp/already/my work")),
            _ => panic!("a quoted path is still a path"),
        }
    }

    #[test]
    fn q_quits_without_creating_anything() {
        let (outcome, _) = screen("q\n");
        assert!(matches!(outcome, FirstScreen::Quit));
    }

    #[test]
    fn end_of_input_quits_rather_than_acting_on_silence() {
        let (outcome, _) = screen("");
        assert!(matches!(outcome, FirstScreen::Quit));
    }
}
