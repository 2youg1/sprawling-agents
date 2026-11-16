// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::Path;

use kernel::consts_policy::HALL_BUILDING;
use kernel::{Address, AxCode, AxError, Payload, RESERVED_PREFIX};

use super::Building;

const REMOVED_DIR: &str = "removed";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Removed {
    addr: Address,
    kept: String,
}

impl Removed {
    #[must_use]
    pub fn addr(&self) -> &Address {
        &self.addr
    }

    #[must_use]
    pub fn kept(&self) -> &str {
        &self.kept
    }
}

pub fn remove(city_root: &Path, addr: &Address) -> Result<Removed, AxError> {
    let building = Building::of(addr)?;
    let refuse = |why: &str, recovery: &str| {
        Err(AxError::failure(
            AxCode::InvalidArgs,
            "remove a building",
            format!("{}: {why}", addr.as_str()),
        )
        .with_recovery(recovery))
    };
    if building.addr() != addr {
        return refuse(
            "a room, not a building",
            "remove the building that holds it, or delete the room's files by hand",
        );
    }
    if addr.as_str() == HALL_BUILDING {
        return refuse(
            "City Hall is the city's own building",
            "City Hall stays; remove one of the other buildings",
        );
    }
    let from = building.root(city_root);
    if !from.is_dir() {
        return refuse(
            "holds no directory",
            "reload the city; the building may already be gone",
        );
    }
    let shelf = city_root.join(RESERVED_PREFIX).join(REMOVED_DIR);
    std::fs::create_dir_all(&shelf).map_err(|err| unmovable(&shelf, &err))?;
    let name = first_free(&shelf, addr.as_str())?;
    let to = shelf.join(&name);
    std::fs::rename(&from, &to).map_err(|err| unmovable(&from, &err))?;
    Ok(Removed {
        addr: addr.clone(),
        kept: format!("{RESERVED_PREFIX}/{REMOVED_DIR}/{name}"),
    })
}

fn first_free(shelf: &Path, name: &str) -> Result<String, AxError> {
    if !shelf.join(name).exists() {
        return Ok(name.to_owned());
    }
    (2_u32..=u32::MAX)
        .map(|n| format!("{name}-{n}"))
        .find(|candidate| !shelf.join(candidate).exists())
        .ok_or_else(|| {
            AxError::failure(
                AxCode::StorageFatal,
                "remove a building",
                format!("{}: every kept name for {name} is taken", shelf.display()),
            )
            .with_recovery("empty the removed buildings you no longer need, then remove again")
        })
}

pub fn removed_payload(removed: &Removed) -> Result<Payload, AxError> {
    let mut map = serde_json::Map::new();
    map.insert(
        "addr".to_owned(),
        serde_json::Value::String(removed.addr.as_str().to_owned()),
    );
    map.insert(
        "kept".to_owned(),
        serde_json::Value::String(removed.kept.clone()),
    );
    Payload::new(map)
}

fn unmovable(path: &Path, err: &std::io::Error) -> AxError {
    AxError::failure(
        AxCode::StorageFatal,
        "remove a building",
        format!("{}: {err}", path.display()),
    )
    .with_recovery("close any program that holds a file in the building open, then remove again")
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests {
    use super::super::{BuildingTemplate, all, create};
    use super::*;

    fn addr(raw: &str) -> Address {
        Address::parse(raw).unwrap()
    }

    #[test]
    fn a_removed_building_leaves_the_city_and_keeps_every_file() {
        let dir = tempfile::tempdir().unwrap();
        let lab = addr("lab");
        create(dir.path(), &lab, BuildingTemplate::Minimal).unwrap();
        std::fs::write(dir.path().join("lab").join("notes.md"), "first").unwrap();

        let first = remove(dir.path(), &lab).unwrap();
        assert_eq!(all(dir.path()).unwrap(), Vec::<Address>::new());
        create(dir.path(), &lab, BuildingTemplate::Minimal).unwrap();
        std::fs::write(dir.path().join("lab").join("notes.md"), "second").unwrap();
        let second = remove(dir.path(), &lab).unwrap();

        let kept = |removed: &Removed| {
            std::fs::read_to_string(dir.path().join(removed.kept()).join("notes.md")).unwrap()
        };
        assert_eq!(
            (first.kept(), kept(&first), second.kept(), kept(&second)),
            (
                ".sprawling/removed/lab",
                "first".to_owned(),
                ".sprawling/removed/lab-2",
                "second".to_owned()
            )
        );
    }

    #[test]
    fn a_room_and_city_hall_are_not_removed() {
        let dir = tempfile::tempdir().unwrap();
        create(dir.path(), &addr("lab"), BuildingTemplate::Minimal).unwrap();
        let refused = [addr("lab/room1"), addr(HALL_BUILDING), addr("mill")]
            .map(|at| remove(dir.path(), &at).map_err(|err| *err.code()));
        assert_eq!(refused, [const { Err(AxCode::InvalidArgs) }; 3]);
    }
}
