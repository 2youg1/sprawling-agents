// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The speech, typing and inking controls, one row each, in page order
//! (`crates/sprawling/spec/Privacy/Controls.lean`).

use wire::{PrivacyBuildEffect as Effect, PrivacyCategory, PrivacyEdition as Edition};

use super::{Control, Editions, Written};
use crate::privacy::target::{Hive, Target};

pub(super) const VOICE_ACTIVATION: Control = Control {
    target: Target::Registry {
        hive: Hive::LocalMachine,
        path: r"SOFTWARE\Policies\Microsoft\Windows\AppPrivacy",
        name: "LetAppsActivateWithVoice",
    },
    written: Written::Dword(2),
    category: PrivacyCategory::SpeechInput,
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

pub(super) const VOICE_ACTIVATION_ABOVE_LOCK: Control = Control {
    target: Target::Registry {
        hive: Hive::LocalMachine,
        path: r"SOFTWARE\Policies\Microsoft\Windows\AppPrivacy",
        name: "LetAppsActivateWithVoiceAboveLock",
    },
    written: Written::Dword(2),
    category: PrivacyCategory::SpeechInput,
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

pub(super) const ONLINE_SPEECH_RECOGNITION: Control = Control {
    target: Target::Registry {
        hive: Hive::CurrentUser,
        path: r"Software\Microsoft\Speech_OneCore\Settings\OnlineSpeechPrivacy",
        name: "HasAccepted",
    },
    written: Written::Dword(0),
    category: PrivacyCategory::SpeechInput,
    editions: Editions {
        honoured: &[Edition::Enterprise, Edition::Server],
        ignored: &[],
    },
    build_effect: Effect::Documented,
};

pub(super) const IMPLICIT_TEXT_COLLECTION: Control = Control {
    target: Target::Registry {
        hive: Hive::CurrentUser,
        path: r"Software\Microsoft\InputPersonalization",
        name: "RestrictImplicitTextCollection",
    },
    written: Written::Dword(1),
    category: PrivacyCategory::SpeechInput,
    editions: Editions {
        honoured: &[Edition::Enterprise, Edition::Server],
        ignored: &[],
    },
    build_effect: Effect::Documented,
};

pub(super) const IMPLICIT_INK_COLLECTION: Control = Control {
    target: Target::Registry {
        hive: Hive::CurrentUser,
        path: r"Software\Microsoft\InputPersonalization",
        name: "RestrictImplicitInkCollection",
    },
    written: Written::Dword(1),
    category: PrivacyCategory::SpeechInput,
    editions: Editions {
        honoured: &[Edition::Enterprise, Edition::Server],
        ignored: &[],
    },
    build_effect: Effect::Documented,
};

pub(super) const HANDWRITING_ERROR_REPORTS: Control = Control {
    target: Target::Registry {
        hive: Hive::LocalMachine,
        path: r"SOFTWARE\Policies\Microsoft\Windows\HandwritingErrorReports",
        name: "PreventHandwritingErrorReports",
    },
    written: Written::Dword(1),
    category: PrivacyCategory::SpeechInput,
    editions: Editions {
        honoured: &[],
        ignored: &[],
    },
    build_effect: Effect::Uncertain,
};

pub(super) const INPUT_PERSONALIZATION: Control = Control {
    target: Target::Registry {
        hive: Hive::LocalMachine,
        path: r"SOFTWARE\Policies\Microsoft\InputPersonalization",
        name: "AllowInputPersonalization",
    },
    written: Written::Dword(0),
    category: PrivacyCategory::SpeechInput,
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

pub(super) const LINGUISTIC_DATA_COLLECTION: Control = Control {
    target: Target::Registry {
        hive: Hive::LocalMachine,
        path: r"SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\TextInput",
        name: "AllowLinguisticDataCollection",
    },
    written: Written::Dword(0),
    category: PrivacyCategory::SpeechInput,
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

pub(super) const HANDWRITING_DATA_SHARING: Control = Control {
    target: Target::Registry {
        hive: Hive::LocalMachine,
        path: r"SOFTWARE\Policies\Microsoft\Windows\TabletPC",
        name: "PreventHandwritingDataSharing",
    },
    written: Written::Dword(1),
    category: PrivacyCategory::SpeechInput,
    editions: Editions {
        honoured: &[],
        ignored: &[],
    },
    build_effect: Effect::Uncertain,
};
