// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct UsdMicros(u64);

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Tokens(u64);

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ByteLen(u64);

macro_rules! quantity {
    ($name:ident) => {
        impl $name {
            pub const fn new(value: u64) -> $name {
                $name(value)
            }

            pub const fn get(self) -> u64 {
                self.0
            }

            pub fn checked_add(self, other: $name) -> Option<$name> {
                self.0.checked_add(other.0).map($name)
            }
        }
    };
}

quantity!(UsdMicros);
quantity!(Tokens);
quantity!(ByteLen);

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct BudgetUse {
    pub usd: UsdMicros,
    pub tokens: Tokens,
}
