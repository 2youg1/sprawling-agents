// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The carrier a socket cannot fill.
//!
//! `Command` is generic over what carries a secret. Fixing that carrier
//! to a type with no values is what makes `WireCommand::PutSecret`
//! unrepresentable rather than merely refused, and the three impls here
//! say the same thing to the three readers that ask: the compiler, a
//! stream of bytes, and a schema a client is generated from.

use serde::{Deserialize, Serialize};

/// Uninhabited on purpose. A value of this type cannot be produced, so
/// `Command<NoSecret>` has no reachable `PutSecret` variant. This is the
/// compile-time half of "a remote connection cannot spell that frame";
/// `Deserialize` supplies the runtime half for bytes that try anyway.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoSecret {}

impl Serialize for NoSecret {
    fn serialize<S: serde::Serializer>(&self, _serializer: S) -> Result<S::Ok, S::Error> {
        match *self {}
    }
}

impl<'de> Deserialize<'de> for NoSecret {
    fn deserialize<D: serde::Deserializer<'de>>(_deserializer: D) -> Result<Self, D::Error> {
        Err(serde::de::Error::custom(
            "a credential cannot be enrolled over a connection; enrol it on the host",
        ))
    }
}

/// The schema half of the same statement: `false` is the schema no value
/// satisfies, so a client generated from it types the field as `never`.
#[cfg(feature = "schema")]
impl schemars::JsonSchema for NoSecret {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Borrowed("NoSecret")
    }

    fn json_schema(_generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
        schemars::Schema::from(false)
    }
}
