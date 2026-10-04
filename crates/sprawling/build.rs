// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Single embed point: gzips the web client bundle into OUT_DIR and
//! generates `client_embed.rs`, the file table main.rs includes; writes
//! the dependency list the lockfile names, and the toolchain pins the
//! doctor reports, beside it. Windows targets also link product resources
//! derived from Cargo's package metadata (`crates/sprawling/Spec.lean` §8-157).
//!
//! The bundle is whatever `just build-web` left at [`BUNDLE_DIR`] in this
//! package's directory: the client's own `index.html` and the hashed
//! assets Vite wrote beside it. There is no
//! second shell in this repository, because the one the client builds is
//! the one the product serves.
//!
//! **Every path here works in three trees**: this checkout, the
//! directory `cargo package` verifies its archive in, and the registry
//! directory `cargo install` unpacks it into (`crates/sprawling/Spec.lean` §8-157).
//! So nothing is found by counting parent directories: the bundle is
//! inside this package, the lockfile is found by cargo's own rule, and
//! a pin file outside the package is read when it is there and left
//! unpinned when it is not.
//!
//! When the bundle is absent the binary still builds - it carries a
//! placeholder page, `CLIENT_COMPLETE` is false, and a warning says so -
//! because `just check` must not demand a JavaScript toolchain. The
//! release gate (`xtask budget`) refuses a release binary whose file
//! table has no client assets, so an incomplete client cannot ship
//! silently.
//!
//! Gzip is deterministic here: fixed compression level, zeroed mtime.
//! The same source bytes embed identically on every build, which the
//! reproducible-build fixture (`cargo xtask repro`) relies on.

use std::io::Write as _;
use std::path::{Path, PathBuf};

/// Where the built client lands: a path relative to this package's
/// directory, in `/`-separated segments.
///
/// **This declaration is the one home of that location.** It is the whole
/// path rather than a name each reader places, because the parent
/// directory spelled separately in each reader is how the bundler and this
/// file came to disagree: the bundle is bun's output, not cargo's, so it
/// stays here whatever `CARGO_TARGET_DIR` says. It is inside this package
/// because a crates.io archive carries this package's directory and
/// nothing else, and this is the only reader that has to work there and
/// in the published tree, neither of which carries `tools/xtask/`; `xtask`
/// reads this constant out of this file, and its artifact gate refuses a
/// client build script or a justfile that spells the path differently.
const BUNDLE_DIR: &str = "web-dist";

/// The files that pin the toolchains this code is developed with, each
/// relative to the checkout's root and named by the constant `pins.rs`
/// gives it. None is in this package, so a build from the crates.io
/// archive finds none of them, and the doctor reports those tools as
/// unpinned (`crates/sprawling/Spec.lean` §8-157).
const PINS: [(&str, &str); 3] = [
    ("RUST_TOOLCHAIN_FILE", "rust-toolchain.toml"),
    ("LEAN_TOOLCHAIN_FILE", "lean-toolchain"),
    ("ZIG_VERSION_FILE", "crates/desktop/ffi/zig-version"),
];

/// What a binary built without the client serves.
///
/// It says which command was not run rather than showing an empty page,
/// because the person meeting it is a contributor who built from source
/// and the answer they need is one line long.
const PLACEHOLDER: &str = r#"<!DOCTYPE html>
<html lang="en"><head><meta charset="utf-8"><title>sprawling</title></head>
<body><p>This binary was built without the client bundle. Run <code>just build-web</code> and rebuild.</p></body></html>
"#;

fn main() {
    // Which release this binary is, set by `release.yml` and by nothing
    // else. Declared here rather than only read through `option_env!`,
    // because cargo otherwise reuses a binary compiled under the previous
    // tag and every copy of it would name the wrong release.
    println!("cargo::rerun-if-env-changed=SPRAWLING_RELEASE_TAG");
    if let Err(msg) = embed() {
        // cargo >= 1.84: `cargo::error` fails the build loudly instead of
        // leaving a stale or missing asset for include_bytes! to trip on.
        println!("cargo::error=binary resource generation failed: {msg}");
    }
}

fn embed() -> Result<(), String> {
    if std::env::var("CARGO_CFG_TARGET_OS")
        .map_err(|err| format!("read resource target OS: {err}"))?
        == "windows"
    {
        winresource::WindowsResource::new()
            .compile()
            .map_err(|err| {
                format!(
                    "compile Windows product resources: {err}; check the target resource toolchain"
                )
            })?;
    }
    let manifest = std::env::var("CARGO_MANIFEST_DIR").map_err(|e| e.to_string())?;
    let out_dir = std::env::var("OUT_DIR").map_err(|e| e.to_string())?;
    let dist = BUNDLE_DIR
        .split('/')
        .fold(PathBuf::from(&manifest), |at, segment| at.join(segment));
    // Emitted even when absent: the first `just build-web` after a
    // placeholder build must trigger a re-embed, not wait for luck.
    println!("cargo::rerun-if-changed={}", dist.display());

    // A bundle is complete when the page exists and something was built
    // beside it. Both halves are asked for: Vite writes `index.html`
    // first, so a build interrupted between the two leaves a page whose
    // every script tag points at nothing.
    let mut files: Vec<(String, PathBuf)> = Vec::new();
    let assets = dist.join("assets");
    let built = dist.join("index.html").is_file()
        && std::fs::read_dir(&assets).is_ok_and(|mut held| held.next().is_some());
    let complete = built;
    if built {
        collect(&dist, &dist, &mut files)?;
    } else {
        println!(
            "cargo::warning=client bundle not found in {}; the binary will carry a placeholder \
             page only - run `just build-web`, then rebuild",
            dist.display()
        );
        let placeholder = PathBuf::from(&out_dir).join("placeholder.html");
        std::fs::write(&placeholder, PLACEHOLDER).map_err(|err| err.to_string())?;
        files.push(("index.html".to_owned(), placeholder));
    }
    files.sort_by(|a, b| a.0.cmp(&b.0));

    let web_out = PathBuf::from(&out_dir).join("web");
    let mut table = String::new();
    for (rel, source) in &files {
        println!("cargo::rerun-if-changed={}", source.display());
        let bytes = std::fs::read(source).map_err(|e| format!("{}: {e}", source.display()))?;
        let gz_path = web_out.join(format!("{}.gz", rel.replace('/', "_")));
        if let Some(parent) = gz_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let gz = gzip(&bytes)?;
        std::fs::write(&gz_path, gz).map_err(|e| format!("{}: {e}", gz_path.display()))?;
        table.push_str(&format!(
            "    wire::EmbeddedFile {{ path: {rel:?}, gz: include_bytes!({:?}) }},\n",
            gz_path.display().to_string().replace('\\', "/")
        ));
    }

    let root = checkout_root(Path::new(&manifest))?;
    embed_dependency_list(&root, &out_dir)?;
    embed_pins(&root, &out_dir)?;

    let generated = format!(
        "// Generated by build.rs; the authority is the file table it read.\n\
         /// Whether the wasm client is inside this binary, or only the shell.\n\
         pub const CLIENT_COMPLETE: bool = {complete};\n\
         /// Where `just build-web` writes the client, relative to the sprawling package.\n\
         pub const CLIENT_BUNDLE_DIR: &str = {BUNDLE_DIR:?};\n\
         /// Every file the browser may request, gzipped at build time.\n\
         pub static CLIENT_FILES: &[wire::EmbeddedFile] = &[\n{table}];\n"
    );
    std::fs::write(PathBuf::from(&out_dir).join("client_embed.rs"), generated)
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// The first directory from this package's upward that holds a
/// `Cargo.lock`, which is where cargo writes it: the workspace root in a
/// checkout, and this package's own directory in an archive, because
/// `cargo package` puts the lockfile at the archive's root.
fn checkout_root(manifest: &Path) -> Result<PathBuf, String> {
    manifest
        .ancestors()
        .find(|dir| dir.join("Cargo.lock").is_file())
        .map(Path::to_path_buf)
        .ok_or_else(|| {
            format!(
                "no Cargo.lock in {} or any directory above it; a build that cannot name \
                 what it is made of is refused",
                manifest.display()
            )
        })
}

/// The embedded dependency list (release item: the binary can name what
/// it is made of). Parsed from Cargo.lock with a ten-line reader rather
/// than a toml dependency: the lockfile's [[package]] blocks are stable,
/// machine-written lines.
fn embed_dependency_list(root: &Path, out_dir: &str) -> Result<(), String> {
    let lock = root.join("Cargo.lock");
    println!("cargo::rerun-if-changed={}", lock.display());
    let text = std::fs::read_to_string(&lock).map_err(|e| format!("{}: {e}", lock.display()))?;
    let mut deps = Vec::new();
    let mut name: Option<String> = None;
    for line in text.lines() {
        if line == "[[package]]" {
            name = None;
        } else if let Some(raw) = line.strip_prefix("name = \"") {
            name = raw.strip_suffix('"').map(str::to_owned);
        } else if let Some(raw) = line.strip_prefix("version = \"")
            && let (Some(package), Some(version)) = (name.take(), raw.strip_suffix('"'))
        {
            deps.push(format!("{package} {version}"));
        }
    }
    deps.sort();
    std::fs::write(
        PathBuf::from(out_dir).join("deps.txt"),
        format!("{}\n", deps.join("\n")),
    )
    .map_err(|e| e.to_string())
}

/// `pins.rs`: one constant per row of [`PINS`], holding that file's whole
/// text, or nothing when the file is absent. Only absence reads as
/// nothing; a pin file that exists and cannot be read fails the build.
fn embed_pins(root: &Path, out_dir: &str) -> Result<(), String> {
    let mut generated = String::from(
        "// Generated by build.rs from the pin files beside the lockfile; an empty string is a \
         file this build did not find.\n",
    );
    for (name, rel) in PINS {
        let path = rel
            .split('/')
            .fold(root.to_path_buf(), |at, segment| at.join(segment));
        println!("cargo::rerun-if-changed={}", path.display());
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => String::new(),
            Err(err) => return Err(format!("{}: {err}", path.display())),
        };
        generated.push_str(&format!("pub(crate) const {name}: &str = {text:?};\n"));
    }
    std::fs::write(PathBuf::from(out_dir).join("pins.rs"), generated).map_err(|e| e.to_string())
}

/// Walks `dist` recursively, recording each file under its request path.
fn collect(root: &Path, dir: &Path, files: &mut Vec<(String, PathBuf)>) -> Result<(), String> {
    let entries = std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.is_dir() {
            collect(root, &path, files)?;
            continue;
        }
        let rel = path
            .strip_prefix(root)
            .map_err(|e| e.to_string())?
            .to_string_lossy()
            .replace('\\', "/");
        files.push((rel, path));
    }
    Ok(())
}

/// Deterministic gzip: fixed level, zeroed mtime.
fn gzip(bytes: &[u8]) -> Result<Vec<u8>, String> {
    let mut encoder = flate2::GzBuilder::new()
        .mtime(0)
        .write(Vec::new(), flate2::Compression::best());
    encoder.write_all(bytes).map_err(|e| e.to_string())?;
    encoder.finish().map_err(|e| e.to_string())
}
