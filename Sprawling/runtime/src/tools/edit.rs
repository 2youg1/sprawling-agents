// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::borrow::Cow;
use std::path::{Path, PathBuf};

use kernel::{
    AxCode, AxError, CostTier, Effect, Payload, RenderIntent, Temporal, Tool, ToolCall, ToolDef,
    ToolMeta, ToolName, ToolOutcome,
};
use serde_json::{Map, Value};

use super::path_lock::PathLocks;
use super::stamp::{Admission, Intent, ReadStamps, Stamp, admission};

mod passage;

pub struct EditTool {
    city_root: PathBuf,

    writable: kernel::WriteDomain,
    desk: EditDesk,
    meta: ToolMeta,
}

#[derive(Clone)]
pub struct EditDesk {

    pub policy: crate::mode::PolicyReader,

    pub seen: ReadStamps,

    pub locks: PathLocks,
}

impl EditTool {

    pub const NAME: &'static str = "edit";

    const FILE_PATH: &'static str = "file_path";
    const OLD_STRING: &'static str = "old_string";
    const NEW_STRING: &'static str = "new_string";
    const REPLACE_ALL: &'static str = "replace_all";

    pub fn definition() -> Result<ToolDef, AxError> {
        Ok(ToolDef {
            name: ToolName::parse(Self::NAME)?,
            description: "Performs exact string replacements in a file you have read in \
                          this run: `old_string` must match exactly once, or set `replace_all` \
                          to change every occurrence. A file that changed after your read is \
                          refused and not overwritten, and the refusal shows the passage as it \
                          now reads. `old_string: \"\"` creates a file that does not exist."
                .to_owned(),
            input_schema: Payload::of(&serde_json::json!({
                "type": "object",
                "properties": {
                    Self::FILE_PATH: { "type": "string",
                        "description": "the file to modify, relative to the city root" },
                    Self::OLD_STRING: { "type": "string", "description":
                        "the exact text to replace; \"\" creates a file that does not exist" },
                    Self::NEW_STRING: { "type": "string",
                        "description": "the text to replace it with, or the whole file when creating" },
                    Self::REPLACE_ALL: { "type": "boolean",
                        "description": "replace every occurrence of old_string (default false)" },
                },
                "required": [Self::FILE_PATH, Self::OLD_STRING, Self::NEW_STRING],
            }))?,
        })
    }

    pub fn new(
        city_root: &Path,
        domain: kernel::Address,
        writable: kernel::WriteDomain,
        desk: EditDesk,
    ) -> Result<EditTool, AxError> {
        let defined = Self::definition()?;
        Ok(EditTool {
            city_root: city_root.to_path_buf(),
            writable,
            desk,
            meta: ToolMeta {
                name: defined.name,

                disclosure: defined.description,
                params: defined.input_schema,
                effect: Effect::Write { domain },
                cost_tier: CostTier::Light,
                timeout: None,
                render: RenderIntent::Diff {
                    locations: Vec::new(),
                },
                temporal: Temporal::Timeless,
            },
        })
    }
}

fn arg<'a>(args: &'a Map<String, Value>, key: &str) -> Result<&'a str, AxError> {
    args.get(key).and_then(Value::as_str).ok_or_else(|| {
        AxError::failure(
            AxCode::InvalidArgs,
            "edit file",
            format!("missing string argument `{key}`"),
        )
        .with_recovery(format!(
            "pass {}, {} and {} - all strings - and {} when every occurrence should change",
            EditTool::FILE_PATH,
            EditTool::OLD_STRING,
            EditTool::NEW_STRING,
            EditTool::REPLACE_ALL,
        ))
    })
}

fn replacing_all(args: &Map<String, Value>) -> Result<bool, AxError> {
    match args.get(EditTool::REPLACE_ALL) {
        None => Ok(false),
        Some(asked) => asked.as_bool().ok_or_else(|| {
            AxError::failure(
                AxCode::InvalidArgs,
                "edit file",
                format!("`{}` is not true or false", EditTool::REPLACE_ALL),
            )
            .with_recovery(format!(
                "pass `{}` as a boolean, or leave it out to replace one occurrence",
                EditTool::REPLACE_ALL
            ))
        }),
    }
}

impl Tool for EditTool {
    fn meta(&self) -> &ToolMeta {
        &self.meta
    }

    fn writes(&self, call: &ToolCall) -> kernel::Writes {
        match arg(call.args.as_map(), Self::FILE_PATH)
            .and_then(|asked| super::chosen_path::within_city(&self.city_root, asked, "edit file"))
            .and_then(|spelled| kernel::Address::parse(&spelled))
        {
            Ok(path) => kernel::Writes::Paths(vec![path]),
            Err(_) => kernel::Writes::Domain,
        }
    }

    fn invoke(&self, call: &ToolCall) -> Result<ToolOutcome, AxError> {
        if call.name != self.meta.name {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "edit file",
                format!("call routed to the wrong tool: {}", call.name.as_str()),
            )
            .with_recovery(format!(
                "call `{}`, the name this tool answers to",
                self.meta.name.as_str()
            )));
        }

        let policy = self.desk.policy.now();
        crate::mode::mode_admits(&policy, Self::NAME)?;
        let args = call.args.as_map();
        let spelled = super::chosen_path::within_city(
            &self.city_root,
            arg(args, Self::FILE_PATH)?,
            "edit file",
        )?;
        let rel: &str = &spelled;
        let old = arg(args, Self::OLD_STRING)?;
        let new = arg(args, Self::NEW_STRING)?;
        let every = replacing_all(args)?;
        if every && old.is_empty() {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "edit file",
                format!(
                    "`{}` with an empty `{}`",
                    Self::REPLACE_ALL,
                    Self::OLD_STRING
                ),
            )
            .with_recovery(format!(
                "leave `{}` out to create {rel}, or name the text to replace",
                Self::REPLACE_ALL
            )));
        }

        let target = kernel::Address::parse(rel).map_err(|err| {
            AxError::failure(
                AxCode::InvalidArgs,
                "edit file",
                format!("{rel}: {}", err.subject()),
            )
            .with_recovery("pass a city-relative path with no `..` and no leading slash")
        })?;
        match kernel::gate::domain(&self.writable, &target, &kernel::TaintSet::empty()) {
            kernel::GateOutcome::Allow => {}

            kernel::GateOutcome::Deny { refusal } => return Err(*refusal),
            kernel::GateOutcome::Ask { question } => return Err(*question),
        }

        let path = self.city_root.join(rel);

        storage::WriteTarget::within("edit file", &self.city_root, &path)
            .map_err(storage::StorageError::into_ax)?;

        let _held = self.desk.locks.hold(&path);
        let intent = if old.is_empty() {
            Intent::Create
        } else {
            Intent::Replace
        };
        let (text, current) = match intent {
            Intent::Create => (
                String::new(),
                Stamp::of(&path).map_err(|err| unreadable(rel, &err))?,
            ),
            Intent::Replace => {

                match kernel::gate::replacing(policy.write, &target) {
                    kernel::GateOutcome::Allow => {}
                    kernel::GateOutcome::Deny { refusal } => return Err(*refusal),
                    kernel::GateOutcome::Ask { question } => return Err(*question),
                }
                read_whole(rel, &path)?
            }
        };
        let seen = self.desk.seen.last(&target);
        match admission(seen.as_ref(), current.as_ref(), intent) {
            Admission::Lands => {}
            Admission::Absent => {
                return Err(AxError::failure(
                    AxCode::InvalidArgs,
                    "edit file",
                    format!("{rel} does not exist"),
                )
                .with_recovery(
                    "check the path; to create a new file pass old_string:\"\" and the whole \
                     file in `new_string`",
                ));
            }
            Admission::Unread => {
                return Err(AxError::failure(
                    AxCode::InvalidArgs,
                    "edit file",
                    format!("{rel} has not been read in this run"),
                )
                .with_recovery(format!(
                    "read {rel} first - a window around the passage you are changing is enough - \
                     then edit"
                )));
            }
            Admission::Moved => {

                self.desk.seen.look(&target, current.clone(), current);
                let why = match passage::around(&text, old) {
                    (passage::Opened::AroundOld, window) => {
                        format!("{rel} changed after this run read it; {window}")
                    }
                    (passage::Opened::AtHead, window) => format!(
                        "{rel} changed after this run read it, and the first line of \
                         `old_string` is no longer in it; {window}"
                    ),
                };
                return Err(AxError::failure(AxCode::VersionConflict, "edit file", why)
                    .with_recovery(
                        "edit again against the passage as it now reads, or read the file",
                    ));
            }
            Admission::Present => return Err(already_exists(rel)),
        }
        if intent == Intent::Create {
            return self.create(&target, rel, &path, new);
        }
        let (old, new) = as_the_file_spells(&text, old, new);
        let (old, new) = (old.as_ref(), new.as_ref());
        let updated = match (text.matches(old).count(), every) {
            (0, _) => Err(AxError::failure(
                AxCode::InvalidArgs,
                "edit file",
                format!("`old_string` matches 0 times in {rel}"),
            )
            .with_recovery("copy `old_string` from the file as it reads, whitespace included")),
            (1, false) => Ok(text.replacen(old, new, 1)),
            (_, true) => Ok(text.replace(old, new)),
            (hits, false) => Err(AxError::failure(
                AxCode::InvalidArgs,
                "edit file",
                format!("`old_string` matches {hits} times in {rel}, not once"),
            )
            .with_recovery(
                "include enough surrounding text in `old_string` to name exactly one \
                 occurrence, or set `replace_all: true` to change all of them",
            )),
        }?;

        storage::WriteTarget::within("edit file", &self.city_root, &path)
            .and_then(|cleared| cleared.replace(updated.as_bytes()))
            .map_err(storage::StorageError::into_ax)?;
        self.note_landing(&target, rel, &path)?;
        Ok(ToolOutcome {
            result: echo(rel, &text, &updated)?,
            attachments: Vec::new(),
        })
    }
}

impl EditTool {

    fn create(
        &self,
        target: &kernel::Address,
        rel: &str,
        path: &Path,
        new: &str,
    ) -> Result<ToolOutcome, AxError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|err| {
                AxError::failure(AxCode::StorageFatal, "edit file", format!("{rel}: {err}"))
                    .with_recovery(format!(
                        "free space on the disk holding the city, or remove the file \
                         standing where {rel}'s parent directory has to go"
                    ))
            })?;
        }
        storage::WriteTarget::within("edit file", &self.city_root, path)
            .and_then(|cleared| cleared.create(new.as_bytes()))
            .map_err(|err| refused_create(err, rel))?;
        self.note_landing(target, rel, path)?;
        Ok(ToolOutcome {
            result: echo(rel, "", new)?,
            attachments: Vec::new(),
        })
    }

    fn note_landing(
        &self,
        target: &kernel::Address,
        rel: &str,
        path: &Path,
    ) -> Result<(), AxError> {
        let landed = Stamp::of(path).map_err(|err| unreadable(rel, &err))?;
        self.desk.seen.look(target, landed.clone(), landed);
        Ok(())
    }
}

fn as_the_file_spells<'a>(text: &str, old: &'a str, new: &'a str) -> (Cow<'a, str>, Cow<'a, str>) {
    if text.contains(old) {
        return (Cow::Borrowed(old), Cow::Borrowed(new));
    }
    let numbering = crate::rendering::Numbering::of(crate::rendering::CURRENT);
    let copied = crate::rendering::Copied::of(old, numbering);
    let mut confirmed = copied.starts().into_iter().filter_map(|start| {
        let (bare, numbers) = copied.drawn_from(start)?;
        begins_on_line(text, &bare, start).then_some((bare, numbers))
    });
    match (confirmed.next(), confirmed.next()) {
        (Some((bare, numbers)), None) => (
            Cow::Owned(bare),
            Cow::Owned(crate::rendering::Copied::of(new, numbering).without(&numbers)),
        ),
        (Some(_), Some(_)) | (None, _) => (Cow::Borrowed(old), Cow::Borrowed(new)),
    }
}

fn begins_on_line(text: &str, passage: &str, line: u64) -> bool {
    !passage.is_empty()
        && text.match_indices(passage).any(|(at, _)| {
            text.get(..at).is_some_and(|before| {
                (before.is_empty() || before.ends_with('\n'))
                    && u64::try_from(before.matches('\n').count()) == Ok(line)
            })
        })
}

fn read_whole(rel: &str, path: &Path) -> Result<(String, Option<Stamp>), AxError> {
    let before = Stamp::of(path).map_err(|err| unreadable(rel, &err))?;
    if before.is_none() {
        return Ok((String::new(), None));
    }
    let bytes = std::fs::read(path).map_err(|err| unreadable(rel, &err))?;
    let after = Stamp::of(path).map_err(|err| unreadable(rel, &err))?;
    if before != after {
        return Err(AxError::failure(
            AxCode::VersionConflict,
            "edit file",
            format!("{rel} was being written while this edit read it"),
        )
        .with_recovery("read the file again, then edit"));
    }
    let text = String::from_utf8(bytes).map_err(|_| {
        AxError::failure(
            AxCode::InvalidArgs,
            "edit file",
            format!("{rel} is not valid UTF-8"),
        )
        .with_recovery(format!(
            "edit {rel} in a program that reads its bytes; this tool changes text \
             files and this one is not text"
        ))
    })?;
    Ok((text, after))
}

fn unreadable(rel: &str, err: &std::io::Error) -> AxError {
    AxError::failure(AxCode::InvalidArgs, "edit file", format!("{rel}: {err}"))
        .with_recovery("check the path; to create a new file pass old_string:\"\"")
}

fn echo(rel: &str, before: &str, after: &str) -> Result<Payload, AxError> {
    let mut result = Map::new();
    result.insert("path".to_owned(), Value::String(rel.to_owned()));
    result.insert(
        "diff".to_owned(),
        Value::String(unified_diff(rel, before, after)),
    );
    Payload::new(result)
}

fn refused_create(err: storage::StorageError, rel: &str) -> AxError {
    if !matches!(err, storage::StorageError::NameTaken { .. }) {
        return err.into_ax();
    }
    already_exists(rel)
}

fn already_exists(rel: &str) -> AxError {
    AxError::failure(
        AxCode::VersionConflict,
        "edit file",
        format!("{rel} already exists, so `old_string: \"\"` cannot create it"),
    )
    .with_recovery("read the file and replace a passage in it, or choose another path")
}

fn unified_diff(path: &str, before: &str, after: &str) -> String {
    let old_lines: Vec<&str> = before.lines().collect();
    let new_lines: Vec<&str> = after.lines().collect();
    let prefix = common_prefix(&old_lines, &new_lines);
    let suffix = common_suffix(&old_lines, &new_lines, prefix);
    let old_end = old_lines.len().saturating_sub(suffix);
    let new_end = new_lines.len().saturating_sub(suffix);

    let mut out = if before.is_empty() {
        format!("--- /dev/null\n+++ b/{path}\n")
    } else {
        format!("--- a/{path}\n+++ b/{path}\n")
    };
    let old_count = old_end.saturating_sub(prefix);
    let new_count = new_end.saturating_sub(prefix);
    out.push_str(&format!(
        "@@ -{},{} +{},{} @@\n",
        prefix.saturating_add(1),
        old_count,
        prefix.saturating_add(1),
        new_count
    ));
    for line in old_lines.get(prefix..old_end).unwrap_or_default() {
        out.push_str(&format!("-{line}\n"));
    }
    for line in new_lines.get(prefix..new_end).unwrap_or_default() {
        out.push_str(&format!("+{line}\n"));
    }
    out
}

fn common_prefix(old: &[&str], new: &[&str]) -> usize {
    let mut i = 0usize;
    while let (Some(a), Some(b)) = (old.get(i), new.get(i)) {
        if a != b {
            break;
        }
        i = i.saturating_add(1);
    }
    i
}

fn common_suffix(old: &[&str], new: &[&str], prefix: usize) -> usize {
    let mut i = 0usize;
    loop {
        let old_left = old.len().saturating_sub(i);
        let new_left = new.len().saturating_sub(i);
        if old_left <= prefix || new_left <= prefix {
            break;
        }
        let (Some(a), Some(b)) = (
            old.get(old_left.saturating_sub(1)),
            new.get(new_left.saturating_sub(1)),
        ) else {
            break;
        };
        if a != b {
            break;
        }
        i = i.saturating_add(1);
    }
    i
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
#[cfg_attr(
    windows,
    expect(clippy::disallowed_methods, reason = "test fixture (child D4)")
)]
mod tests;
