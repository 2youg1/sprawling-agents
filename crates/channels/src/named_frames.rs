// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One declaration of a frame family, three things generated from it:
//! the enum, the name of each variant, and the table the schema hash is
//! built from.
//!
//! **A frame's spelling used to have three homes.** The variant said
//! `RunHistory`, a hand-written `match` arm said `"RunHistory"`, and a
//! hand-written array said `"RunHistory"` a third time; the handshake
//! hash read only the array. An exhaustive `match` stops a new variant
//! from compiling without a name, and stops nothing at all about a new
//! variant missing from the array — so the array was guarded by a test
//! that listed every variant a fourth time, and by two counts written
//! as literals. This macro deletes all of that: the variant list is the
//! only place a frame is spelled, and the table cannot disagree with it
//! because the table is the list.
//!
//! **The table is in declaration order**, which is the order the
//! handshake hash mixes the names in. Moving a variant therefore
//! changes the hash and needs a `WIRE_V` bump, exactly as adding one
//! does.
//!
//! One macro rather than two, because `Query` and `Command` differ in
//! their generic carrier and in nothing else that matters here —
//! `carried_name` already takes this shape for four newtypes.

/// Declares a frame enum together with its name table.
///
/// The optional `<Carrier = Default>` clause carries `Command`'s secret
/// parameter; `Query` has none and omits it.
macro_rules! named_frames {
    (
        $(#[$enum_attr:meta])*
        pub enum $frame:ident $(<$carrier:ident = $carried:ty>)? {
            $(
                $(#[$variant_attr:meta])*
                $variant:ident $({ $($field:tt)* })?
            ),* $(,)?
        }

        $(#[$names_attr:meta])*
        pub const $names:ident;
    ) => {
        $(#[$enum_attr])*
        pub enum $frame $(<$carrier = $carried>)? {
            $(
                $(#[$variant_attr])*
                $variant $({ $($field)* })?,
            )*
        }

        $(#[$names_attr])*
        pub const $names: [&str; [$(stringify!($variant)),*].len()] =
            [$(stringify!($variant)),*];

        impl $(<$carrier>)? $frame $(<$carrier>)? {
            /// The wire name of this frame: the variant's own spelling,
            /// which is also its entry in the name table above.
            #[must_use]
            pub fn name(&self) -> &'static str {
                match *self {
                    $(Self::$variant { .. } => stringify!($variant),)*
                }
            }
        }
    };
}

pub(crate) use named_frames;
