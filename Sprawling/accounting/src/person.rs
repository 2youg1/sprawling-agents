// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::{Path, PathBuf};

use kernel::{AxCode, AxError};
use wire::{CorePreferences, PreferencePatch, PreferencesAnswer};

use crate::home::Home;

const UI: &str = "ui";

const CORE: &str = "core";

const CITY: &str = "city";

const MEMORY: &str = "memory";

const AUTO: &str = "auto";

pub fn read() -> Result<PreferencesAnswer, AxError> {
    stated(&file()?)
}

pub fn put(patch: PreferencePatch) -> Result<(), AxError> {
    land(&file()?, patch)
}

fn land(file: &Path, patch: PreferencePatch) -> Result<(), AxError> {
    city::edit_document(file, |held| {
        let mut document = document(file)?;
        match patch {
            PreferencePatch::CorePriority(_)
            | PreferencePatch::CorePlacement(_)
            | PreferencePatch::RunMemory(_) => {
                let mut settled = section(&document, file)?;
                settled.apply(patch);
                document.insert(
                    CORE.to_owned(),
                    toml::Value::try_from(&settled.core).map_err(|err| invalid(file, &err))?,
                );
            }
            PreferencePatch::Lang(_)
            | PreferencePatch::Welcomed(_)
            | PreferencePatch::Panel(_)
            | PreferencePatch::Tier(_)
            | PreferencePatch::Appearance(_)
            | PreferencePatch::Proxying(_)
            | PreferencePatch::Chord(_)
            | PreferencePatch::Tags(_)
            | PreferencePatch::Theme(_) => {
                let mut settled = section(&document, file)?;
                settled.apply(patch);
                document.insert(UI.to_owned(), rendered(&settled, file)?);
            }
        }
        let text = toml::to_string_pretty(&document).map_err(|err| invalid(file, &err))?;
        held.replace(text.as_bytes())
    })
}

pub fn city() -> Result<Option<PathBuf>, AxError> {
    stated_city(&file()?)
}

fn stated_city(file: &Path) -> Result<Option<PathBuf>, AxError> {
    match document(file)?.get(CITY) {
        None => Ok(None),
        Some(toml::Value::String(raw)) if Path::new(raw).is_absolute() => {
            Ok(Some(PathBuf::from(raw)))
        }
        Some(other) => Err(AxError::failure(
            AxCode::ConfigInvalid,
            "read the city the User named",
            format!("{}: {CITY} = {other} is not an absolute path", file.display()),
        )
        .with_recovery(format!(
            "write {CITY} as the whole path of the city's folder, or delete the line to use the default"
        ))),
    }
}

pub fn settle_city(city: &Path) -> Result<(), AxError> {
    name_city(&file()?, city)
}

fn name_city(file: &Path, city: &Path) -> Result<(), AxError> {
    let whole = std::path::absolute(city).map_err(|err| {
        AxError::failure(
            AxCode::StorageFatal,
            "name the User's city",
            format!("{}: {err}", city.display()),
        )
        .with_recovery("start again from a folder this process can read")
    })?;
    let Some(spelled) = whole.to_str() else {
        return Err(AxError::failure(
            AxCode::ConfigInvalid,
            "name the User's city",
            format!("{} is not text", whole.display()),
        )
        .with_recovery("choose a folder whose path has no undecodable characters"));
    };
    city::edit_document(file, |held| {
        let mut document = document(file)?;
        document.insert(CITY.to_owned(), toml::Value::String(spelled.to_owned()));
        let text = toml::to_string_pretty(&document).map_err(|err| invalid(file, &err))?;
        held.replace(text.as_bytes())
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MachineMemory {
    Auto,
    Bytes(u64),
}

pub fn machine_memory() -> Result<MachineMemory, AxError> {
    stated_machine_memory(&file()?)
}

fn stated_machine_memory(file: &Path) -> Result<MachineMemory, AxError> {
    let refused = |other: &toml::Value| {
        AxError::failure(
            AxCode::ConfigInvalid,
            "read the machine's memory budget",
            format!(
                "{}: {MEMORY} = {other} is neither \"{AUTO}\" nor a number of bytes",
                file.display()
            ),
        )
        .with_recovery(format!(
            "write {MEMORY} = \"{AUTO}\" to let the city size it, a whole number of bytes, \
             or delete the line"
        ))
    };
    match document(file)?.get(MEMORY) {
        None => Ok(MachineMemory::Auto),
        Some(toml::Value::String(spelled)) if spelled == AUTO => Ok(MachineMemory::Auto),
        Some(stated @ toml::Value::Integer(bytes)) => u64::try_from(*bytes)
            .map(MachineMemory::Bytes)
            .map_err(|_| refused(stated)),
        Some(other) => Err(refused(other)),
    }
}

pub use wire::{CorePlacement, CorePriority};

pub fn core_priority() -> Result<CorePriority, AxError> {
    stated_core_priority(&file()?)
}

fn stated_core_priority(file: &Path) -> Result<CorePriority, AxError> {
    Ok(stated(file)?.core.priority)
}

pub fn core_placement() -> Result<CorePlacement, AxError> {
    stated_core_placement(&file()?)
}

fn stated_core_placement(file: &Path) -> Result<CorePlacement, AxError> {
    Ok(stated(file)?.core.placement)
}

fn file() -> Result<PathBuf, AxError> {
    Ok(Home::detect()?.person_config())
}

fn stated(file: &Path) -> Result<PreferencesAnswer, AxError> {
    section(&document(file)?, file)
}

fn document(file: &Path) -> Result<toml::Table, AxError> {
    match std::fs::read_to_string(file) {
        Ok(text) => {
            city::admit_format(city::FormatFile::PersonConfig, &text)?;
            toml::from_str(&text).map_err(|err| invalid(file, &err))
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(toml::Table::new()),
        Err(err) => Err(AxError::failure(
            AxCode::StorageFatal,
            "read the User's preferences",
            format!("{}: {err}", file.display()),
        )
        .with_recovery(
            "fix that file's permissions; the preferences it holds are read at every start",
        )),
    }
}

fn section(document: &toml::Table, file: &Path) -> Result<PreferencesAnswer, AxError> {
    if document.get(UI).and_then(|ui| ui.get(CORE)).is_some() {
        return Err(invalid(file, &"[ui] core is not stored here; use [core]"));
    }
    let mut answer: PreferencesAnswer = match document.get(UI) {
        None => PreferencesAnswer::default(),
        Some(section) => section
            .clone()
            .try_into()
            .map_err(|err| invalid(file, &format!("[{UI}]: {err}")))?,
    };
    answer.core = match document.get(CORE) {
        None => CorePreferences::default(),
        Some(section) => section
            .clone()
            .try_into()
            .map_err(|err| invalid(file, &format!("[{CORE}]: {err}")))?,
    };
    Ok(answer)
}

fn rendered(settled: &PreferencesAnswer, file: &Path) -> Result<toml::Value, AxError> {
    let mut ui = toml::Value::try_from(settled).map_err(|err| invalid(file, &err))?;
    if let toml::Value::Table(table) = &mut ui {
        table.remove(CORE);
    }
    Ok(ui)
}

fn invalid(file: &Path, why: &impl std::fmt::Display) -> AxError {
    AxError::failure(
        AxCode::ConfigInvalid,
        "read the User's preferences",
        format!("{}: {why}", file.display()),
    )
    .with_recovery(
        "fix the named section by hand, or delete it and choose again on the settings page",
    )
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
    use wire::{Chord, Lang, SessionTags, Tag};

    #[test]
    fn what_the_file_states_and_what_the_answer_states_are_one_record() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("config.toml");
        let mut expected = PreferencesAnswer::default();
        for patch in [
            PreferencePatch::Welcomed(true),
            PreferencePatch::Lang(Lang::Zh),
            PreferencePatch::Chord(Chord {
                action: "go.city".to_owned(),
                spelled: "accel+2".to_owned(),
            }),
            PreferencePatch::Panel(false),
            PreferencePatch::Tags(SessionTags {
                city: kernel::Address::parse("harbour").unwrap(),
                room: kernel::Address::parse("hall/mayor").unwrap(),
                began: kernel::Seq::new(7),
                tags: vec![Tag::parse("重构").unwrap(), Tag::parse("pin").unwrap()],
            }),
            PreferencePatch::Lang(Lang::En),
            PreferencePatch::CorePlacement(CorePlacement::SoftShares),
            PreferencePatch::RunMemory(std::num::NonZeroU64::new(64 << 20)),
            PreferencePatch::CorePriority(CorePriority::Normal),
            PreferencePatch::RunMemory(None),
        ] {
            expected.apply(patch.clone());
            land(&file, patch).unwrap();
            assert_eq!(stated(&file).unwrap(), expected);
        }
        assert_eq!(expected.lang, Some(Lang::En));
    }

    #[test]
    fn the_named_city_is_absolute_and_naming_it_touches_nothing_else() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("config.toml");
        std::fs::write(&file, "[mine]\nkept = 1\n").unwrap();
        assert_eq!(stated_city(&file).unwrap(), None);

        let city = dir.path().join("sprawling").join("city");
        name_city(&file, &city).unwrap();
        assert_eq!(stated_city(&file).unwrap(), Some(city));
        let text = std::fs::read_to_string(&file).unwrap();
        assert!(text.contains("kept = 1"), "{text}");
        assert!(!text.contains(city::FORMAT_KEY), "{text}");

        std::fs::write(&file, "city = \"sprawling/city\"\n").unwrap();
        assert_eq!(
            stated_city(&file).unwrap_err().code(),
            &AxCode::ConfigInvalid
        );
    }

    #[test]
    fn the_machine_memory_is_auto_or_bytes() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("config.toml");
        let read = |text: &str| {
            std::fs::write(&file, text).unwrap();
            stated_machine_memory(&file).map_err(|err| *err.code())
        };
        assert_eq!(
            [
                read(""),
                read("memory = \"auto\"\n"),
                read("memory = 1024\n"),
                read("memory = -1\n"),
                read("memory = \"lots\"\n"),
            ],
            [
                Ok(MachineMemory::Auto),
                Ok(MachineMemory::Auto),
                Ok(MachineMemory::Bytes(1024)),
                Err(AxCode::ConfigInvalid),
                Err(AxCode::ConfigInvalid),
            ]
        );
    }

    #[test]
    fn a_person_file_from_a_newer_build_names_the_upgrade() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("config.toml");
        std::fs::write(&file, "format = 2\n[ui]\n").unwrap();
        let refused = stated(&file).unwrap_err();
        assert!(
            refused.recovery().contains("upgrade sprawling"),
            "{refused}"
        );
    }

    #[test]
    fn a_section_this_build_does_not_read_survives_a_write() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("config.toml");
        std::fs::write(&file, "[theirs]\nkept = \"yes\"\n").unwrap();
        land(&file, PreferencePatch::Welcomed(true)).unwrap();
        let text = std::fs::read_to_string(&file).unwrap();
        assert!(text.contains("kept"), "{text}");
        assert!(stated(&file).unwrap().welcomed);
    }

    #[test]
    fn a_person_who_turns_the_raise_off_gets_normal_priority() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("config.toml");
        let absent = stated_core_priority(&file).unwrap();
        std::fs::write(
            &file,
            "[core]
priority = \"normal\"
",
        )
        .unwrap();
        let off = stated_core_priority(&file).unwrap();
        std::fs::write(
            &file,
            "[core]
priority = \"fast\"
",
        )
        .unwrap();
        let misspelled = stated_core_priority(&file).map_err(|err| *err.code());
        assert_eq!(
            (absent, off, misspelled),
            (
                CorePriority::Raised,
                CorePriority::Normal,
                Err(AxCode::ConfigInvalid)
            )
        );
    }

    #[test]
    fn each_placement_spelling_reads_as_its_arm() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("config.toml");
        let absent = stated_core_placement(&file).map_err(|err| *err.code());
        let stated = ["none", "soft", "soft_shares", "pinned", "hard"].map(|arm| {
            std::fs::write(
                &file,
                format!(
                    "[core]
placement = \"{arm}\"
"
                ),
            )
            .unwrap();
            stated_core_placement(&file).map_err(|err| *err.code())
        });
        assert_eq!(
            (absent, stated),
            (
                Ok(CorePlacement::Soft),
                [
                    Ok(CorePlacement::Off),
                    Ok(CorePlacement::Soft),
                    Ok(CorePlacement::SoftShares),
                    Ok(CorePlacement::Pinned),
                    Err(AxCode::ConfigInvalid),
                ]
            )
        );
    }

    #[test]
    fn the_core_priority_lands_in_its_own_section_and_reads_back() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("config.toml");
        land(&file, PreferencePatch::Lang(Lang::Zh)).unwrap();
        land(&file, PreferencePatch::CorePriority(CorePriority::Normal)).unwrap();
        assert_eq!(
            (
                stated_core_priority(&file).unwrap(),
                stated(&file).unwrap().lang
            ),
            (CorePriority::Normal, Some(Lang::Zh))
        );
    }

    #[test]
    fn performance_answer_reads_the_core_section() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("config.toml");
        std::fs::write(
            &file,
            "[core]
priority = \"normal\"
placement = \"none\"
",
        )
        .unwrap();
        assert_eq!(
            stated(&file).unwrap().core,
            CorePreferences {
                priority: CorePriority::Normal,
                placement: CorePlacement::Off,
                memory_bytes: None,
            }
        );
    }

    #[test]
    fn a_zero_ceiling_and_a_duplicate_authority_are_refused_without_overwriting() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("config.toml");
        for text in [
            "[core]
memory_bytes = 0
",
            "[ui.core]
priority = \"normal\"
",
        ] {
            std::fs::write(&file, text).unwrap();
            assert_eq!(
                land(&file, PreferencePatch::Panel(false))
                    .unwrap_err()
                    .code(),
                &AxCode::ConfigInvalid
            );
            assert_eq!(std::fs::read_to_string(&file).unwrap(), text);
        }
    }

    #[test]
    fn a_file_that_does_not_parse_is_refused_rather_than_replaced() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("config.toml");
        std::fs::write(&file, "[ui]\nlang = \"fr\"\n").unwrap();
        let err = stated(&file).unwrap_err();
        assert_eq!(err.code(), &AxCode::ConfigInvalid);
        assert!(err.subject().contains("[ui]"), "{}", err.subject());
    }

    #[test]
    fn a_body_size_under_the_floor_is_refused_in_the_file_and_a_large_one_is_read() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("config.toml");
        let ui = |px: u32| {
            format!(
                "[ui.appearance]\nlighting = \"dark\"\nsans = \"maple\"\nmono = \"maple\"\n\
                 sans_stack = \"\"\nmono_stack = \"\"\nbody_px = {px}\n\
                 density = \"compact\"\nchroma = \"full\"\nmotion = \"system\"\n"
            )
        };
        std::fs::write(&file, ui(11)).unwrap();
        assert_eq!(stated(&file).unwrap_err().code(), &AxCode::ConfigInvalid);
        std::fs::write(&file, ui(24)).unwrap();
        assert_eq!(
            stated(&file).unwrap().appearance.body_px,
            Some(wire::BodyPx::new(24).unwrap())
        );
    }
}
