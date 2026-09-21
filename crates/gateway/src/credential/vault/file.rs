// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
//! The encrypted vault file: the backend for a machine whose credential
//! service keeps nothing across a reboot.
//!
//! The threat this answers is a tool call that reads files: an agent of
//! this city can grep the machine it runs on, so a credential in
//! cleartext beside the configuration is one the agent can put in a
//! window. Whoever already holds the passphrase is not kept out, and
//! disk theft is a different question (gateway-SPEC 8-21).
//!
//! One entry is sealed on its own with ChaCha20-Poly1305 under a fresh
//! 96-bit nonce, and the additional data binds the format version, the
//! the entry's name, so a ciphertext only opens where it was sealed.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use argon2::{Algorithm, Argon2, Params, Version};
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use chacha20poly1305::aead::{Aead, Payload};
use chacha20poly1305::{ChaCha20Poly1305, KeyInit, Nonce};
use kernel::{AxCode, AxError, SecretRef};
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

use super::{Persistence, Vault};

/// A file written by a later format opens with nothing from this one.
const FORMAT_VERSION: u8 = 1;
const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 12;
const KEY_LEN: usize = 32;
/// Argon2id cost, RFC 9106's second recommended setting: 64 MiB, three
/// passes, one lane. The one home for it — the file records the salt and
/// not these numbers, so changing them raises `FORMAT_VERSION`.
const MEMORY_KIB: u32 = 65_536;
const TIME_COST: u32 = 3;
const LANES: u32 = 1;
/// The one entry that holds no credential: opening it is how a wrong
/// passphrase becomes a refusal at the door.
const VERIFIER_SUBJECT: &str = "verifier";
const VERIFIER_PLAINTEXT: &[u8] = b"sprawling encrypted vault";
/// What a person does with a file this build cannot make sense of.
const START_OVER: &str = "move this file aside and store the credentials again";

/// The file, as it lies on disk: every byte is either public (version,
/// salt) or sealed, so a grep of it reads base64 of ciphertext.
#[derive(Serialize, Deserialize)]
struct Document {
    version: u8,
    salt: String,
    verifier: String,
    entries: BTreeMap<String, String>,
}

/// A vault kept in one encrypted file, unlocked by a passphrase. The
/// derived key stays in memory for the life of the process: Argon2id is
/// deliberately slow, and deriving it per read would put a second of
/// latency on every model call.
pub(crate) struct FileVault {
    path: PathBuf,
    key: Zeroizing<[u8; KEY_LEN]>,
    salt: [u8; SALT_LEN],
    entries: BTreeMap<String, String>,
}

impl FileVault {
    pub(crate) const SOURCE: &'static str = "encrypted-file";
    pub(crate) const PERSISTENCE: Persistence = Persistence::AcrossRebootsWithPassphrase;

    /// Open the file at `path`, or create it, deriving the key from
    /// `passphrase`.
    ///
    /// # Errors
    /// `E_CONFIG_INVALID` when the file cannot be read or written, when
    /// its bytes are not this format, or when the passphrase does not
    /// open it — never a panic, and never an empty vault that loses
    /// what the file holds.
    pub(crate) fn open(path: PathBuf, passphrase: &Zeroizing<String>) -> Result<Self, AxError> {
        let Some(bytes) = read_if_present(&path)? else {
            let mut salt = [0_u8; SALT_LEN];
            fill(&mut salt, "draw a salt for the credential file")?;
            let key = derive(passphrase, &salt)?;
            let fresh = FileVault {
                path,
                key,
                salt,
                entries: BTreeMap::new(),
            };
            fresh.save()?;
            return Ok(fresh);
        };
        let document: Document = serde_json::from_slice(&bytes).map_err(|err| {
            refusal(
                "read the credential file",
                format!("its bytes are not an encrypted vault: {err}"),
                START_OVER,
            )
        })?;
        if document.version != FORMAT_VERSION {
            return Err(refusal(
                "read the credential file",
                format!(
                    "it is written in format {} and this build writes {FORMAT_VERSION}",
                    document.version
                ),
                "run the build that wrote this file, or move it aside and store the \
                 credentials again",
            ));
        }
        let salt = decode_salt(&document.salt)?;
        let key = derive(passphrase, &salt)?;
        let opened = FileVault {
            path,
            key,
            salt,
            entries: document.entries,
        };
        let verifier = opened
            .unseal(VERIFIER_SUBJECT, &document.verifier)
            .map_err(|_| {
                refusal(
                    "unlock the credential file",
                    "the passphrase does not open this file".to_owned(),
                    "enter the passphrase this file was created with; nothing in it can                      be read without that passphrase",
                )
            })?;
        if verifier.as_slice() != VERIFIER_PLAINTEXT {
            return Err(refusal(
                "unlock the credential file",
                "the file's own marker is not what this format writes".to_owned(),
                START_OVER,
            ));
        }
        Ok(opened)
    }

    /// The additional data of one sealed piece: format version, then
    /// what the piece is — so a ciphertext only opens where it was
    /// sealed.
    fn aad(subject: &str) -> Vec<u8> {
        let mut aad = Vec::from(b"sprawling-vault/".as_slice());
        aad.push(FORMAT_VERSION);
        aad.extend_from_slice(subject.as_bytes());
        aad
    }

    fn cipher(&self) -> Result<ChaCha20Poly1305, AxError> {
        ChaCha20Poly1305::new_from_slice(self.key.as_slice()).map_err(|err| {
            refusal(
                "unlock the credential file",
                format!("the derived key is the wrong length: {err}"),
                "report this: the key length is fixed by this build and cannot be \
                 changed by anything on disk",
            )
        })
    }

    fn seal(&self, subject: &str, plaintext: &[u8]) -> Result<String, AxError> {
        let mut nonce = [0_u8; NONCE_LEN];
        fill(&mut nonce, "draw a nonce for the credential file")?;
        let aad = FileVault::aad(subject);
        let sealed = self
            .cipher()?
            .encrypt(
                &Nonce::from(nonce),
                Payload {
                    msg: plaintext,
                    aad: &aad,
                },
            )
            .map_err(|err| {
                refusal(
                    "seal a credential",
                    format!("the cipher refused this value: {err}"),
                    "store a shorter value; this backend seals one credential at a time",
                )
            })?;
        let mut record = Vec::from(nonce.as_slice());
        record.extend_from_slice(&sealed);
        Ok(BASE64.encode(record))
    }

    fn unseal(&self, subject: &str, encoded: &str) -> Result<Zeroizing<Vec<u8>>, AxError> {
        let record = BASE64.decode(encoded).map_err(|err| {
            refusal(
                "read a credential from the file",
                format!("its record is not base64: {err}"),
                START_OVER,
            )
        })?;
        let (nonce, sealed) = record.split_at_checked(NONCE_LEN).ok_or_else(|| {
            refusal(
                "read a credential from the file",
                "its record is shorter than one nonce".to_owned(),
                START_OVER,
            )
        })?;
        let nonce: [u8; NONCE_LEN] = nonce.try_into().map_err(|_| {
            refusal(
                "read a credential from the file",
                "its nonce is the wrong length".to_owned(),
                START_OVER,
            )
        })?;
        let aad = FileVault::aad(subject);
        let plain = self
            .cipher()?
            .decrypt(
                &Nonce::from(nonce),
                Payload {
                    msg: sealed,
                    aad: &aad,
                },
            )
            .map_err(|_| {
                refusal(
                    "read a credential from the file",
                    format!("the record for {subject} did not open"),
                    "enter the passphrase this file was created with, or store this \
                     credential again",
                )
            })?;
        Ok(Zeroizing::new(plain))
    }

    /// Write the whole file, and leave either the old file or the new
    /// one behind — never a half-written one.
    ///
    /// **This module owns the replacement of this one file**, and
    /// gateway-SPEC 8-21 records why it is not `city::document`.
    fn save(&self) -> Result<(), AxError> {
        let document = Document {
            version: FORMAT_VERSION,
            salt: BASE64.encode(self.salt),
            verifier: self.seal(VERIFIER_SUBJECT, VERIFIER_PLAINTEXT)?,
            entries: self.entries.clone(),
        };
        let bytes = serde_json::to_vec_pretty(&document).map_err(|err| {
            refusal(
                "write the credential file",
                format!("the file could not be composed: {err}"),
                "report this: every value in it was sealed by this build a moment ago",
            )
        })?;
        let staged = self.path.with_extension("staged");
        write_private(&staged, &bytes)?;
        std::fs::rename(&staged, &self.path).map_err(|err| {
            refusal(
                "write the credential file",
                format!("the staged file could not take its place: {err}"),
                "check that the directory holding the credential file is writable, then \
                 store the credential again",
            )
        })
    }
}

impl Vault for FileVault {
    fn put(&mut self, reference: &SecretRef, value: Zeroizing<String>) -> Result<(), AxError> {
        let sealed = self.seal(&reference.to_string(), value.as_bytes())?;
        self.entries.insert(reference.to_string(), sealed);
        self.save()
    }

    fn get(&self, reference: &SecretRef) -> Result<Option<Zeroizing<String>>, AxError> {
        let key = reference.to_string();
        let Some(record) = self.entries.get(&key) else {
            return Ok(None);
        };
        let plain = self.unseal(&key, record)?;
        let text = String::from_utf8(plain.to_vec()).map_err(|_| {
            refusal(
                "read a credential from the file",
                format!("the value stored for {key} is not text"),
                "store this credential again",
            )
        })?;
        Ok(Some(Zeroizing::new(text)))
    }

    fn delete(&mut self, reference: &SecretRef) -> Result<(), AxError> {
        if self.entries.remove(&reference.to_string()).is_none() {
            return Ok(());
        }
        self.save()
    }
}

/// One shape for every refusal here: the action, what stopped it, and
/// what the person can do.
fn refusal(action: &'static str, subject: String, recovery: &'static str) -> AxError {
    AxError::failure(AxCode::ConfigInvalid, action, subject).with_recovery(recovery)
}

fn fill(bytes: &mut [u8], action: &'static str) -> Result<(), AxError> {
    getrandom::fill(bytes).map_err(|err| {
        refusal(
            action,
            format!("this machine's randomness is unavailable: {err}"),
            "retry; a credential file is not written from any other source of randomness",
        )
    })
}

fn derive(
    passphrase: &Zeroizing<String>,
    salt: &[u8; SALT_LEN],
) -> Result<Zeroizing<[u8; KEY_LEN]>, AxError> {
    let params = Params::new(MEMORY_KIB, TIME_COST, LANES, Some(KEY_LEN)).map_err(|err| {
        refusal(
            "derive the credential file's key",
            format!("the cost settings of this build are not accepted: {err}"),
            "report this: the settings are constants of the build and nothing on disk \
             can change them",
        )
    })?;
    let mut key = Zeroizing::new([0_u8; KEY_LEN]);
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
        .hash_password_into(passphrase.as_bytes(), salt, key.as_mut_slice())
        .map_err(|err| {
            refusal(
                "derive the credential file's key",
                format!("the passphrase could not be turned into a key: {err}"),
                "enter a passphrase of at least one character and try again",
            )
        })?;
    Ok(key)
}

fn decode_salt(encoded: &str) -> Result<[u8; SALT_LEN], AxError> {
    let raw = BASE64.decode(encoded).map_err(|err| {
        refusal(
            "read the credential file",
            format!("its salt is not base64: {err}"),
            START_OVER,
        )
    })?;
    raw.try_into().map_err(|_| {
        refusal(
            "read the credential file",
            "its salt is the wrong length".to_owned(),
            START_OVER,
        )
    })
}

fn read_if_present(path: &Path) -> Result<Option<Vec<u8>>, AxError> {
    match std::fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(refusal(
            "read the credential file",
            format!("it could not be read: {err}"),
            "check that the file is readable by the account the city runs as",
        )),
    }
}

/// Write bytes where only this account can read them.
///
/// On Unix the mode is set before the first byte, because a file created
/// at the default mode is world-readable for as long as it takes to
/// chmod it. Windows inherits the directory's ACL, which is the same
/// guarantee expressed by the platform.
fn write_private(path: &Path, bytes: &[u8]) -> Result<(), AxError> {
    use std::io::Write as _;

    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.mode(0o600);
    }
    let mut file = options.open(path).map_err(|err| {
        refusal(
            "write the credential file",
            format!("it could not be opened for writing: {err}"),
            "check that the directory holding the credential file is writable",
        )
    })?;
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|err| {
            refusal(
                "write the credential file",
                format!("its bytes did not reach the disk: {err}"),
                "check the free space on the disk holding the credential file, then \
                 store the credential again",
            )
        })
}

#[cfg(test)]
mod tests;
