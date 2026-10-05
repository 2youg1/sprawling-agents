// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Windows native exec, specified by
//! `crates/runtime/spec/Tools/Exec/NativeWindows.lean`.

#[cfg(test)]
mod tests {
    #[test]
    fn default_windows_native_admission_requires_all_five_axes() {
        use super::super::confinement::{Confinement, Guarantee, Kept};
        let chosen = Confinement::detect();
        for axis in Guarantee::ALL {
            assert_eq!(chosen.assurances().of(axis), Kept::Yes, "{axis:?}");
        }
    }
}
