// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};

use kernel::{Address, AxCode, AxError, B3Hash, Locator};
use same_file::Handle;

use super::chosen_path::{self, Located, ReadBound};

#[derive(Clone)]
pub struct BoundReader {
    pub(in crate::tools) city_root: PathBuf,
    pub(in crate::tools) bound: ReadBound,
    pub(in crate::tools) block_store: PathBuf,
}

impl BoundReader {
    #[must_use]
    pub fn new(city_root: &Path, bound: ReadBound, block_store: &Path) -> BoundReader {
        BoundReader {
            city_root: city_root.to_path_buf(),
            bound,
            block_store: block_store.to_path_buf(),
        }
    }

    pub fn open(&self, asked: &str, action: &'static str) -> Result<Opened, AxError> {
        if let Some(found) = super::read::locator::open_locator(asked, self, action) {
            let (locator, bytes) = found?;
            let named = match locator {
                Locator::Cas { hash, .. } => Named::Block(hash),
                Locator::File { address, .. } => Named::File(address),
            };
            return Ok(Opened {
                named,
                source: Source::Held(Cursor::new(bytes)),
            });
        }
        let spelled = chosen_path::within_city(&self.city_root, asked, action)?;
        let addr = chosen_path::admit(&spelled, action, &*self.bound)?;

        let city = chosen_path::CityRoot::anchor(&self.city_root, action)?;
        let real = match chosen_path::land(&city, &addr, action, &*self.bound)?.location {
            Located::Present(real) if real.is_file() => real,
            Located::Present(_) => {
                return Err(AxError::failure(
                    AxCode::InvalidArgs,
                    action,
                    format!("{asked} is not a file"),
                )
                .with_recovery("name one file; `search` finds files by name"));
            }
            Located::Absent(_) => return Err(not_there(asked, action)),
        };
        let opened = std::fs::File::open(&real)
            .and_then(Handle::from_file)
            .map_err(|err| unopened(asked, action, &err))?;
        chosen_path::still_judged(asked, &real, &opened, action)?;
        Ok(Opened {
            named: Named::File(addr),
            source: Source::Disk(opened),
        })
    }

    pub fn pin(&self, asked: &str, action: &'static str) -> Result<Locator, AxError> {
        let mut bytes = Vec::new();
        self.open(asked, action)?
            .read_to_end(&mut bytes)
            .map_err(|err| {
                AxError::failure(AxCode::StorageFatal, action, format!("{asked}: {err}"))
                    .with_recovery("the User has to make the file readable")
            })?;
        Ok(Locator::cas(B3Hash::digest(&bytes)))
    }
}

fn not_there(asked: &str, action: &'static str) -> AxError {
    AxError::failure(AxCode::InvalidArgs, action, format!("{asked} is not there"))
        .with_recovery("name a file that exists; `search` finds files by name")
}

fn unopened(asked: &str, action: &'static str, err: &std::io::Error) -> AxError {
    if err.kind() == std::io::ErrorKind::NotFound {
        return not_there(asked, action);
    }
    AxError::failure(AxCode::StorageFatal, action, format!("{asked}: {err}"))
        .with_recovery("the User has to make the file readable")
}

pub struct Opened {
    named: Named,
    source: Source,
}

enum Source {
    Disk(Handle),

    Held(Cursor<Vec<u8>>),
}

impl Opened {
    #[must_use]
    pub fn named(&self) -> &Named {
        &self.named
    }

    pub fn byte_len(&self) -> std::io::Result<u64> {
        match &self.source {
            Source::Disk(handle) => handle.as_file().metadata().map(|meta| meta.len()),
            Source::Held(bytes) => Ok(u64::try_from(bytes.get_ref().len()).unwrap_or(u64::MAX)),
        }
    }
}

impl Read for Opened {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match &mut self.source {
            Source::Disk(handle) => handle.as_file_mut().read(buf),
            Source::Held(bytes) => bytes.read(buf),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Named {
    File(Address),
    Block(B3Hash),
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
    use std::sync::{Arc, Mutex};

    use kernel::{ReadVerdict, Tool as _, ToolCall, ToolName};

    use super::*;

    fn vault_closed() -> ReadBound {
        Arc::new(|addr: &Address| {
            if addr.as_str().starts_with("vault") {
                ReadVerdict::Confidential
            } else {
                ReadVerdict::Open
            }
        })
    }

    fn origin(building: &str) -> storage::BlockOrigin {
        storage::BlockOrigin {
            run: kernel::RunId::from_bytes([7; 16]),
            building: Address::parse(building).unwrap(),
        }
    }

    fn refused(opened: Result<Opened, AxError>) -> AxError {
        match opened {
            Ok(opened) => panic!("{asked:?} was let through", asked = opened.named),
            Err(err) => err,
        }
    }

    #[test]
    fn the_door_refuses_what_read_refuses_with_the_same_code() {
        let city = tempfile::tempdir().unwrap();
        let store = kernel::layout::CityLayout::new(city.path()).cas();
        storage::Cas::open(&store).unwrap();
        let door = BoundReader::new(city.path(), vault_closed(), &store);
        let read = crate::tools::ReadTool::new(
            city.path(),
            Arc::new(Mutex::new(crate::catalog::Catalog::new())),
            vault_closed(),
            &store,
        )
        .unwrap();
        let stray = format!("cas:b3-{}", B3Hash::digest(b"never stored"));
        let asked = [
            "vault/room/shot.png",
            "lab/.sprawling/RULES.toml",
            stray.as_str(),
        ];
        let codes: Vec<(AxCode, AxCode, String)> = asked
            .iter()
            .map(|asked| {
                let by_door = refused(door.open(asked, "ocr"));
                let call = ToolCall {
                    id: "call-1".to_owned(),
                    name: ToolName::parse("read").unwrap(),
                    args: kernel::Payload::new(
                        serde_json::json!({ "file_path": asked })
                            .as_object()
                            .cloned()
                            .unwrap(),
                    )
                    .unwrap(),
                };
                let by_read = read.invoke(&call).unwrap_err();
                (
                    *by_door.code(),
                    *by_read.code(),
                    by_door.action().to_owned(),
                )
            })
            .collect();
        let owed = vec![(AxCode::GateDenied, AxCode::GateDenied, "ocr".to_owned()); 3];
        assert_eq!(codes, owed);
    }

    #[test]
    fn a_file_and_a_block_come_back_as_their_bytes_and_what_they_were() {
        let city = tempfile::tempdir().unwrap();
        let store = kernel::layout::CityLayout::new(city.path()).cas();
        let hash = storage::Cas::open(&store)
            .unwrap()
            .put_for(b"block bytes", &origin("hall"))
            .unwrap();
        std::fs::create_dir_all(city.path().join("hall").join("room")).unwrap();
        std::fs::write(
            city.path().join("hall").join("room").join("shot.png"),
            b"file bytes",
        )
        .unwrap();
        let door = BoundReader::new(city.path(), vault_closed(), &store);
        let read_back = |asked: &str| {
            door.open(asked, "ocr").map(|mut opened| {
                let mut bytes = Vec::new();
                opened.read_to_end(&mut bytes).unwrap();
                (opened.named().clone(), bytes)
            })
        };
        assert_eq!(
            vec![
                read_back("hall/room/shot.png"),
                read_back(&format!("cas:b3-{hash}")),
            ],
            vec![
                Ok((
                    Named::File(Address::parse("hall/room/shot.png").unwrap()),
                    b"file bytes".to_vec()
                )),
                Ok((Named::Block(hash), b"block bytes".to_vec())),
            ]
        );
    }

    #[test]
    fn a_file_that_is_not_there_or_is_a_directory_is_the_callers_mistake() {
        let city = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(city.path().join("lab").join("room")).unwrap();
        let store = kernel::layout::CityLayout::new(city.path()).cas();
        let door = BoundReader::new(city.path(), vault_closed(), &store);
        let codes: Vec<AxCode> = ["lab/room/absent.png", "lab/room"]
            .into_iter()
            .map(|asked| *refused(door.open(asked, "ocr")).code())
            .collect();
        assert_eq!(codes, vec![AxCode::InvalidArgs, AxCode::InvalidArgs]);
    }
}
