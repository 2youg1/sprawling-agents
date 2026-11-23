// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#[test]
fn pwsh_is_handed_to_a_run_only_at_major_seven_or_later() {
    use crate::doctor::host::seven_or_later;
    use crate::doctor::{Presence, Version};
    let at = std::path::PathBuf::from("pwsh");
    let said = |text: &str| Presence::Present {
        at: at.clone(),
        version: Version::Said(text.to_owned()),
    };
    assert_eq!(seven_or_later(&said("PowerShell 7.4.6")), Some(at.clone()));
    assert_eq!(seven_or_later(&said("PowerShell 10.0.0")), Some(at.clone()));
    assert_eq!(seven_or_later(&said("PowerShell 6.2.7")), None);
    assert_eq!(
        seven_or_later(&Presence::Present {
            at: at.clone(),
            version: Version::Silent,
        }),
        None
    );
}

#[test]
fn python_is_handed_to_a_run_only_at_major_three() {
    use crate::doctor::host::three_only;
    use crate::doctor::{Presence, Version};
    let at = std::path::PathBuf::from("python");
    let said = |text: &str| Presence::Present {
        at: at.clone(),
        version: Version::Said(text.to_owned()),
    };
    assert_eq!(three_only(&said("Python 3.12.1")), Some(at.clone()));
    assert_eq!(three_only(&said("Python 3.14.0")), Some(at.clone()));
    assert_eq!(three_only(&said("Python 2.7.18")), None);
    assert_eq!(
        three_only(&Presence::Present {
            at: at.clone(),
            version: Version::Silent,
        }),
        None
    );
}
