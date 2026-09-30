// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A key a person pastes into a dispatch goes to the vault, and the
//! dispatch carries its `secret:` reference instead (sprawling-SPEC.md
//! 8-87).

use kernel::{AxCode, AxError, Sealed, SecretRef};
use zeroize::Zeroizing;

use super::super::RunWorker;
use super::super::credentials::signing::Arrival;

#[cfg(test)]
mod tests;

/// The realm every pasted key is kept under.
const REALM: &str = "pasted";

impl RunWorker {
    /// Moves every provider-shaped key in `text` into the vault and
    /// returns the text with each key replaced by its reference.
    ///
    /// Only shape-table hits are taken: an entropy hit is as often a
    /// commit hash the resident has to read as it is a credential. Text
    /// with no hit comes back as it arrived, without a copy.
    ///
    /// # Errors
    /// Propagates the vault refusing the key and the ledger refusing
    /// its `secret_captured` record. The caller must not go on with the
    /// original text, because that is the leak this exists to close.
    pub(in crate::assembly) fn take_custody(&mut self, text: String) -> Result<String, AxError> {
        match kept_text(&text, |provider, key| self.keep_pasted(provider, key))? {
            Some(kept) => {
                drop(Zeroizing::new(text));
                Ok(kept)
            }
            None => Ok(text),
        }
    }

    /// Puts one pasted key in the vault under a name no earlier key
    /// holds: the ledger's next position, which the `secret_captured`
    /// record this writes moves past.
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

/// `text` with every provider-shaped key replaced by what `keep` puts in
/// its place, or `None` when `text` holds no such key.
///
/// Only shape-table hits are taken: an entropy hit is as often a commit
/// hash the resident has to read as it is a credential.
///
/// # Errors
/// Propagates `keep` refusing a key, and a span off the text's character
/// boundaries.
pub(in crate::assembly) fn kept_text<InPlace: std::fmt::Display>(
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

/// A span `scan` returned that does not fall on the text's character
/// boundaries. `scan` promises in-bounds spans over ASCII shapes, so this
/// is a defect in that promise, and the dispatch stops rather than
/// sending the text on.
fn outside_text() -> AxError {
    AxError::failure(
        AxCode::InvalidArgs,
        "take a pasted key into custody",
        "a detected key does not lie on the text's character boundaries".to_owned(),
    )
    .with_recovery("remove the key from the text and enrol it on the settings page instead")
}
