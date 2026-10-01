// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! `cargo xtask shots`: every page of the client, each fold of it, in
//! two windows and two lightings, as PNG files and one index a person
//! reads them through (tools/xtask/Spec.lean §8-44 and xtask D10).
//!
//! **This is not a gate.** It asserts nothing and compares no two
//! pictures; the render gate judges properties, and a person judges
//! these. It is a command of its own so that the gate roster holds only
//! steps that can go red.

mod camera;
mod pages;

use std::path::Path;

use camera::{Camera, Source};
use pages::{FRAMES, Shot};

use crate::report::XtaskError;
use crate::walk;

/// Where the pictures and the index are written.
const OUT: &str = "target/shots";

/// The client's route table, whose `BARE` keys are the pages.
const ROUTES: &str = "client/src/core/route.ts";

/// The flag that photographs a served city instead of the bundle.
const ORIGIN: &str = "--origin";

pub(crate) fn run(root: &Path, args: &[String]) -> Result<String, XtaskError> {
    let origin = origin_in(args)?;
    let bundle = crate::bundle::dist(root)?;
    let source = match origin.as_deref() {
        Some(origin) => Source::Origin(origin),
        None if bundle.join("index.html").is_file() => Source::Bundle(&bundle),
        None => {
            return Err(XtaskError::Doc {
                file: walk::rel(root, &bundle.join("index.html")),
                msg: "photograph the client: the bundle is not built (no-bundle); run \
                      `just build-web`, or name a served city with `--origin <url>`"
                    .to_owned(),
            });
        }
    };
    let browser = crate::render::engine::browser().ok_or_else(|| XtaskError::Doc {
        file: "SPRAWLING_BROWSER".to_owned(),
        msg: "photograph the client: no headless browser was found (no-browser); install a \
              Chromium-family browser, or point `SPRAWLING_BROWSER` at one"
            .to_owned(),
    })?;
    let routes = pages::routes_in(&walk::read_text(&root.join(ROUTES))?);
    if routes.is_empty() {
        return Err(XtaskError::Doc {
            file: ROUTES.to_owned(),
            msg: "photograph the client: no route was read from the `BARE` table (no-routes); \
                  the table this command reads its pages from has moved or changed shape, so \
                  point `pages::routes_in` at where it lives now"
                .to_owned(),
        });
    }
    let out = root.join(OUT);
    std::fs::create_dir_all(&out).map_err(|source| XtaskError::Io {
        path: OUT.to_owned(),
        source,
    })?;
    let camera = Camera::open(&browser, &out, source)?;
    let taken = take(&camera, &routes, &out);
    camera.close()?;
    let shots = taken?;
    let index = out.join("index.md");
    std::fs::write(&index, pages::index(&shots)).map_err(|source| XtaskError::Io {
        path: walk::rel(root, &index),
        source,
    })?;
    let absent = pages::missing(&out, &shots);
    if !absent.is_empty() {
        return Err(XtaskError::Cmd {
            cmd: "cargo xtask shots".to_owned(),
            msg: format!(
                "the engine exited without writing {} of {} pictures: {}; run the command again, \
                 and if the same ones are missing, open that route in the browser by hand",
                absent.len(),
                shots.len(),
                absent.join(", ")
            ),
        });
    }
    Ok(format!(
        "{} pictures of {} pages in {OUT}; the index is {OUT}/index.md\n",
        shots.len(),
        routes.len()
    ))
}

/// Every picture of every route, in route order, then window, then fold.
fn take(camera: &Camera<'_>, routes: &[String], out: &Path) -> Result<Vec<Shot>, XtaskError> {
    let mut shots = Vec::new();
    for route in routes {
        for frame in FRAMES {
            for shot in pages::shots_of(route, frame, &camera.folds(route, frame)?) {
                camera.shoot(&shot, &out.join(shot.file()))?;
                shots.push(shot);
            }
        }
    }
    Ok(shots)
}

/// The served city named after `--origin`, if one is; any other
/// argument is refused rather than ignored.
fn origin_in(args: &[String]) -> Result<Option<String>, XtaskError> {
    match args {
        [] => Ok(None),
        [flag, origin] if flag == ORIGIN && origin.starts_with("http") => Ok(Some(origin.clone())),
        _ => Err(XtaskError::Doc {
            file: "cargo xtask shots".to_owned(),
            msg: format!(
                "read the arguments `{}` (usage); run `cargo xtask shots`, or \
                 `cargo xtask shots {ORIGIN} http://127.0.0.1:8787` for a served city",
                args.join(" ")
            ),
        }),
    }
}
