// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use wire::{PrivacyBuildEffect as Effect, PrivacyCategory, PrivacyEdition as Edition};

use super::super::{Control, Editions, Written};
use crate::privacy::target::{Hive, Target};

pub(crate) const APP_CAMERA: Control = Control {
    target: Target::Registry {
        hive: Hive::LocalMachine,
        path: r"SOFTWARE\Policies\Microsoft\Windows\AppPrivacy",
        name: "LetAppsAccessCamera",
    },
    written: Written::Dword(2),
    category: PrivacyCategory::AppPermissions,
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

pub(crate) const APP_MICROPHONE: Control = Control {
    target: Target::Registry {
        hive: Hive::LocalMachine,
        path: r"SOFTWARE\Policies\Microsoft\Windows\AppPrivacy",
        name: "LetAppsAccessMicrophone",
    },
    written: Written::Dword(2),
    category: PrivacyCategory::AppPermissions,
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

pub(crate) const APP_NOTIFICATIONS: Control = Control {
    target: Target::Registry {
        hive: Hive::LocalMachine,
        path: r"SOFTWARE\Policies\Microsoft\Windows\AppPrivacy",
        name: "LetAppsAccessNotifications",
    },
    written: Written::Dword(2),
    category: PrivacyCategory::AppPermissions,
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

pub(crate) const APP_BACKGROUND: Control = Control {
    target: Target::Registry {
        hive: Hive::LocalMachine,
        path: r"SOFTWARE\Policies\Microsoft\Windows\AppPrivacy",
        name: "LetAppsRunInBackground",
    },
    written: Written::Dword(2),
    category: PrivacyCategory::AppPermissions,
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

pub(crate) const APP_SCREEN_CAPTURE_PROGRAMMATIC: Control = Control {
    target: Target::Registry {
        hive: Hive::LocalMachine,
        path: r"SOFTWARE\Policies\Microsoft\Windows\AppPrivacy",
        name: "LetAppsAccessGraphicsCaptureProgrammatic",
    },
    written: Written::Dword(2),
    category: PrivacyCategory::AppPermissions,
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

pub(crate) const APP_SCREEN_CAPTURE_BORDERLESS: Control = Control {
    target: Target::Registry {
        hive: Hive::LocalMachine,
        path: r"SOFTWARE\Policies\Microsoft\Windows\AppPrivacy",
        name: "LetAppsAccessGraphicsCaptureWithoutBorder",
    },
    written: Written::Dword(2),
    category: PrivacyCategory::AppPermissions,
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

pub(crate) const APP_GAZE_INPUT: Control = Control {
    target: Target::Registry {
        hive: Hive::LocalMachine,
        path: r"SOFTWARE\Policies\Microsoft\Windows\AppPrivacy",
        name: "LetAppsAccessGazeInput",
    },
    written: Written::Dword(2),
    category: PrivacyCategory::AppPermissions,
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

pub(crate) const APP_HUMAN_PRESENCE: Control = Control {
    target: Target::Registry {
        hive: Hive::LocalMachine,
        path: r"SOFTWARE\Policies\Microsoft\Windows\AppPrivacy",
        name: "LetAppsAccessHumanPresence",
    },
    written: Written::Dword(2),
    category: PrivacyCategory::AppPermissions,
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

pub(crate) const APP_FOREGROUND_TEXT: Control = Control {
    target: Target::Registry {
        hive: Hive::LocalMachine,
        path: r"SOFTWARE\Policies\Microsoft\Windows\AppPrivacy",
        name: "LetAppsAccessForegroundText",
    },
    written: Written::Dword(2),
    category: PrivacyCategory::AppPermissions,
    editions: Editions {
        honoured: &[
            Edition::Pro,
            Edition::Enterprise,
            Edition::Education,
            Edition::IotEnterprise,
        ],
        ignored: &[],
    },
    build_effect: Effect::Documented,
};

pub(crate) const CAMERA_DEVICE: Control = Control {
    target: Target::Registry {
        hive: Hive::LocalMachine,
        path: r"SOFTWARE\Policies\Microsoft\Camera",
        name: "AllowCamera",
    },
    written: Written::Dword(0),
    category: PrivacyCategory::AppPermissions,
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
