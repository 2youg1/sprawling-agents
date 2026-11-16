// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

macro_rules! named_frames {
    (
        $(#[$enum_attr:meta])*
        pub enum $frame:ident $(<$carrier:ident = $carried:ty>)? {
            $(
                $(#[$variant_attr:meta])*
                $variant:ident $({ $($field:tt)* })? $(( $($carried_value:tt)* ))?
            ),* $(,)?
        }

        $(#[$names_attr:meta])*
        pub const $names:ident;
    ) => {
        $(#[$enum_attr])*
        pub enum $frame $(<$carrier = $carried>)? {
            $(
                $(#[$variant_attr])*
                $variant $({ $($field)* })? $(( $($carried_value)* ))?,
            )*
        }

        $(#[$names_attr])*
        pub const $names: [&str; [$(stringify!($variant)),*].len()] =
            [$(stringify!($variant)),*];

        impl $(<$carrier>)? $frame $(<$carrier>)? {

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
