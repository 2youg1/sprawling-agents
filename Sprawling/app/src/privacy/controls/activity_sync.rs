// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The activity history and cross-device controls, one row each, in page order
//! (`crates/sprawling/spec/Privacy/Controls.lean`).

use wire::{PrivacyBuildEffect as Effect, PrivacyCategory, PrivacyEdition as Edition};

use super::{Control, Editions, Written};
use crate::privacy::target::{Hive, Target};

pub(super) const START_LAUNCH_TRACKING: Control = Control {
    target: Target::Registry {
        hive: Hive::CurrentUser,
        path: r"SOFTWARE\Microsoft\Windows\CurrentVersion\Explorer\Advanced",
        name: "Start_TrackProgs",
    },
    written: Written::Dword(0),
    category: PrivacyCategory::ActivitySync,
    editions: Editions {
        honoured: &[Edition::Enterprise, Edition::Server],
        ignored: &[],
    },
    build_effect: Effect::Documented,
};

pub(super) const ACTIVITY_FEED: Control = Control {
    target: Target::Registry {
        hive: Hive::LocalMachine,
        path: r"SOFTWARE\Policies\Microsoft\Windows\System",
        name: "EnableActivityFeed",
    },
    written: Written::Dword(0),
    category: PrivacyCategory::ActivitySync,
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

pub(super) const ACTIVITY_PUBLISH: Control = Control {
    target: Target::Registry {
        hive: Hive::LocalMachine,
        path: r"SOFTWARE\Policies\Microsoft\Windows\System",
        name: "PublishUserActivities",
    },
    written: Written::Dword(0),
    category: PrivacyCategory::ActivitySync,
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

pub(super) const ACTIVITY_UPLOAD: Control = Control {
    target: Target::Registry {
        hive: Hive::LocalMachine,
        path: r"SOFTWARE\Policies\Microsoft\Windows\System",
        name: "UploadUserActivities",
    },
    written: Written::Dword(0),
    category: PrivacyCategory::ActivitySync,
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
    build_effect: Effect::NoCurrentEffect,
};

pub(super) const CLIPBOARD_HISTORY: Control = Control {
    target: Target::Registry {
        hive: Hive::LocalMachine,
        path: r"SOFTWARE\Policies\Microsoft\Windows\System",
        name: "AllowClipboardHistory",
    },
    written: Written::Dword(0),
    category: PrivacyCategory::ActivitySync,
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

pub(super) const CLOUD_CLIPBOARD: Control = Control {
    target: Target::Registry {
        hive: Hive::LocalMachine,
        path: r"SOFTWARE\Policies\Microsoft\Windows\System",
        name: "AllowCrossDeviceClipboard",
    },
    written: Written::Dword(0),
    category: PrivacyCategory::ActivitySync,
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

pub(super) const CONTINUE_EXPERIENCES: Control = Control {
    target: Target::Registry {
        hive: Hive::LocalMachine,
        path: r"SOFTWARE\Policies\Microsoft\Windows\System",
        name: "EnableCdp",
    },
    written: Written::Dword(0),
    category: PrivacyCategory::ActivitySync,
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

pub(super) const PHONE_PC_LINKING: Control = Control {
    target: Target::Registry {
        hive: Hive::LocalMachine,
        path: r"SOFTWARE\Policies\Microsoft\Windows\System",
        name: "EnableMmx",
    },
    written: Written::Dword(0),
    category: PrivacyCategory::ActivitySync,
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

pub(super) const MESSAGE_SYNC: Control = Control {
    target: Target::Registry {
        hive: Hive::LocalMachine,
        path: r"SOFTWARE\Policies\Microsoft\Windows\Messaging",
        name: "AllowMessageSync",
    },
    written: Written::Dword(0),
    category: PrivacyCategory::ActivitySync,
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
