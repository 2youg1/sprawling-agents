// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The names the privacy page and the host share: which control, which
//! line of the request list, why a line is not written, and the closed
//! sets every control is described by (`crates/wire/spec/Privacy.lean`
//! §8-85).
//!
//! **Only names cross the wire.** A control's target path, the value it
//! writes and its edition lists are defined once, in the binary's
//! `privacy::controls`; the words the page shows are defined once, in the
//! client's `lang.json`, keyed by these spellings. Each set therefore has
//! exactly one list of members, and every reader takes it from here.

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
