// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{AxCode, AxError, Sealed, SecretRef};
use zeroize::Zeroizing;

use super::super::RunWorker;
use super::super::credentials::signing::Arrival;

#[cfg(test)]
mod tests;

const REALM: &str = "pasted";

impl RunWorker {

    pub(in crate::worker) fn take_custody(&mut self, text: String) -> Result<String, AxError> {
        match kept_text(&text, |provider, key| self.keep_pasted(provider, key))? {
            Some(kept) => {
                drop(Zeroizing::new(text));
                Ok(kept)
            }
            None => Ok(text),
        }
    }

    fn keep_pasted(&mut self, provider: &str, key: &str) -> Result<SecretRef, AxError> {
        let reference = SecretRef::new(
            REALM,
            &format!("{provider}-{}", self.ledger.position().value()),
        )?;
        self.put_secret(
            &reference,
            Sealed::new(Box::new(key.to_owned())),
            Arrival::Pasted,
        )?;
        Ok(reference)
    }
}

pub(in crate::worker) fn kept_text<InPlace: std::fmt::Display>(
    text: &str,
    mut keep: impl FnMut(&str, &str) -> Result<InPlace, AxError>,
) -> Result<Option<String>, AxError> {
    let keys: Vec<_> = kernel::secret::scan(text.as_bytes())
        .into_iter()
        .filter_map(|span| span.provider.map(|provider| (span, provider)))
        .collect();
    if keys.is_empty() {
        return Ok(None);
    }
    let mut kept = String::with_capacity(text.len());
    let mut cursor = 0_usize;
    for (span, provider) in keys {
        let end = span.start.checked_add(span.len).ok_or_else(outside_text)?;
        kept.push_str(text.get(cursor..span.start).ok_or_else(outside_text)?);
        let key = text.get(span.start..end).ok_or_else(outside_text)?;
        kept.push_str(&keep(provider, key)?.to_string());
        cursor = end;
    }
    kept.push_str(text.get(cursor..).ok_or_else(outside_text)?);
    Ok(Some(kept))
}

fn outside_text() -> AxError {
    AxError::failure(
        AxCode::InvalidArgs,
        "take a pasted key into custody",
        "a detected key does not lie on the text's character boundaries".to_owned(),
    )
    .with_recovery("remove the key from the text and enrol it on the settings page instead")
}
