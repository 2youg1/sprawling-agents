// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The suggestions, advertising and cloud content controls, one row each, in page order
//! (`crates/sprawling/spec/Privacy/Controls.lean`).

use wire::{PrivacyBuildEffect as Effect, PrivacyCategory, PrivacyEdition as Edition};

use super::{Control, Editions, Written};
use crate::privacy::target::{Hive, Target};

pub(super) const TAILORED_EXPERIENCES: Control = Control {
    target: Target::Registry {
        hive: Hive::CurrentUser,
        path: r"SOFTWARE\Policies\Microsoft\Windows\CloudContent",
        name: "DisableTailoredExperiencesWithDiagnosticData",
    },
    written: Written::Dword(1),
    category: PrivacyCategory::Content,
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

pub(super) const SPOTLIGHT: Control = Control {
    target: Target::Registry {
        hive: Hive::CurrentUser,
        path: r"SOFTWARE\Policies\Microsoft\Windows\CloudContent",
        name: "DisableWindowsSpotlightFeatures",
    },
    written: Written::Dword(1),
    category: PrivacyCategory::Content,
    editions: Editions {
        honoured: &[
            Edition::Enterprise,
            Edition::Education,
            Edition::IotEnterprise,
        ],
        ignored: &[Edition::Pro, Edition::Server],
    },
    build_effect: Effect::Documented,
};

pub(super) const CONSUMER_FEATURES: Control = Control {
    target: Target::Registry {
        hive: Hive::LocalMachine,
        path: r"SOFTWARE\Policies\Microsoft\Windows\CloudContent",
        name: "DisableWindowsConsumerFeatures",
    },
    written: Written::Dword(1),
    category: PrivacyCategory::Content,
    editions: Editions {
        honoured: &[
            Edition::Enterprise,
            Edition::Education,
            Edition::IotEnterprise,
        ],
        ignored: &[Edition::Home, Edition::Pro, Edition::Server],
    },
    build_effect: Effect::Documented,
};

pub(super) const CLOUD_OPTIMIZED_CONTENT: Control = Control {
    target: Target::Registry {
        hive: Hive::LocalMachine,
        path: r"SOFTWARE\Policies\Microsoft\Windows\CloudContent",
        name: "DisableCloudOptimizedContent",
    },
    written: Written::Dword(1),
    category: PrivacyCategory::Content,
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

pub(super) const WINDOWS_TIPS: Control = Control {
    target: Target::Registry {
        hive: Hive::LocalMachine,
        path: r"SOFTWARE\Policies\Microsoft\Windows\CloudContent",
        name: "DisableSoftLanding",
    },
    written: Written::Dword(1),
    category: PrivacyCategory::Content,
    editions: Editions {
        honoured: &[
            Edition::Enterprise,
            Edition::Education,
            Edition::IotEnterprise,
        ],
        ignored: &[Edition::Home, Edition::Pro, Edition::Server],
    },
    build_effect: Effect::Documented,
};

pub(super) const CONSUMER_ACCOUNT_STATE_CONTENT: Control = Control {
    target: Target::Registry {
        hive: Hive::LocalMachine,
        path: r"SOFTWARE\Policies\Microsoft\Windows\CloudContent",
        name: "DisableConsumerAccountStateContent",
    },
    written: Written::Dword(1),
    category: PrivacyCategory::Content,
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

pub(super) const THIRD_PARTY_SUGGESTIONS: Control = Control {
    target: Target::Registry {
        hive: Hive::CurrentUser,
        path: r"SOFTWARE\Policies\Microsoft\Windows\CloudContent",
        name: "DisableThirdPartySuggestions",
    },
    written: Written::Dword(1),
    category: PrivacyCategory::Content,
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

pub(super) const SETTINGS_ONLINE_TIPS: Control = Control {
    target: Target::Registry {
        hive: Hive::LocalMachine,
        path: r"SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\Explorer",
        name: "AllowOnlineTips",
    },
    written: Written::Dword(0),
    category: PrivacyCategory::Content,
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

pub(super) const WIDGETS: Control = Control {
    target: Target::Registry {
        hive: Hive::LocalMachine,
        path: r"SOFTWARE\Policies\Microsoft\Dsh",
        name: "AllowNewsAndInterests",
    },
    written: Written::Dword(0),
    category: PrivacyCategory::Content,
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

pub(super) const ADVERTISING_ID: Control = Control {
    target: Target::Registry {
        hive: Hive::LocalMachine,
        path: r"SOFTWARE\Policies\Microsoft\Windows\AdvertisingInfo",
        name: "DisabledByGroupPolicy",
    },
    written: Written::Dword(1),
    category: PrivacyCategory::Content,
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

pub(super) const LANGUAGE_LIST_WEBSITES: Control = Control {
    target: Target::Registry {
        hive: Hive::CurrentUser,
        path: r"Control Panel\International\User Profile",
        name: "HttpAcceptLanguageOptOut",
    },
    written: Written::Dword(1),
    category: PrivacyCategory::Content,
    editions: Editions {
        honoured: &[Edition::Enterprise, Edition::Server],
        ignored: &[],
    },
    build_effect: Effect::Documented,
};
