// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

mod answer;
mod operation;

pub use answer::{PrivacyAnswer, PrivacyControlEntry, PrivacyCurrent, PrivacyEditions};
pub use answer::{PrivacyHistory, PrivacyHost, PrivacyIntent, PrivacyNotWrittenEntry};
pub use answer::{PrivacyOriginalLine, PrivacyTarget, PrivacyValue, PrivacyWindows};
pub use operation::{PrivacyAction, PrivacyOutcome, PrivacyRequest, PrivacyResult};

use serde::{Deserialize, Serialize};

macro_rules! listed {
    (
        $(#[$attr:meta])*
        pub enum $name:ident { $($(#[$member:meta])* $variant:ident),* $(,)? }
    ) => {
        $(#[$attr])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(rename_all = "snake_case")]
        #[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
        pub enum $name { $($(#[$member])* $variant),* }

        impl $name {

            pub const ALL: [Self; [$(stringify!($variant)),*].len()] = [$(Self::$variant),*];
        }
    };
}

listed! {

    pub enum PrivacyControl {

        PowershellTelemetryOptout,
        CeipConsolidatorTask,
        CeipUsbTask,
        WindowsCeip,
        DiagnosticData,
        FeedbackNotifications,
        StepsRecorder,
        ErrorReporting,
        ErrorReportingAdditionalData,
        DiagnosticLogCollection,
        DumpCollection,
        ApplicationTelemetry,
        DeviceCensusTask,
        DeviceCensusUserTask,
        DotnetCliTelemetryOptout,

        VoiceActivation,
        VoiceActivationAboveLock,
        OnlineSpeechRecognition,
        ImplicitTextCollection,
        ImplicitInkCollection,
        HandwritingErrorReports,
        InputPersonalization,
        LinguisticDataCollection,
        HandwritingDataSharing,

        LocationProvider,
        LocationFeature,
        Sensors,

        SearchWeb,
        SearchWebResults,
        CloudSearch,
        Cortana,
        CortanaAboveLock,
        SearchLocation,
        SearchHighlights,

        TailoredExperiences,
        Spotlight,
        ConsumerFeatures,
        CloudOptimizedContent,
        WindowsTips,
        ConsumerAccountStateContent,
        ThirdPartySuggestions,
        SettingsOnlineTips,
        Widgets,
        AdvertisingId,
        LanguageListWebsites,

        StartLaunchTracking,
        ActivityFeed,
        ActivityPublish,
        ActivityUpload,
        ClipboardHistory,
        CloudClipboard,
        ContinueExperiences,
        PhonePcLinking,
        MessageSync,

        OnesettingsDownloads,
        FindMyDevice,
        DeliveryOptimization,
        OnedriveFileStorage,
        OnedrivePreSigninTraffic,
        PushNotificationsNetwork,

        AppLocation,
        AppAccountInfo,
        AppMotion,
        AppPhoneCalls,
        AppTrustedDevices,
        AppUnpairedDevices,
        AppDiagnosticInfo,
        AppContacts,
        AppCalendar,
        AppCallHistory,
        AppEmail,
        AppTasks,
        AppMessaging,
        AppRadios,
        AppCamera,
        AppMicrophone,
        AppNotifications,
        AppBackground,
        AppScreenCaptureProgrammatic,
        AppScreenCaptureBorderless,
        AppGazeInput,
        AppHumanPresence,
        AppForegroundText,
        CameraDevice,

        AppSystemAiModels,
        RecallSnapshots,
        RecallComponent,
        ClickToDo,
    }
}

listed! {

    pub enum PrivacyOriginal {
        K01,
        K02,
        K03,
        K04,
        K05,
        K06,
        K07,
        K08,
        K09,
        K10,
        K11,
        K12,
        K13,
        K14,
        K15,
        K16,
        K17,
        K18,
        K19,
        K20,
        K21,
        K22,
        K23,
        K24,
        K25,
        K26,
        K27,
        K28,
        K29,
        K30,
        K31,
        K32,
        K33,
        K34,
        K35,
        K36,
        K37,
        K38,
        K39,
        K40,
        K41,
        K42,
        K43,
        K44,
        K45,
        K46,
        K47,
        K48,
        K49,
        K50,
        K51,
        K52,
    }
}

listed! {

    pub enum PrivacyNotWritten {

        Absent,

        Obsolete,

        Undeterminable,

        NeedsOperationKind,
    }
}

listed! {

    pub enum PrivacyCategory {
        Diagnostics,
        SpeechInput,
        LocationSensors,
        Search,
        Content,
        ActivitySync,
        CloudServices,
        AppPermissions,
        WindowsAi,
    }
}

listed! {

    pub enum PrivacyEdition {
        Home,
        Pro,
        Enterprise,
        Education,
        IotEnterprise,
        Server,
    }
}

listed! {

    pub enum PrivacyBuildEffect {

        Documented,

        Uncertain,

        NoCurrentEffect,
    }
}

listed! {

    pub enum PrivacyEditionFit {

        Honoured,

        Ignored,

        NotStated,
    }
}

listed! {

    pub enum PrivacyScope {
        User,
        Machine,
    }
}

listed! {

    pub enum PrivacySettlement {

        Applied,

        NotApplied,

        Restored,

        Abandoned,
    }
}

listed! {

    pub enum PrivacyFaultCode {
        Identity,
        Clock,
        History,
        Unreadable,
        Unresolved,
        Changed,
        TargetAbsent,
        NothingOwned,
        Conflict,
        NothingUnresolved,
        HistoryFull,
        Expired,
        AccessDenied,
        ElevationDeclined,
        NotApplied,
        ReadbackMismatch,
        Unknown,
        ReceiptLost,
    }
}

impl PrivacyFaultCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Identity => "identity",
            Self::Clock => "clock",
            Self::History => "history",
            Self::Unreadable => "unreadable",
            Self::Unresolved => "unresolved",
            Self::Changed => "changed",
            Self::TargetAbsent => "target_absent",
            Self::NothingOwned => "nothing_owned",
            Self::Conflict => "conflict",
            Self::NothingUnresolved => "nothing_unresolved",
            Self::HistoryFull => "history_full",
            Self::Expired => "expired",
            Self::AccessDenied => "access_denied",
            Self::ElevationDeclined => "elevation_declined",
            Self::NotApplied => "not_applied",
            Self::ReadbackMismatch => "readback_mismatch",
            Self::Unknown => "unknown",
            Self::ReceiptLost => "receipt_lost",
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests {
    use super::PrivacyFaultCode;

    #[test]
    fn a_fault_code_reads_as_it_travels() {
        for code in PrivacyFaultCode::ALL {
            assert_eq!(
                serde_json::to_value(code).unwrap(),
                serde_json::Value::from(code.as_str())
            );
        }
    }
}
