// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The search controls, one row each, in page order
//! (`crates/sprawling/spec/Privacy/Controls.lean`).

use wire::{PrivacyBuildEffect as Effect, PrivacyCategory, PrivacyEdition as Edition};

use super::{Control, Editions, Written};
use crate::privacy::target::{Hive, Target};

pub(super) const SEARCH_WEB: Control = Control {
    target: Target::Registry {
        hive: Hive::LocalMachine,
        path: r"SOFTWARE\Policies\Microsoft\Windows\Windows Search",
        name: "DisableWebSearch",
    },
    written: Written::Dword(1),
    category: PrivacyCategory::Search,
    editions: Editions {
        honoured: &[],
        ignored: &[],
    },
    build_effect: Effect::Uncertain,
};

pub(super) const SEARCH_WEB_RESULTS: Control = Control {
    target: Target::Registry {
        hive: Hive::LocalMachine,
        path: r"SOFTWARE\Policies\Microsoft\Windows\Windows Search",
        name: "ConnectedSearchUseWeb",
    },
    written: Written::Dword(0),
    category: PrivacyCategory::Search,
    editions: Editions {
        honoured: &[
            Edition::Enterprise,
            Edition::Education,
            Edition::IotEnterprise,
        ],
        ignored: &[Edition::Pro],
    },
    build_effect: Effect::Documented,
};

pub(super) const CLOUD_SEARCH: Control = Control {
    target: Target::Registry {
        hive: Hive::LocalMachine,
        path: r"SOFTWARE\Policies\Microsoft\Windows\Windows Search",
        name: "AllowCloudSearch",
    },
    written: Written::Dword(0),
    category: PrivacyCategory::Search,
    editions: Editions {
        honoured: &[
            Edition::Pro,
            Edition::Enterprise,
            Edition::Education,
            Edition::IotEnterprise,
            Edition::Server,
        ],
        ignored: &[],
    },
    build_effect: Effect::Documented,
};

pub(super) const CORTANA: Control = Control {
    target: Target::Registry {
        hive: Hive::LocalMachine,
        path: r"SOFTWARE\Policies\Microsoft\Windows\Windows Search",
        name: "AllowCortana",
    },
    written: Written::Dword(0),
    category: PrivacyCategory::Search,
    editions: Editions {
        honoured: &[
            Edition::Pro,
            Edition::Enterprise,
            Edition::Education,
            Edition::IotEnterprise,
            Edition::Server,
        ],
        ignored: &[],
    },
    build_effect: Effect::Uncertain,
};

pub(super) const CORTANA_ABOVE_LOCK: Control = Control {
    target: Target::Registry {
        hive: Hive::LocalMachine,
        path: r"SOFTWARE\Policies\Microsoft\Windows\Windows Search",
        name: "AllowCortanaAboveLock",
    },
    written: Written::Dword(0),
    category: PrivacyCategory::Search,
    editions: Editions {
        honoured: &[
            Edition::Pro,
            Edition::Enterprise,
            Edition::Education,
            Edition::IotEnterprise,
            Edition::Server,
        ],
        ignored: &[],
    },
    build_effect: Effect::Uncertain,
};

pub(super) const SEARCH_LOCATION: Control = Control {
    target: Target::Registry {
        hive: Hive::LocalMachine,
        path: r"SOFTWARE\Policies\Microsoft\Windows\Windows Search",
        name: "AllowSearchToUseLocation",
    },
    written: Written::Dword(0),
    category: PrivacyCategory::Search,
    editions: Editions {
        honoured: &[
            Edition::Pro,
            Edition::Enterprise,
            Edition::Education,
            Edition::IotEnterprise,
            Edition::Server,
        ],
        ignored: &[],
    },
    build_effect: Effect::Documented,
};

pub(super) const SEARCH_HIGHLIGHTS: Control = Control {
    target: Target::Registry {
        hive: Hive::LocalMachine,
        path: r"SOFTWARE\Policies\Microsoft\Windows\Windows Search",
        name: "EnableDynamicContentInWSB",
    },
    written: Written::Dword(0),
    category: PrivacyCategory::Search,
    editions: Editions {
        honoured: &[
            Edition::Pro,
            Edition::Enterprise,
            Edition::Education,
            Edition::IotEnterprise,
            Edition::Server,
        ],
        ignored: &[],
    },
    build_effect: Effect::Documented,
};
