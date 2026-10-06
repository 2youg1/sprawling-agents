// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The privacy page's vocabulary (`crates/wire/spec/Privacy.lean`): the
//! closed sets every control is described by (§8-85), the answer to
//! `Query::Privacy` (§8-86) and the operation `Command::PrivacyOperation`
//! carries (§8-87).
//!
//! **The host's table is the one authority.** A control's target path, the
//! value it writes and its edition lists are defined once, in the binary's
//! `privacy::controls`, and reach the page only inside the answer; the
//! words the page shows are defined once, in the client's `lang.json`,
//! keyed by the spellings of these sets. Each set therefore has exactly
//! one list of members, and every reader takes it from here.

mod answer;
mod operation;

pub use answer::{PrivacyAnswer, PrivacyControlEntry, PrivacyCurrent, PrivacyEditions};
pub use answer::{PrivacyHistory, PrivacyHost, PrivacyIntent, PrivacyNotWrittenEntry};
pub use answer::{PrivacyOriginalLine, PrivacyTarget, PrivacyValue, PrivacyWindows};
pub use operation::{PrivacyAction, PrivacyOutcome, PrivacyRequest, PrivacyResult};

use serde::{Deserialize, Serialize};

/// Declares a closed set together with `ALL`, its members in declaration
/// order. The array is the variant list itself, so a member cannot be
/// added to the enum and left out of the order the page draws in.
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
            /// Every member, in the order the privacy page shows them.
            pub const ALL: [Self; [$(stringify!($variant)),*].len()] = [$(Self::$variant),*];
        }
    };
}

listed! {
    /// One host setting this binary may change for the person on request
    /// (a privacy control). Declared in page order: by
    /// [`PrivacyCategory`], and within a category in the order of the
    /// controls table.
    pub enum PrivacyControl {
        // Diagnostics and telemetry.
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
        // Speech, typing and inking.
        VoiceActivation,
        VoiceActivationAboveLock,
        OnlineSpeechRecognition,
        ImplicitTextCollection,
        ImplicitInkCollection,
        HandwritingErrorReports,
        InputPersonalization,
        LinguisticDataCollection,
        HandwritingDataSharing,
        // Location and sensors.
        LocationProvider,
        LocationFeature,
        Sensors,
        // Search.
        SearchWeb,
        SearchWebResults,
        CloudSearch,
        Cortana,
        CortanaAboveLock,
        SearchLocation,
        SearchHighlights,
        // Suggestions, advertising and cloud content.
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
        // Activity history and cross-device.
        StartLaunchTracking,
        ActivityFeed,
        ActivityPublish,
        ActivityUpload,
        ClipboardHistory,
        CloudClipboard,
        ContinueExperiences,
        PhonePcLinking,
        MessageSync,
        // Cloud services and network connections.
        OnesettingsDownloads,
        FindMyDevice,
        DeliveryOptimization,
        OnedriveFileStorage,
        OnedrivePreSigninTraffic,
        PushNotificationsNetwork,
        // App permissions.
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
        // Windows AI.
        AppSystemAiModels,
        RecallSnapshots,
        RecallComponent,
        ClickToDo,
    }
}

listed! {
    /// One line of the privacy request list as the person wrote it (an
    /// original item), numbered as the list numbers it.
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
    /// Why an original item is not written. The research decided each
    /// one; nothing here is a judgement the host makes at run time.
    pub enum PrivacyNotWritten {
        /// No current policy template or policy CSP has the target.
        Absent,
        /// The target exists and Microsoft has retired what it governs.
        Obsolete,
        /// The sources do not settle the target's type or written value.
        Undeterminable,
        /// Writing it needs an operation kind this version does not have.
        NeedsOperationKind,
    }
}

listed! {
    /// The sections of the privacy page, in page order.
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
    /// The Windows editions Microsoft's documentation names when it says
    /// where a policy is honoured or ignored.
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
    /// What the sources say about whether writing a control changes
    /// anything on current Windows builds.
    pub enum PrivacyBuildEffect {
        /// The effect is documented for current builds.
        Documented,
        /// The setting exists, and the sources leave open whether current
        /// builds still act on it.
        Uncertain,
        /// The service or feature it governs has ended; writing it changes
        /// nothing today.
        NoCurrentEffect,
    }
}

listed! {
    /// How a control's edition lists read for the edition of this host.
    pub enum PrivacyEditionFit {
        /// The host's edition is in the control's honoured list.
        Honoured,
        /// The host's edition is in the control's ignored list.
        Ignored,
        /// Neither list names the host's edition, or the host's edition is
        /// not one of [`PrivacyEdition`].
        NotStated,
    }
}

listed! {
    /// Whose settings a control changes. A machine-scope write goes
    /// through administrator approval.
    pub enum PrivacyScope {
        User,
        Machine,
    }
}

listed! {
    /// The person's check of an operation that has no conclusion. It
    /// writes nothing; it records what the target held.
    pub enum PrivacySettlement {
        /// The target held the operation's modified value: an apply now
        /// owns its change.
        Applied,
        /// The target held the operation's original value.
        NotApplied,
        /// The target held a restore's modified value: the change it
        /// undid is no longer owned.
        Restored,
        /// The target held neither value; nothing is owned and nothing is
        /// put back.
        Abandoned,
    }
}

listed! {
    /// Why a privacy operation ended without the change asked for, or
    /// with a result nobody could confirm: the closed set of stable codes
    /// (`crates/sprawling/spec/Privacy.lean` §12).
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
    /// The code as it travels, which every privacy error's subject also
    /// opens with.
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

    /// The subject prefix and the wire spelling are one spelling.
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
