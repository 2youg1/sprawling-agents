// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::Path;

pub(crate) fn identity_answer(city_root: &Path) -> wire::Answer {
    match stated(city_root) {
        Ok(answer) => wire::Answer::Identity(Box::new(answer)),

        Err(_) => super::super::prepared::unavailable("Identity".to_owned()),
    }
}

fn stated(city_root: &Path) -> Result<wire::IdentityAnswer, kernel::AxError> {
    let naming = match city::read_naming(city_root)? {
        Ok(naming) => naming,
        Err(city::Unreadable {
            document,
            line,
            why,
        }) => {
            return Ok(wire::IdentityAnswer::Unreadable {
                document: document_of(document),
                line,
                why,
            });
        }
    };
    Ok(wire::IdentityAnswer::Stated(wire::StatedIdentity {
        user_id: naming.person().map(str::to_owned),
        imported_from: naming.imported_from().map(str::to_owned),
        about: naming.about().to_owned(),
        mayor: naming.mayor().map(str::to_owned),
        version: naming.version()?,
        preferences_text: text_of(city_root, city::Governed::Preferences)?,
        mayor_text: text_of(city_root, city::Governed::Mayor)?,
    }))
}

fn document_of(which: city::Governed) -> wire::GovernedDocument {
    match which {
        city::Governed::Mayor => wire::GovernedDocument::Mayor,
        city::Governed::Clerk => wire::GovernedDocument::Clerk,
        city::Governed::Preferences => wire::GovernedDocument::Preferences,
    }
}

fn text_of(city_root: &Path, which: city::Governed) -> Result<String, kernel::AxError> {
    let path = which.path(city_root);
    match std::fs::read_to_string(&path) {
        Ok(text) => Ok(text),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        Err(err) => Err(kernel::AxError::failure(
            kernel::AxCode::StorageFatal,
            "read a governed document",
            format!("{}: {err}", path.display()),
        )
        .with_recovery("fix that file's permissions")),
    }
}
