// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Money and quantities as integers (15.3-6), and what a piece of work
//! turned out to cost.
//!
//! There is no ceiling here and no decision that reads one. Nobody can
//! price a piece of work before it runs, so the only brake this city has
//! is `Halt`: it shuts a scope to new work and stops the background
//! members inside it. What survives is the reporting half — `BudgetUse`
//! is a payload field of `Progress::Unplanned` and of the evidence a
//! finished run carries, which is how a person is told what was spent
//! rather than told in advance what may be.

use serde::{Deserialize, Serialize};

/// One micro-USD. Decimal price lists convert at the single accounting
/// entry point (S3 gateway::cost); decisions never touch floats.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct UsdMicros(u64);

/// Whole tokens.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Tokens(u64);

/// Whole bytes.
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

            /// Overflow is `None`; the caller owns the verdict, which is
            /// `E_INVALID_ARGS` at every site that has one. Choosing an
            /// error story here would force it on every caller.
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
