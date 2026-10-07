// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The windows AI controls, one row each, in page order
//! (`crates/sprawling/spec/Privacy/Controls.lean`).

use wire::{PrivacyBuildEffect as Effect, PrivacyCategory, PrivacyEdition as Edition};

use super::{Control, Editions, Written};
use crate::privacy::target::{Hive, Target};

pub(super) const APP_SYSTEM_AI_MODELS: Control = Control {
    target: Target::Registry {
        hive: Hive::LocalMachine,
        path: r"SOFTWARE\Policies\Microsoft\Windows\AppPrivacy",
        name: "LetAppsAccessSystemAIModels",
    },
    written: Written::Dword(2),
    category: PrivacyCategory::WindowsAi,
    editions: Editions {
        honoured: &[],
        ignored: &[],
    },
    build_effect: Effect::Documented,
};

pub(super) const RECALL_SNAPSHOTS: Control = Control {
    target: Target::Registry {
        hive: Hive::LocalMachine,
        path: r"SOFTWARE\Policies\Microsoft\Windows\WindowsAI",
        name: "DisableAIDataAnalysis",
    },
    written: Written::Dword(1),
    category: PrivacyCategory::WindowsAi,
    editions: Editions {
        honoured: &[
            Edition::Pro,
            Edition::Enterprise,
            Edition::Education,
            Edition::IotEnterprise,
        ],
        ignored: &[Edition::Server],
    },
    build_effect: Effect::Documented,
};

pub(super) const RECALL_COMPONENT: Control = Control {
    target: Target::Registry {
        hive: Hive::LocalMachine,
        path: r"SOFTWARE\Policies\Microsoft\Windows\WindowsAI",
        name: "AllowRecallEnablement",
    },
    written: Written::Dword(0),
    category: PrivacyCategory::WindowsAi,
    editions: Editions {
        honoured: &[
            Edition::Pro,
            Edition::Enterprise,
            Edition::Education,
            Edition::IotEnterprise,
        ],
        ignored: &[Edition::Server],
    },
    build_effect: Effect::Documented,
};

pub(super) const CLICK_TO_DO: Control = Control {
    target: Target::Registry {
        hive: Hive::LocalMachine,
        path: r"SOFTWARE\Policies\Microsoft\Windows\WindowsAI",
        name: "DisableClickToDo",
    },
    written: Written::Dword(1),
    category: PrivacyCategory::WindowsAi,
    editions: Editions {
        honoured: &[
            Edition::Pro,
            Edition::Enterprise,
            Edition::Education,
            Edition::IotEnterprise,
        ],
        ignored: &[Edition::Server],
    },
    build_effect: Effect::Uncertain,
};
