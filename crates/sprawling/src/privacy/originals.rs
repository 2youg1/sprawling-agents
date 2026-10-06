// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The original items: each line of the privacy request list as the person
//! wrote it, and what became of it (`crates/sprawling/spec/Privacy/Controls.lean`).
//!
//! A line either writes a control, shared with any line that names the
//! same target, or is not written for a reason the research settled; a
//! line not written names the controls that come closest, which may be
//! none. The text is kept as written, misspellings included, because the
//! page shows the person their own words.

use wire::{PrivacyControl, PrivacyNotWritten, PrivacyOriginal};

/// One original item.
#[derive(Debug)]
pub(crate) struct Original {
    pub(crate) text: &'static str,
    pub(crate) disposition: Disposition,
}

/// What became of an original item.
#[derive(Debug)]
pub(crate) enum Disposition {
    Writes(PrivacyControl),
    NotWritten {
        reason: PrivacyNotWritten,
        alternatives: &'static [PrivacyControl],
    },
}

const fn writes(text: &'static str, control: PrivacyControl) -> Original {
    Original {
        text,
        disposition: Disposition::Writes(control),
    }
}

const fn not_written(
    text: &'static str,
    reason: PrivacyNotWritten,
    alternatives: &'static [PrivacyControl],
) -> Original {
    Original {
        text,
        disposition: Disposition::NotWritten {
            reason,
            alternatives,
        },
    }
}

/// The original item `item`.
pub(crate) const fn original(item: PrivacyOriginal) -> Original {
    match item {
        PrivacyOriginal::K01 => writes(
            "POWERSHELLTELEMETRYOPTOUT=1",
            PrivacyControl::PowershellTelemetryOptout,
        ),
        PrivacyOriginal::K02 => not_written(
            "TELEMETRY_OPT_IN_OUT",
            PrivacyNotWritten::Undeterminable,
            &[
                PrivacyControl::DotnetCliTelemetryOptout,
                PrivacyControl::PowershellTelemetryOptout,
            ],
        ),
        PrivacyOriginal::K03 => not_written(
            "阻止 `DeviceCensus.exe",
            PrivacyNotWritten::NeedsOperationKind,
            &[
                PrivacyControl::DeviceCensusTask,
                PrivacyControl::DeviceCensusUserTask,
            ],
        ),
        PrivacyOriginal::K04 => not_written(
            "移除 `AitAgent` 任务",
            PrivacyNotWritten::Absent,
            &[PrivacyControl::ApplicationTelemetry],
        ),
        PrivacyOriginal::K05 => writes("停用 `Consolidator", PrivacyControl::CeipConsolidatorTask),
        PrivacyOriginal::K06 => not_written(
            "停用 `KernelCeipTask",
            PrivacyNotWritten::Absent,
            &[PrivacyControl::WindowsCeip],
        ),
        PrivacyOriginal::K07 => writes("停用 `UsbCeip", PrivacyControl::CeipUsbTask),
        PrivacyOriginal::K08 => writes("CEIPEnable` 和类似键设为 `O", PrivacyControl::WindowsCeip),
        PrivacyOriginal::K09 => not_written(
            "“完全停止 SQM”",
            PrivacyNotWritten::Undeterminable,
            &[
                PrivacyControl::WindowsCeip,
                PrivacyControl::ApplicationTelemetry,
            ],
        ),
        PrivacyOriginal::K10 => writes(
            "AllowTelemetry=0`，诊断级别“无数据”",
            PrivacyControl::DiagnosticData,
        ),
        PrivacyOriginal::K11 => not_written(
            "AgentActivationEnabled=0",
            PrivacyNotWritten::Undeterminable,
            &[
                PrivacyControl::VoiceActivation,
                PrivacyControl::VoiceActivationAboveLock,
            ],
        ),
        PrivacyOriginal::K12 => writes(
            "LetAppsActivateWithVoice=2",
            PrivacyControl::VoiceActivation,
        ),
        PrivacyOriginal::K13 => writes(
            "LetAppsActivateWithVoiceAboveLock=2` 确保 WinLogon 时麦克风硬件禁用",
            PrivacyControl::VoiceActivationAboveLock,
        ),
        PrivacyOriginal::K14 => writes(
            "DisableWindowsLocationProvider=1",
            PrivacyControl::LocationProvider,
        ),
        PrivacyOriginal::K15 => writes("DisableLocation=1", PrivacyControl::LocationFeature),
        PrivacyOriginal::K16 => writes("DisableWebSearch=1", PrivacyControl::SearchWeb),
        PrivacyOriginal::K17 => writes("ConnectedSearchUseWeb=0", PrivacyControl::SearchWebResults),
        PrivacyOriginal::K18 => writes(
            "TailoredExperiencesWithDiagnosticDataEnabled=0",
            PrivacyControl::TailoredExperiences,
        ),
        PrivacyOriginal::K19 => writes(
            "ContentDeliveryManager` 阻止 Spotlight/Recommendations",
            PrivacyControl::Spotlight,
        ),
        PrivacyOriginal::K20 => writes(
            r"OnlineSpeechPrivacy\HasAccepted=0",
            PrivacyControl::OnlineSpeechRecognition,
        ),
        PrivacyOriginal::K21 => writes(
            "DoNotShowFeedbackNotifications=1",
            PrivacyControl::FeedbackNotifications,
        ),
        PrivacyOriginal::K22 => writes(
            "RestrictImplicitText/InkCollection=1` 中 Text 部分",
            PrivacyControl::ImplicitTextCollection,
        ),
        PrivacyOriginal::K23 => writes("同上 Ink 部分", PrivacyControl::ImplicitInkCollection),
        PrivacyOriginal::K24 => writes(
            "PreventHandwritingErrorReports=1",
            PrivacyControl::HandwritingErrorReports,
        ),
        PrivacyOriginal::K25 => writes("DisableSensors=1", PrivacyControl::Sensors),
        PrivacyOriginal::K26 => not_written(
            "Disablelnventory=1",
            PrivacyNotWritten::Undeterminable,
            &[PrivacyControl::WindowsCeip],
        ),
        PrivacyOriginal::K27 => writes("Start_TrackProgs=0", PrivacyControl::StartLaunchTracking),
        PrivacyOriginal::K28 => writes(
            "AllowlnputPersonalization=0",
            PrivacyControl::InputPersonalization,
        ),
        PrivacyOriginal::K29 => writes(
            r"HKLM\Policies(Windows\StepRecorder` 禁用 `PSR.exe",
            PrivacyControl::StepsRecorder,
        ),
        PrivacyOriginal::K30 => writes(
            "RestrictImplicitInkCollection/TextCollection=1` 中 Ink",
            PrivacyControl::ImplicitInkCollection,
        ),
        PrivacyOriginal::K31 => writes("同上 Text", PrivacyControl::ImplicitTextCollection),
        PrivacyOriginal::K32 => writes("EnableActivityFeed=0", PrivacyControl::ActivityFeed),
        PrivacyOriginal::K33 => writes("LetAppsAccessLocation=2", PrivacyControl::AppLocation),
        PrivacyOriginal::K34 => {
            writes("LetAppsAccessAccountInfo=2", PrivacyControl::AppAccountInfo)
        }
        PrivacyOriginal::K35 => writes("LetAppsAccessMotion=2", PrivacyControl::AppMotion),
        PrivacyOriginal::K36 => writes("LetAppsAccessPhone=2", PrivacyControl::AppPhoneCalls),
        PrivacyOriginal::K37 => writes(
            "LetAppsAccessTrustedDevices=2",
            PrivacyControl::AppTrustedDevices,
        ),
        PrivacyOriginal::K38 => writes(
            "LetAppsSyncWithDevices=2",
            PrivacyControl::AppUnpairedDevices,
        ),
        PrivacyOriginal::K39 => not_written(
            "AllowCrossDeviceSync=0",
            PrivacyNotWritten::Absent,
            &[
                PrivacyControl::CloudClipboard,
                PrivacyControl::ContinueExperiences,
                PrivacyControl::PhonePcLinking,
                PrivacyControl::ActivityFeed,
            ],
        ),
        PrivacyOriginal::K40 => writes(
            "LetAppsGetDiagnosticInfo=2",
            PrivacyControl::AppDiagnosticInfo,
        ),
        PrivacyOriginal::K41 => writes("LetAppsAccessContacts=2", PrivacyControl::AppContacts),
        PrivacyOriginal::K42 => writes("LetAppsAccessCalendar=2", PrivacyControl::AppCalendar),
        PrivacyOriginal::K43 => {
            writes("LetAppsAccessCallHistory=2", PrivacyControl::AppCallHistory)
        }
        PrivacyOriginal::K44 => writes("LetAppsAccessEmail=2", PrivacyControl::AppEmail),
        PrivacyOriginal::K45 => writes("LetAppsAccessTasks=2", PrivacyControl::AppTasks),
        PrivacyOriginal::K46 => writes("LetAppsAccessMessaging=2", PrivacyControl::AppMessaging),
        PrivacyOriginal::K47 => writes("LetAppsAccessRadios=2", PrivacyControl::AppRadios),
        PrivacyOriginal::K48 => not_written(
            "LetAppsAccessBluetooth=2",
            PrivacyNotWritten::Absent,
            &[
                PrivacyControl::AppRadios,
                PrivacyControl::AppTrustedDevices,
                PrivacyControl::AppUnpairedDevices,
            ],
        ),
        PrivacyOriginal::K49 => not_written(
            "LetAppsAccessDocumentsFolder=2",
            PrivacyNotWritten::Absent,
            &[],
        ),
        PrivacyOriginal::K50 => not_written(
            "LetAppsAccessPicturesFolder=2",
            PrivacyNotWritten::Absent,
            &[],
        ),
        PrivacyOriginal::K51 => not_written(
            "LetAppsAccessVideosFolder=2",
            PrivacyNotWritten::Absent,
            &[],
        ),
        PrivacyOriginal::K52 => {
            not_written("LetAppsAccessFileSystem=2", PrivacyNotWritten::Absent, &[])
        }
    }
}

/// How many original items are written and how many are not.
const fn tally() -> (usize, usize) {
    let mut written = 0_usize;
    let mut not = 0_usize;
    let mut rest: &[PrivacyOriginal] = &PrivacyOriginal::ALL;
    while let [item, tail @ ..] = rest {
        match original(*item).disposition {
            Disposition::Writes(_) => written = written.saturating_add(1),
            Disposition::NotWritten { .. } => not = not.saturating_add(1),
        }
        rest = tail;
    }
    (written, not)
}

const TALLY: (usize, usize) = tally();
const _: () = assert!(PrivacyOriginal::ALL.len() == 52);
const _: () = assert!(TALLY.0 == 39 && TALLY.1 == 13);

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing, reason = "test code")]
mod tests {
    use super::*;

    /// The page explains a line it does not write in the line's own
    /// words, kept in the client's phrase table under the line's name; the
    /// client cannot know which lines those are, so this table checks it.
    /// A written line has no explanation, so one left over from an earlier
    /// decision fails here too.
    #[test]
    fn exactly_the_lines_not_written_have_a_reason_in_both_languages() {
        let words: serde_json::Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../client/src/lang.json"
        )))
        .unwrap();
        let wrong: Vec<String> = PrivacyOriginal::ALL
            .iter()
            .filter_map(|item| {
                let name = serde_json::to_value(item).unwrap();
                let key = format!("privacy_original_{}_reason", name.as_str().unwrap());
                let entry = &words[&key];
                let worded = ["en", "zh"]
                    .iter()
                    .all(|lang| entry[lang].as_str().is_some_and(|text| !text.is_empty()));
                let owed = matches!(original(*item).disposition, Disposition::NotWritten { .. });
                (worded != owed || (!owed && !entry.is_null())).then_some(key)
            })
            .collect();
        assert_eq!(wrong, Vec::<String>::new());
    }
}
