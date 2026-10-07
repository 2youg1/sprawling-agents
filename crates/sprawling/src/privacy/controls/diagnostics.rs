// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The diagnostics and telemetry controls, one row each, in page order
//! (`crates/sprawling/spec/Privacy/Controls.lean`).

use wire::{PrivacyBuildEffect as Effect, PrivacyCategory, PrivacyEdition as Edition};

use super::{Control, Editions, Written};
use crate::privacy::target::{Hive, Target};

pub(super) const POWERSHELL_TELEMETRY_OPTOUT: Control = Control {
    target: Target::UserEnvironment {
        name: "POWERSHELL_TELEMETRY_OPTOUT",
    },
    written: Written::Text("1"),
    category: PrivacyCategory::Diagnostics,
    editions: Editions {
        honoured: &[
            Edition::Home,
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

pub(super) const CEIP_CONSOLIDATOR_TASK: Control = Control {
    target: Target::ScheduledTask {
        path: r"\Microsoft\Windows\Customer Experience Improvement Program\",
        name: "Consolidator",
    },
    written: Written::Disabled,
    category: PrivacyCategory::Diagnostics,
    editions: Editions {
        honoured: &[
            Edition::Home,
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

pub(super) const CEIP_USB_TASK: Control = Control {
    target: Target::ScheduledTask {
        path: r"\Microsoft\Windows\Customer Experience Improvement Program\",
        name: "UsbCeip",
    },
    written: Written::Disabled,
    category: PrivacyCategory::Diagnostics,
    editions: Editions {
        honoured: &[
            Edition::Home,
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

pub(super) const WINDOWS_CEIP: Control = Control {
    target: Target::Registry {
        hive: Hive::LocalMachine,
        path: r"SOFTWARE\Policies\Microsoft\SQMClient\Windows",
        name: "CEIPEnable",
    },
    written: Written::Dword(0),
    category: PrivacyCategory::Diagnostics,
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

pub(super) const DIAGNOSTIC_DATA: Control = Control {
    target: Target::Registry {
        hive: Hive::LocalMachine,
        path: r"SOFTWARE\Policies\Microsoft\Windows\DataCollection",
        name: "AllowTelemetry",
    },
    written: Written::Dword(0),
    category: PrivacyCategory::Diagnostics,
    editions: Editions {
        honoured: &[Edition::Enterprise, Edition::Education, Edition::Server],
        ignored: &[Edition::Home, Edition::Pro],
    },
    build_effect: Effect::Documented,
};

pub(super) const FEEDBACK_NOTIFICATIONS: Control = Control {
    target: Target::Registry {
        hive: Hive::LocalMachine,
        path: r"SOFTWARE\Policies\Microsoft\Windows\DataCollection",
        name: "DoNotShowFeedbackNotifications",
    },
    written: Written::Dword(1),
    category: PrivacyCategory::Diagnostics,
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

pub(super) const STEPS_RECORDER: Control = Control {
    target: Target::Registry {
        hive: Hive::LocalMachine,
        path: r"SOFTWARE\Policies\Microsoft\Windows\AppCompat",
        name: "DisableUAR",
    },
    written: Written::Dword(1),
    category: PrivacyCategory::Diagnostics,
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

pub(super) const ERROR_REPORTING: Control = Control {
    target: Target::Registry {
        hive: Hive::LocalMachine,
        path: r"SOFTWARE\Policies\Microsoft\Windows\Windows Error Reporting",
        name: "Disabled",
    },
    written: Written::Dword(1),
    category: PrivacyCategory::Diagnostics,
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

pub(super) const ERROR_REPORTING_ADDITIONAL_DATA: Control = Control {
    target: Target::Registry {
        hive: Hive::LocalMachine,
        path: r"SOFTWARE\Policies\Microsoft\Windows\Windows Error Reporting",
        name: "DontSendAdditionalData",
    },
    written: Written::Dword(1),
    category: PrivacyCategory::Diagnostics,
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

pub(super) const DIAGNOSTIC_LOG_COLLECTION: Control = Control {
    target: Target::Registry {
        hive: Hive::LocalMachine,
        path: r"SOFTWARE\Policies\Microsoft\Windows\DataCollection",
        name: "LimitDiagnosticLogCollection",
    },
    written: Written::Dword(1),
    category: PrivacyCategory::Diagnostics,
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

pub(super) const DUMP_COLLECTION: Control = Control {
    target: Target::Registry {
        hive: Hive::LocalMachine,
        path: r"SOFTWARE\Policies\Microsoft\Windows\DataCollection",
        name: "LimitDumpCollection",
    },
    written: Written::Dword(1),
    category: PrivacyCategory::Diagnostics,
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

pub(super) const APPLICATION_TELEMETRY: Control = Control {
    target: Target::Registry {
        hive: Hive::LocalMachine,
        path: r"SOFTWARE\Policies\Microsoft\Windows\AppCompat",
        name: "AITEnable",
    },
    written: Written::Dword(0),
    category: PrivacyCategory::Diagnostics,
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

pub(super) const DEVICE_CENSUS_TASK: Control = Control {
    target: Target::ScheduledTask {
        path: r"\Microsoft\Windows\Device Information\",
        name: "Device",
    },
    written: Written::Disabled,
    category: PrivacyCategory::Diagnostics,
    editions: Editions {
        honoured: &[
            Edition::Home,
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

pub(super) const DEVICE_CENSUS_USER_TASK: Control = Control {
    target: Target::ScheduledTask {
        path: r"\Microsoft\Windows\Device Information\",
        name: "Device User",
    },
    written: Written::Disabled,
    category: PrivacyCategory::Diagnostics,
    editions: Editions {
        honoured: &[
            Edition::Home,
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

pub(super) const DOTNET_CLI_TELEMETRY_OPTOUT: Control = Control {
    target: Target::UserEnvironment {
        name: "DOTNET_CLI_TELEMETRY_OPTOUT",
    },
    written: Written::Text("1"),
    category: PrivacyCategory::Diagnostics,
    editions: Editions {
        honoured: &[
            Edition::Home,
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
