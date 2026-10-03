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
mod case;
mod pages;

use std::path::Path;

use camera::{Camera, Source};
use case::Took;
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
    let strays = camera.close()?;
    let (shots, retried) = taken?;
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
        "{} pictures of {} pages in {OUT}; the index is {OUT}/index.md; each engine's stderr \
         is in {OUT}/engine\n{}",
        shots.len(),
        routes.len(),
        retried
            .iter()
            .map(|(case, why)| format!("retried once: {case}, after {why}\n"))
            .chain(
                strays
                    .iter()
                    .map(|why| format!("helper outlived its engine: {why}\n")),
            )
            .collect::<String>()
    ))
}

/// Every picture of every route, in route order, then window, then fold,
/// and each case that took a second try with why its first failed.
fn take(camera: &Camera<'_>, routes: &[String], out: &Path) -> Result<Taken, XtaskError> {
    let mut shots = Vec::new();
    let mut retried = Vec::new();
    let mut note = |case: String, took: Took| match took {
        Took::First => {}
        Took::Retried(why) => retried.push((case, why)),
    };
    for route in routes {
        for frame in FRAMES {
            let (folds, took) = camera.folds(route, frame)?;
            note(format!("{route} at {} (measuring)", frame.width), took);
            for shot in pages::shots_of(route, frame, &folds) {
                let took = camera.shoot(&shot, &out.join(shot.file()))?;
                note(shot.file(), took);
                shots.push(shot);
            }
        }
    }
    Ok((shots, retried))
}

/// The pictures one run took, and the cases it ran twice with the reason
/// the first try failed.
type Taken = (Vec<Shot>, Vec<(String, String)>);

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
