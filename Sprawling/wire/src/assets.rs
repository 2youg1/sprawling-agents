// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{AxCode, AxError};

#[derive(Debug, Clone, Copy)]
pub struct EmbeddedFile {

    pub path: &'static str,

    pub gz: &'static [u8],
}

#[derive(Debug)]
pub enum ClientAssets {

    Embedded(&'static [EmbeddedFile]),

    Disk(std::path::PathBuf),
}

#[derive(Debug)]
pub enum AssetReply {
    Found {
        bytes: Vec<u8>,
        content_type: &'static str,

        gzipped: bool,
    },
    Miss(AxError),
}

impl ClientAssets {

    #[must_use]
    pub fn lookup(&self, request_path: &str) -> AssetReply {
        let Some(rel) = sanitize_asset_path(request_path) else {
            return AssetReply::Miss(
                AxError::failure(
                    AxCode::InvalidArgs,
                    "serve a client asset",
                    format!("the path {request_path} steps outside the client bundle"),
                )
                .with_recovery("request a bundle-relative path such as /web.js"),
            );
        };
        match self {
            ClientAssets::Embedded(files) => {
                for file in *files {
                    if file.path == rel {
                        return AssetReply::Found {
                            bytes: file.gz.to_vec(),
                            content_type: content_type_of(&rel),
                            gzipped: true,
                        };
                    }
                }
                AssetReply::Miss(missing_asset(&rel, "this binary"))
            }
            ClientAssets::Disk(root) => {
                let full = root.join(&rel);
                match std::fs::read(&full) {
                    Ok(bytes) => AssetReply::Found {
                        bytes,
                        content_type: content_type_of(&rel),
                        gzipped: false,
                    },
                    Err(_) => AssetReply::Miss(missing_asset(&rel, "the --web-dir directory")),
                }
            }
        }
    }
}

fn missing_asset(rel: &str, source: &str) -> AxError {
    AxError::failure(
        AxCode::InvalidArgs,
        "serve a client asset",
        format!("{source} does not carry {rel}"),
    )
    .with_recovery("rebuild the client (`just build-web`), then rebuild or restart the server")
}

fn sanitize_asset_path(raw: &str) -> Option<String> {
    let trimmed = raw.trim_start_matches('/');
    if trimmed.is_empty() {
        return Some("index.html".to_owned());
    }
    if trimmed.contains('\\') || trimmed.contains(':') {
        return None;
    }
    let mut segments = Vec::new();
    for segment in trimmed.split('/') {
        if segment.is_empty() || segment.starts_with('.') {
            return None;
        }
        segments.push(segment);
    }
    Some(segments.join("/"))
}

fn content_type_of(rel: &str) -> &'static str {
    let suffix = rel.rsplit('.').next().unwrap_or_default();
    match suffix {
        "html" => "text/html; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "wasm" => "application/wasm",
        "css" => "text/css; charset=utf-8",
        "json" | "map" => "application/json",
        "svg" => "image/svg+xml",
        _ => "application/octet-stream",
    }
}
#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests {
    use super::*;

    const BUNDLE: &[EmbeddedFile] = &[
        EmbeddedFile {
            path: "index.html",
            gz: b"gzipped-html",
        },
        EmbeddedFile {
            path: "web.js",
            gz: b"gzipped-js",
        },
        EmbeddedFile {
            path: "web_bg.wasm",
            gz: b"gzipped-wasm",
        },
        EmbeddedFile {
            path: "snippets/dioxus-abc/inline0.js",
            gz: b"gzipped-snippet",
        },
    ];

    #[test]
    fn the_embedded_bundle_answers_the_paths_the_page_asks_for() {
        let assets = ClientAssets::Embedded(BUNDLE);
        for (asked, want_type, want_bytes) in [
            ("", "text/html; charset=utf-8", b"gzipped-html".as_slice()),
            ("/", "text/html; charset=utf-8", b"gzipped-html".as_slice()),
            (
                "web.js",
                "text/javascript; charset=utf-8",
                b"gzipped-js".as_slice(),
            ),
            (
                "/web_bg.wasm",
                "application/wasm",
                b"gzipped-wasm".as_slice(),
            ),
            (
                "snippets/dioxus-abc/inline0.js",
                "text/javascript; charset=utf-8",
                b"gzipped-snippet".as_slice(),
            ),
        ] {
            let AssetReply::Found {
                bytes,
                content_type,
                gzipped,
            } = assets.lookup(asked)
            else {
                panic!("{asked} must be found");
            };
            assert_eq!(bytes, want_bytes, "{asked}");
            assert_eq!(content_type, want_type, "{asked}");
            assert!(gzipped, "embedded files travel gzipped");
        }
    }

    #[test]
    fn a_path_outside_the_bundle_is_a_miss_that_names_the_path() {
        let assets = ClientAssets::Embedded(BUNDLE);
        for hostile in [
            "../Cargo.toml",
            "a/../../secret",
            "a//b.js",
            "C:/windows/system32",
            "snippets\\x\\y.js",
            ".git/config",
        ] {
            let AssetReply::Miss(err) = assets.lookup(hostile) else {
                panic!("{hostile} must miss");
            };
            assert!(!err.recovery().is_empty(), "a miss says what to do next");
        }
    }

    #[test]
    fn an_unknown_file_misses_and_the_recovery_names_the_rebuild() {
        let assets = ClientAssets::Embedded(BUNDLE);
        let AssetReply::Miss(err) = assets.lookup("missing.js") else {
            panic!("an absent file is a miss");
        };
        assert!(err.recovery().contains("just build-web"));
    }

    #[test]
    fn the_disk_arm_reads_per_request_and_never_claims_gzip() {
        let dir = std::env::temp_dir().join(format!("sprawl-assets-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("web.js"), b"fresh from disk").unwrap();
        let assets = ClientAssets::Disk(dir.clone());
        let AssetReply::Found {
            bytes,
            content_type,
            gzipped,
        } = assets.lookup("/web.js")
        else {
            panic!("the file exists");
        };
        assert_eq!(bytes, b"fresh from disk");
        assert_eq!(content_type, "text/javascript; charset=utf-8");
        assert!(!gzipped, "disk bytes are identity-encoded");
        std::fs::write(dir.join("web.js"), b"edited").unwrap();
        let AssetReply::Found { bytes, .. } = assets.lookup("web.js") else {
            panic!("still there");
        };
        assert_eq!(
            bytes, b"edited",
            "a refresh sees the edit without a restart"
        );
        std::fs::remove_dir_all(&dir).ok();
    }
}
