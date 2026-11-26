// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

const LIBRARY: &str = "sprawling_calyx_shell";
const FACE: &str = "calyx";
const CONSOLE_LEAF: &str = "DEP_SPRAWLING_CONSOLE_LEAF_ZIG_SOURCE";

fn main() -> ExitCode {
    match build() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn build() -> Result<(), String> {
    println!("cargo::rerun-if-env-changed=DOCS_RS");
    if std::env::var_os("DOCS_RS").is_some() {
        return Ok(());
    }
    println!("cargo::rerun-if-changed=build.zig");
    println!("cargo::rerun-if-changed=shell");
    let here = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").map_err(|e| e.to_string())?);
    let leaf = leaf_source(&here)?;
    println!("cargo::rerun-if-changed={leaf}");

    let out = PathBuf::from(std::env::var("OUT_DIR").map_err(|e| e.to_string())?);
    let zig_target = zig_target()?;
    built(&here, &out, &zig_target, &leaf)?;
    println!(
        "cargo::rustc-link-search=native={}",
        out.join("lib").display()
    );
    println!("cargo::rustc-link-lib=static={LIBRARY}");
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let target_env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    if (target_os == "linux" || target_os == "macos") && target_env != "musl" {
        println!("cargo::rustc-link-lib=dylib=c");
        println!("cargo::rustc-link-lib=dylib=pthread");
        println!("cargo::rustc-link-lib=dylib=m");
    }
    install_face(&out, &target_os)?;
    Ok(())
}

fn leaf_source(here: &Path) -> Result<String, String> {
    std::env::var(CONSOLE_LEAF).map_err(|source| {
        let path = here.join("Cargo.toml").display().to_string();
        let value = match source {
            std::env::VarError::NotPresent => "<missing>".to_owned(),
            std::env::VarError::NotUnicode(raw) => format!("{raw:?}"),
        };
        let target = std::env::var_os("TARGET").map_or_else(
            || "<target>".to_owned(),
            |value| value.to_string_lossy().into_owned(),
        );
        format!(
            r#"```toml
code = "E_CALYX_ZIG_SOURCE"
action = "build the Calyx shell"
subject = "sprawling-calyx"
path = {path:?}
value = {value:?}
rule = "Cargo supplies {CONSOLE_LEAF} from sprawling-console-ffi's zig_source metadata"
fix = "Remove the [target.{target}.sprawling_console_leaf] table from Cargo configuration and rerun cargo build --locked -p sprawling-calyx"
```"#
        )
    })
}

fn zig_target() -> Result<String, String> {
    let arch = match std::env::var("CARGO_CFG_TARGET_ARCH")
        .map_err(|err| format!("cargo set no CARGO_CFG_TARGET_ARCH: {err}"))?
        .as_str()
    {
        "x86_64" => "x86_64",
        "aarch64" => "aarch64",
        other => {
            return Err(format!(
                "the Calyx shell is built for x86_64 and aarch64, not {other}"
            ));
        }
    };
    let os = std::env::var("CARGO_CFG_TARGET_OS")
        .map_err(|err| format!("cargo set no CARGO_CFG_TARGET_OS: {err}"))?;
    let env = std::env::var("CARGO_CFG_TARGET_ENV")
        .map_err(|err| format!("cargo set no CARGO_CFG_TARGET_ENV: {err}"))?;
    match (os.as_str(), env.as_str()) {
        ("windows", "gnu") => Ok(format!("{arch}-windows-gnu")),
        ("windows", _) => Ok(format!("{arch}-windows-msvc")),
        ("linux", "musl") => Ok(format!("{arch}-linux-musl")),
        ("linux", _) => Ok(format!("{arch}-linux-gnu")),
        ("macos", _) => Ok(format!("{arch}-macos")),
        (other, _) => Err(format!(
            "the Calyx shell is built for Windows, Linux and macOS, not {other}"
        )),
    }
}

#[expect(clippy::disallowed_methods, reason = "build script (child D4)")]
fn built(here: &Path, out: &Path, zig_target: &str, leaf: &str) -> Result<(), String> {
    let status = Command::new("zig")
        .current_dir(here)
        .args([
            "build",
            "-Doptimize=safe",
            &format!("-Dtarget={zig_target}"),
            &format!("-Dconsole-leaf={leaf}"),
            "--cache-dir",
        ])
        .arg(out.join("zig-cache"))
        .arg("--prefix")
        .arg(out)
        .status()
        .map_err(|err| format!("zig did not start: {err}"))?;
    if status.success() {
        let lib = out.join("lib");
        if lib.join(format!("lib{LIBRARY}.a")).exists()
            || lib.join(format!("{LIBRARY}.lib")).exists()
        {
            return Ok(());
        }
        return Err(format!(
            "zig build succeeded but {LIBRARY} was not installed under {}",
            lib.display()
        ));
    }
    Err(format!(
        "zig build refused the Calyx shell for {zig_target} (status {status})"
    ))
}

fn install_face(out: &Path, target_os: &str) -> Result<(), String> {
    let name = if target_os == "windows" {
        format!("{FACE}.exe")
    } else {
        FACE.to_owned()
    };
    let built = out.join("bin").join(&name);
    if !built.is_file() {
        return Err(format!(
            "zig build succeeded but `{name}` was not installed under {}",
            out.join("bin").display()
        ));
    }
    let profile = profile_dir(out)?;
    let dest = profile.join(&name);
    std::fs::copy(&built, &dest).map_err(|err| {
        format!(
            "copy Zig `{name}` into cargo target dir {}: {err}",
            dest.display()
        )
    })?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        let mut perms = std::fs::metadata(&dest)
            .map_err(|err| format!("stat {}: {err}", dest.display()))?
            .permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&dest, perms)
            .map_err(|err| format!("chmod {}: {err}", dest.display()))?;
    }
    println!("cargo::rerun-if-changed={}", built.display());
    Ok(())
}

fn profile_dir(out: &Path) -> Result<&Path, String> {
    out.ancestors()
        .find(|path| path.ends_with("build"))
        .and_then(Path::parent)
        .ok_or_else(|| {
            format!(
                "```toml\ncode = \"E_CALYX_BUILD_PROFILE\"\n\
                 action = \"place the Calyx executable\"\n\
                 subject = \"Cargo OUT_DIR\"\npath = {out:?}\n\
                 value = \"no enclosing build directory with a parent\"\n\
                 rule = \"Cargo places build-script output under the profile build directory\"\n\
                 fix = \"use the workspace Cargo configuration and run just dist-mei\"\n```"
            )
        })
}
