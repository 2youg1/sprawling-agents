// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The cloud services and network connections controls, one row each, in page order
//! (`crates/sprawling/spec/Privacy/Controls.lean`).

use wire::{PrivacyBuildEffect as Effect, PrivacyCategory, PrivacyEdition as Edition};

use super::{Control, Editions, Written};
use crate::privacy::target::{Hive, Target};

pub(super) const ONESETTINGS_DOWNLOADS: Control = Control {
    target: Target::Registry {
        hive: Hive::LocalMachine,
        path: r"SOFTWARE\Policies\Microsoft\Windows\DataCollection",
        name: "DisableOneSettingsDownloads",
    },
    written: Written::Dword(1),
    category: PrivacyCategory::CloudServices,
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

pub(super) const FIND_MY_DEVICE: Control = Control {
    target: Target::Registry {
        hive: Hive::LocalMachine,
        path: r"SOFTWARE\Policies\Microsoft\FindMyDevice",
        name: "AllowFindMyDevice",
    },
    written: Written::Dword(0),
    category: PrivacyCategory::CloudServices,
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

pub(super) const DELIVERY_OPTIMIZATION: Control = Control {
    target: Target::Registry {
        hive: Hive::LocalMachine,
        path: r"SOFTWARE\Policies\Microsoft\Windows\DeliveryOptimization",
        name: "DODownloadMode",
    },
    written: Written::Dword(99),
    category: PrivacyCategory::CloudServices,
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

pub(super) const ONEDRIVE_FILE_STORAGE: Control = Control {
    target: Target::Registry {
        hive: Hive::LocalMachine,
        path: r"SOFTWARE\Policies\Microsoft\Windows\OneDrive",
        name: "DisableFileSyncNGSC",
    },
    written: Written::Dword(1),
    category: PrivacyCategory::CloudServices,
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

pub(super) const ONEDRIVE_PRE_SIGNIN_TRAFFIC: Control = Control {
    target: Target::Registry {
        hive: Hive::LocalMachine,
        path: r"SOFTWARE\Microsoft\OneDrive",
        name: "PreventNetworkTrafficPreUserSignIn",
    },
    written: Written::Dword(1),
    category: PrivacyCategory::CloudServices,
    editions: Editions {
        honoured: &[Edition::Server],
        ignored: &[],
    },
    build_effect: Effect::Documented,
};

pub(super) const PUSH_NOTIFICATIONS_NETWORK: Control = Control {
    target: Target::Registry {
        hive: Hive::LocalMachine,
        path: r"SOFTWARE\Policies\Microsoft\Windows\CurrentVersion\PushNotifications",
        name: "NoCloudApplicationNotification",
    },
    written: Written::Dword(1),
    category: PrivacyCategory::CloudServices,
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
