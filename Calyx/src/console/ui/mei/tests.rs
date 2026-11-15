// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::sync::Arc;

use super::*;

fn clock() -> Clock {
    Arc::new(|| Ok(TimeMs::new(0)))
}

#[test]
fn replay_is_drained_before_the_writer_waits_for_a_key() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("trace.jsonl");
    std::fs::write(
        &path,
        concat!(
            "{\"role\":\"assistant\",\"content\":\"first\"}\n",
            "{\"role\":\"assistant\",\"content\":\"second\"}\n",
            "{\"role\":\"assistant\",\"content\":\"third\"}\n"
        ),
    )
    .unwrap();
    let replay = BufReader::new(std::fs::File::open(path).unwrap());
    let (to, _from) = std::sync::mpsc::sync_channel(UI_DEPTH);
    let mut ui = Ui::new(to, Some(replay), clock());
    ui.screen.cut();
    ui.advance_replay();
    assert!(ui.replay_done);
}

#[test]
fn replay_advances_in_bounded_batches_so_escape_stays_live() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("trace.jsonl");
    let mut text = String::new();
    for index in 0..REPLAY_BATCH + 4 {
        text.push_str(&format!(
            "{{\"role\":\"assistant\",\"content\":\"line {index}\"}}\n"
        ));
    }
    std::fs::write(&path, text).unwrap();
    let replay = BufReader::new(std::fs::File::open(path).unwrap());
    let (to, _from) = std::sync::mpsc::sync_channel(UI_DEPTH);
    let mut ui = Ui::new(to, Some(replay), clock());
    ui.screen.cut();
    ui.advance_replay();
    assert!(!ui.replay_done, "one batch must not pin the whole trace");
    ui.advance_replay();
    assert!(ui.replay_done);
}

#[test]
fn shift_enter_inserts_a_newline_instead_of_submitting() {
    let (to, _from) = std::sync::mpsc::sync_channel(UI_DEPTH);
    let mut ui = Ui::new(to, None, clock());
    ui.screen.cut();
    ui.key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));
    ui.key(KeyEvent::new(KeyCode::Enter, KeyModifiers::SHIFT));
    assert_eq!(ui.editor.text(), "x\n");
    assert!(ui.process.is_none());
}

fn submit(ui: &mut Ui, text: &str) {
    for character in text.chars() {
        ui.key(KeyEvent::new(KeyCode::Char(character), KeyModifiers::NONE));
    }
    ui.key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
}

#[test]
fn new_and_resume_select_explicit_jsonl_files_without_rewriting_old_ones() {
    let dir = tempfile::tempdir().unwrap();
    let (to, _from) = std::sync::mpsc::sync_channel(UI_DEPTH);
    let mut ui = Ui::new(to, None, clock());
    ui.screen.cut();
    ui.session_dir = dir.path().to_path_buf();
    submit(&mut ui, "/new");
    let first = ui.session.as_ref().unwrap().path().to_path_buf();
    ui.session
        .as_mut()
        .unwrap()
        .append(&[
            "{\"role\":\"user\",\"content\":\"first\"}".to_owned(),
            "{\"role\":\"assistant\",\"content\":\"done\",\"stop_reason\":\"end_turn\"}"
                .to_owned(),
        ])
        .unwrap();
    let before = std::fs::read(&first).unwrap();
    submit(&mut ui, "/new");
    let second = ui.session.as_ref().unwrap().path().to_path_buf();
    assert_ne!(first, second);
    assert_eq!(std::fs::read(&first).unwrap(), before);

    submit(&mut ui, &format!("/resume {}", first.display()));
    assert_eq!(ui.session.as_ref().unwrap().path(), first.as_path());
    assert_eq!(ui.session.as_ref().unwrap().history().len(), 2);
    assert_eq!(std::fs::read(&second).unwrap(), b"");
}

#[test]
fn a_lost_terminal_never_saves_an_incompletely_drained_segment() {
    let dir = tempfile::tempdir().unwrap();
    let (to, from) = std::sync::mpsc::sync_channel(UI_DEPTH);
    let mut ui = Ui::new(to.clone(), None, clock());
    ui.screen.cut();
    ui.session_dir = dir.path().to_path_buf();
    submit(&mut ui, "/new");
    let path = ui.session.as_ref().unwrap().path().to_path_buf();
    ui.process = Some(MeiProcess::finished_for_test());
    assert!(
        ui.show(Show::Mei(MeiEvent::Line(
            "{\"role\":\"user\",\"content\":\"first\"}".to_owned()
        )))
        .is_none()
    );
    assert!(ui.show(Show::Lost).is_none());
    to.send(Show::Mei(MeiEvent::Line(
        "{\"role\":\"assistant\",\"content\":\"done\",\"stop_reason\":\"end_turn\"}".to_owned(),
    )))
    .unwrap();
    to.send(Show::Mei(MeiEvent::Finished {
        result: Ok("done".to_owned()),
        cancelled: false,
    }))
    .unwrap();
    let _code = ui.give_back(&from, ExitCode::FAILURE);
    assert_eq!(std::fs::read(path).unwrap(), b"");
    assert!(ui.pending.is_empty());
}

#[test]
fn only_a_successful_segment_is_appended_to_the_current_jsonl() {
    let dir = tempfile::tempdir().unwrap();
    let (to, _from) = std::sync::mpsc::sync_channel(UI_DEPTH);
    let mut ui = Ui::new(to, None, clock());
    ui.screen.cut();
    ui.session_dir = dir.path().to_path_buf();
    submit(&mut ui, "/new");
    ui.mei(MeiEvent::Line("ordinary diagnostic".to_owned()));
    assert!(ui.pending.is_empty());
    let path = ui.session.as_ref().unwrap().path().to_path_buf();
    let segment = vec![
        "{\"role\":\"user\",\"content\":\"first\"}".to_owned(),
        "{\"role\":\"assistant\",\"content\":\"done\",\"stop_reason\":\"end_turn\"}".to_owned(),
    ];
    ui.pending = segment.clone();
    ui.mei(MeiEvent::Finished {
        result: Ok("done".to_owned()),
        cancelled: false,
    });
    assert_eq!(std::fs::read_to_string(&path).unwrap().lines().count(), 2);

    ui.pending = vec!["{\"role\":\"user\",\"content\":\"failed\"}".to_owned()];
    ui.mei(MeiEvent::Finished {
        result: Err("failed".to_owned()),
        cancelled: false,
    });
    assert_eq!(std::fs::read_to_string(&path).unwrap().lines().count(), 2);

    ui.pending = vec!["{\"role\":\"user\",\"content\":\"cancelled\"}".to_owned()];
    ui.mei(MeiEvent::Finished {
        result: Err("cancelled".to_owned()),
        cancelled: true,
    });
    assert_eq!(std::fs::read_to_string(&path).unwrap().lines().count(), 2);
    assert_eq!(ui.session.as_ref().unwrap().history(), segment);
}
