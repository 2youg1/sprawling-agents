// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

mod activity_sync;
mod app_permissions;
mod cloud_services;
mod content;
mod diagnostics;
mod location_sensors;
mod search;
mod speech_input;
mod windows_ai;

use wire::{
    PrivacyBuildEffect, PrivacyCategory, PrivacyControl, PrivacyEdition, PrivacyEditionFit,
};

use super::target::{OperationKind, RawValue, Snapshot, Target, TaskState};

#[derive(Debug)]
pub(crate) struct Control {
    pub(crate) target: Target,
    pub(crate) written: Written,
    pub(crate) category: PrivacyCategory,
    pub(crate) editions: Editions,
    pub(crate) build_effect: PrivacyBuildEffect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Written {
    Dword(u32),

    Text(&'static str),

    Disabled,
}

#[derive(Debug)]
pub(crate) struct Editions {
    pub(crate) honoured: &'static [PrivacyEdition],
    pub(crate) ignored: &'static [PrivacyEdition],
}

pub(crate) const fn definition(control: PrivacyControl) -> &'static Control {
    match control {
        PrivacyControl::PowershellTelemetryOptout => &diagnostics::POWERSHELL_TELEMETRY_OPTOUT,
        PrivacyControl::CeipConsolidatorTask => &diagnostics::CEIP_CONSOLIDATOR_TASK,
        PrivacyControl::CeipUsbTask => &diagnostics::CEIP_USB_TASK,
        PrivacyControl::WindowsCeip => &diagnostics::WINDOWS_CEIP,
        PrivacyControl::DiagnosticData => &diagnostics::DIAGNOSTIC_DATA,
        PrivacyControl::FeedbackNotifications => &diagnostics::FEEDBACK_NOTIFICATIONS,
        PrivacyControl::StepsRecorder => &diagnostics::STEPS_RECORDER,
        PrivacyControl::ErrorReporting => &diagnostics::ERROR_REPORTING,
        PrivacyControl::ErrorReportingAdditionalData => {
            &diagnostics::ERROR_REPORTING_ADDITIONAL_DATA
        }
        PrivacyControl::DiagnosticLogCollection => &diagnostics::DIAGNOSTIC_LOG_COLLECTION,
        PrivacyControl::DumpCollection => &diagnostics::DUMP_COLLECTION,
        PrivacyControl::ApplicationTelemetry => &diagnostics::APPLICATION_TELEMETRY,
        PrivacyControl::DeviceCensusTask => &diagnostics::DEVICE_CENSUS_TASK,
        PrivacyControl::DeviceCensusUserTask => &diagnostics::DEVICE_CENSUS_USER_TASK,
        PrivacyControl::DotnetCliTelemetryOptout => &diagnostics::DOTNET_CLI_TELEMETRY_OPTOUT,
        PrivacyControl::VoiceActivation => &speech_input::VOICE_ACTIVATION,
        PrivacyControl::VoiceActivationAboveLock => &speech_input::VOICE_ACTIVATION_ABOVE_LOCK,
        PrivacyControl::OnlineSpeechRecognition => &speech_input::ONLINE_SPEECH_RECOGNITION,
        PrivacyControl::ImplicitTextCollection => &speech_input::IMPLICIT_TEXT_COLLECTION,
        PrivacyControl::ImplicitInkCollection => &speech_input::IMPLICIT_INK_COLLECTION,
        PrivacyControl::HandwritingErrorReports => &speech_input::HANDWRITING_ERROR_REPORTS,
        PrivacyControl::InputPersonalization => &speech_input::INPUT_PERSONALIZATION,
        PrivacyControl::LinguisticDataCollection => &speech_input::LINGUISTIC_DATA_COLLECTION,
        PrivacyControl::HandwritingDataSharing => &speech_input::HANDWRITING_DATA_SHARING,
        PrivacyControl::LocationProvider => &location_sensors::LOCATION_PROVIDER,
        PrivacyControl::LocationFeature => &location_sensors::LOCATION_FEATURE,
        PrivacyControl::Sensors => &location_sensors::SENSORS,
        PrivacyControl::SearchWeb => &search::SEARCH_WEB,
        PrivacyControl::SearchWebResults => &search::SEARCH_WEB_RESULTS,
        PrivacyControl::CloudSearch => &search::CLOUD_SEARCH,
        PrivacyControl::Cortana => &search::CORTANA,
        PrivacyControl::CortanaAboveLock => &search::CORTANA_ABOVE_LOCK,
        PrivacyControl::SearchLocation => &search::SEARCH_LOCATION,
        PrivacyControl::SearchHighlights => &search::SEARCH_HIGHLIGHTS,
        PrivacyControl::TailoredExperiences => &content::TAILORED_EXPERIENCES,
        PrivacyControl::Spotlight => &content::SPOTLIGHT,
        PrivacyControl::ConsumerFeatures => &content::CONSUMER_FEATURES,
        PrivacyControl::CloudOptimizedContent => &content::CLOUD_OPTIMIZED_CONTENT,
        PrivacyControl::WindowsTips => &content::WINDOWS_TIPS,
        PrivacyControl::ConsumerAccountStateContent => &content::CONSUMER_ACCOUNT_STATE_CONTENT,
        PrivacyControl::ThirdPartySuggestions => &content::THIRD_PARTY_SUGGESTIONS,
        PrivacyControl::SettingsOnlineTips => &content::SETTINGS_ONLINE_TIPS,
        PrivacyControl::Widgets => &content::WIDGETS,
        PrivacyControl::AdvertisingId => &content::ADVERTISING_ID,
        PrivacyControl::LanguageListWebsites => &content::LANGUAGE_LIST_WEBSITES,
        PrivacyControl::StartLaunchTracking => &activity_sync::START_LAUNCH_TRACKING,
        PrivacyControl::ActivityFeed => &activity_sync::ACTIVITY_FEED,
        PrivacyControl::ActivityPublish => &activity_sync::ACTIVITY_PUBLISH,
        PrivacyControl::ActivityUpload => &activity_sync::ACTIVITY_UPLOAD,
        PrivacyControl::ClipboardHistory => &activity_sync::CLIPBOARD_HISTORY,
        PrivacyControl::CloudClipboard => &activity_sync::CLOUD_CLIPBOARD,
        PrivacyControl::ContinueExperiences => &activity_sync::CONTINUE_EXPERIENCES,
        PrivacyControl::PhonePcLinking => &activity_sync::PHONE_PC_LINKING,
        PrivacyControl::MessageSync => &activity_sync::MESSAGE_SYNC,
        PrivacyControl::OnesettingsDownloads => &cloud_services::ONESETTINGS_DOWNLOADS,
        PrivacyControl::FindMyDevice => &cloud_services::FIND_MY_DEVICE,
        PrivacyControl::DeliveryOptimization => &cloud_services::DELIVERY_OPTIMIZATION,
        PrivacyControl::OnedriveFileStorage => &cloud_services::ONEDRIVE_FILE_STORAGE,
        PrivacyControl::OnedrivePreSigninTraffic => &cloud_services::ONEDRIVE_PRE_SIGNIN_TRAFFIC,
        PrivacyControl::PushNotificationsNetwork => &cloud_services::PUSH_NOTIFICATIONS_NETWORK,
        PrivacyControl::AppLocation => &app_permissions::APP_LOCATION,
        PrivacyControl::AppAccountInfo => &app_permissions::APP_ACCOUNT_INFO,
        PrivacyControl::AppMotion => &app_permissions::APP_MOTION,
        PrivacyControl::AppPhoneCalls => &app_permissions::APP_PHONE_CALLS,
        PrivacyControl::AppTrustedDevices => &app_permissions::APP_TRUSTED_DEVICES,
        PrivacyControl::AppUnpairedDevices => &app_permissions::APP_UNPAIRED_DEVICES,
        PrivacyControl::AppDiagnosticInfo => &app_permissions::APP_DIAGNOSTIC_INFO,
        PrivacyControl::AppContacts => &app_permissions::APP_CONTACTS,
        PrivacyControl::AppCalendar => &app_permissions::APP_CALENDAR,
        PrivacyControl::AppCallHistory => &app_permissions::APP_CALL_HISTORY,
        PrivacyControl::AppEmail => &app_permissions::APP_EMAIL,
        PrivacyControl::AppTasks => &app_permissions::APP_TASKS,
        PrivacyControl::AppMessaging => &app_permissions::APP_MESSAGING,
        PrivacyControl::AppRadios => &app_permissions::APP_RADIOS,
        PrivacyControl::AppCamera => &app_permissions::APP_CAMERA,
        PrivacyControl::AppMicrophone => &app_permissions::APP_MICROPHONE,
        PrivacyControl::AppNotifications => &app_permissions::APP_NOTIFICATIONS,
        PrivacyControl::AppBackground => &app_permissions::APP_BACKGROUND,
        PrivacyControl::AppScreenCaptureProgrammatic => {
            &app_permissions::APP_SCREEN_CAPTURE_PROGRAMMATIC
        }
        PrivacyControl::AppScreenCaptureBorderless => {
            &app_permissions::APP_SCREEN_CAPTURE_BORDERLESS
        }
        PrivacyControl::AppGazeInput => &app_permissions::APP_GAZE_INPUT,
        PrivacyControl::AppHumanPresence => &app_permissions::APP_HUMAN_PRESENCE,
        PrivacyControl::AppForegroundText => &app_permissions::APP_FOREGROUND_TEXT,
        PrivacyControl::CameraDevice => &app_permissions::CAMERA_DEVICE,
        PrivacyControl::AppSystemAiModels => &windows_ai::APP_SYSTEM_AI_MODELS,
        PrivacyControl::RecallSnapshots => &windows_ai::RECALL_SNAPSHOTS,
        PrivacyControl::RecallComponent => &windows_ai::RECALL_COMPONENT,
        PrivacyControl::ClickToDo => &windows_ai::CLICK_TO_DO,
    }
}

impl Written {
    pub(crate) fn target(self, current: &Snapshot) -> Option<Snapshot> {
        match self {
            Self::Dword(value) => Some(Snapshot::Registry(RawValue::dword(value))),
            Self::Text(text) => Some(Snapshot::Registry(RawValue::text(text))),
            Self::Disabled => match current {
                Snapshot::Task(
                    TaskState::Enabled { definition_sha256 }
                    | TaskState::Disabled { definition_sha256 },
                ) => Some(Snapshot::Task(TaskState::Disabled {
                    definition_sha256: *definition_sha256,
                })),
                Snapshot::Task(TaskState::Absent) | Snapshot::Registry(_) => None,
            },
        }
    }
}

impl Editions {
    pub(crate) fn fit(&self, edition: Option<PrivacyEdition>) -> PrivacyEditionFit {
        match edition {
            Some(edition) if self.honoured.contains(&edition) => PrivacyEditionFit::Honoured,
            Some(edition) if self.ignored.contains(&edition) => PrivacyEditionFit::Ignored,
            Some(_) | None => PrivacyEditionFit::NotStated,
        }
    }
}

struct Tally {
    hklm: usize,
    hkcu: usize,
    environment: usize,
    task: usize,

    suited: bool,
}

const fn tally() -> Tally {
    let mut tally = Tally {
        hklm: 0,
        hkcu: 0,
        environment: 0,
        task: 0,
        suited: true,
    };
    let mut rest: &[PrivacyControl] = &PrivacyControl::ALL;
    while let [control, tail @ ..] = rest {
        let row = definition(*control);
        let suits = match row.target.kind() {
            OperationKind::RegistryValueHklm => {
                tally.hklm = tally.hklm.saturating_add(1);
                matches!(row.written, Written::Dword(_))
            }
            OperationKind::RegistryValueHkcu => {
                tally.hkcu = tally.hkcu.saturating_add(1);
                matches!(row.written, Written::Dword(_))
            }
            OperationKind::EnvironmentVariableUser => {
                tally.environment = tally.environment.saturating_add(1);
                matches!(row.written, Written::Text(_))
            }
            OperationKind::ScheduledTaskEnabled => {
                tally.task = tally.task.saturating_add(1);
                matches!(row.written, Written::Disabled)
            }
        };
        tally.suited = tally.suited && suits;
        rest = tail;
    }
    tally
}

const TALLY: Tally = tally();
const _: () = assert!(PrivacyControl::ALL.len() == 88);
const _: () = assert!(TALLY.hklm == 74 && TALLY.hkcu == 8);
const _: () = assert!(TALLY.environment == 2 && TALLY.task == 4);
const _: () = assert!(TALLY.suited);

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing, reason = "test code")]
mod tests {
    use super::*;

    #[test]
    fn controls_run_in_category_order_with_disjoint_edition_lists() {
        let places: Vec<usize> = PrivacyControl::ALL
            .iter()
            .map(|control| {
                PrivacyCategory::ALL
                    .iter()
                    .position(|category| *category == definition(*control).category)
                    .unwrap()
            })
            .collect();
        assert!(places.is_sorted());
        for control in PrivacyControl::ALL {
            let editions = &definition(control).editions;
            assert!(
                editions
                    .honoured
                    .iter()
                    .all(|edition| !editions.ignored.contains(edition)),
                "{control:?}"
            );
        }
    }

    #[test]
    fn edition_fit_reads_the_lists_and_an_unnamed_edition_is_not_stated() {
        let editions = &definition(PrivacyControl::DiagnosticData).editions;
        assert_eq!(
            [
                Some(PrivacyEdition::Home),
                Some(PrivacyEdition::Enterprise),
                Some(PrivacyEdition::IotEnterprise),
                None,
            ]
            .map(|edition| editions.fit(edition)),
            [
                PrivacyEditionFit::Ignored,
                PrivacyEditionFit::Honoured,
                PrivacyEditionFit::NotStated,
                PrivacyEditionFit::NotStated,
            ]
        );
    }

    #[test]
    fn written_values_keep_raw_encoding_and_task_definition() {
        let definition_sha256 = [7; 32];
        let absent = Snapshot::Registry(RawValue::Absent);
        assert_eq!(
            [
                Written::Dword(99).target(&absent),
                Written::Text("1").target(&absent),
                Written::Disabled.target(&Snapshot::Task(TaskState::Enabled { definition_sha256 })),
                Written::Disabled.target(&Snapshot::Task(TaskState::Absent)),
            ],
            [
                Some(Snapshot::Registry(RawValue::Present {
                    kind: 4,
                    bytes: vec![99, 0, 0, 0],
                })),
                Some(Snapshot::Registry(RawValue::Present {
                    kind: 1,
                    bytes: vec![49, 0, 0, 0],
                })),
                Some(Snapshot::Task(TaskState::Disabled { definition_sha256 })),
                None,
            ]
        );
    }
}
