// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The app permissions controls, one row each, in page order
//! (`crates/sprawling/spec/Privacy/Controls.lean`).

use wire::{PrivacyBuildEffect as Effect, PrivacyCategory, PrivacyEdition as Edition};

use super::{Control, Editions, Written};
use crate::privacy::target::{Hive, Target};

pub(super) const APP_LOCATION: Control = Control {
    target: Target::Registry {
        hive: Hive::LocalMachine,
        path: r"SOFTWARE\Policies\Microsoft\Windows\AppPrivacy",
        name: "LetAppsAccessLocation",
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

pub(super) const APP_ACCOUNT_INFO: Control = Control {
    target: Target::Registry {
        hive: Hive::LocalMachine,
        path: r"SOFTWARE\Policies\Microsoft\Windows\AppPrivacy",
        name: "LetAppsAccessAccountInfo",
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

pub(super) const APP_MOTION: Control = Control {
    target: Target::Registry {
        hive: Hive::LocalMachine,
        path: r"SOFTWARE\Policies\Microsoft\Windows\AppPrivacy",
        name: "LetAppsAccessMotion",
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

pub(super) const APP_PHONE_CALLS: Control = Control {
    target: Target::Registry {
        hive: Hive::LocalMachine,
        path: r"SOFTWARE\Policies\Microsoft\Windows\AppPrivacy",
        name: "LetAppsAccessPhone",
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

pub(super) const APP_TRUSTED_DEVICES: Control = Control {
    target: Target::Registry {
        hive: Hive::LocalMachine,
        path: r"SOFTWARE\Policies\Microsoft\Windows\AppPrivacy",
        name: "LetAppsAccessTrustedDevices",
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

pub(super) const APP_UNPAIRED_DEVICES: Control = Control {
    target: Target::Registry {
        hive: Hive::LocalMachine,
        path: r"SOFTWARE\Policies\Microsoft\Windows\AppPrivacy",
        name: "LetAppsSyncWithDevices",
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

pub(super) const APP_DIAGNOSTIC_INFO: Control = Control {
    target: Target::Registry {
        hive: Hive::LocalMachine,
        path: r"SOFTWARE\Policies\Microsoft\Windows\AppPrivacy",
        name: "LetAppsGetDiagnosticInfo",
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

pub(super) const APP_CONTACTS: Control = Control {
    target: Target::Registry {
        hive: Hive::LocalMachine,
        path: r"SOFTWARE\Policies\Microsoft\Windows\AppPrivacy",
        name: "LetAppsAccessContacts",
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

pub(super) const APP_CALENDAR: Control = Control {
    target: Target::Registry {
        hive: Hive::LocalMachine,
        path: r"SOFTWARE\Policies\Microsoft\Windows\AppPrivacy",
        name: "LetAppsAccessCalendar",
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

pub(super) const APP_CALL_HISTORY: Control = Control {
    target: Target::Registry {
        hive: Hive::LocalMachine,
        path: r"SOFTWARE\Policies\Microsoft\Windows\AppPrivacy",
        name: "LetAppsAccessCallHistory",
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

pub(super) const APP_EMAIL: Control = Control {
    target: Target::Registry {
        hive: Hive::LocalMachine,
        path: r"SOFTWARE\Policies\Microsoft\Windows\AppPrivacy",
        name: "LetAppsAccessEmail",
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

pub(super) const APP_TASKS: Control = Control {
    target: Target::Registry {
        hive: Hive::LocalMachine,
        path: r"SOFTWARE\Policies\Microsoft\Windows\AppPrivacy",
        name: "LetAppsAccessTasks",
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

pub(super) const APP_MESSAGING: Control = Control {
    target: Target::Registry {
        hive: Hive::LocalMachine,
        path: r"SOFTWARE\Policies\Microsoft\Windows\AppPrivacy",
        name: "LetAppsAccessMessaging",
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

pub(super) const APP_RADIOS: Control = Control {
    target: Target::Registry {
        hive: Hive::LocalMachine,
        path: r"SOFTWARE\Policies\Microsoft\Windows\AppPrivacy",
        name: "LetAppsAccessRadios",
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

pub(super) const APP_CAMERA: Control = Control {
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

pub(super) const APP_MICROPHONE: Control = Control {
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

pub(super) const APP_NOTIFICATIONS: Control = Control {
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

pub(super) const APP_BACKGROUND: Control = Control {
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

pub(super) const APP_SCREEN_CAPTURE_PROGRAMMATIC: Control = Control {
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

pub(super) const APP_SCREEN_CAPTURE_BORDERLESS: Control = Control {
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

pub(super) const APP_GAZE_INPUT: Control = Control {
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

pub(super) const APP_HUMAN_PRESENCE: Control = Control {
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

pub(super) const APP_FOREGROUND_TEXT: Control = Control {
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

pub(super) const CAMERA_DEVICE: Control = Control {
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
