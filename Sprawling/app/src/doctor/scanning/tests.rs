// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::{Path, PathBuf};

use super::{Drive, Exclusion, Scanning, Untold, drive_said, exclusion_said, lines};
use crate::doctor::asking::Ended;

fn exited(code: i32, stdout: &str) -> Ended {
    Ended::Exited {
        code: Some(code),
        kept: stdout.to_owned(),
    }
}

const REFUSED_IN_GBK: [u8; 34] = [
    0xce, 0xde, 0xb7, 0xa8, 0xb4, 0xf2, 0xbf, 0xaa, 0xbe, 0xed, 0xa1, 0xa3, 0x0d, 0x0a, 0xb4, 0xed,
    0xce, 0xf3, 0x20, 0x35, 0x3a, 0x20, 0xbe, 0xdc, 0xbe, 0xf8, 0xb7, 0xc3, 0xce, 0xca, 0xa1, 0xa3,
    0x0d, 0x0a,
];

#[test]
fn fsutil_refusing_an_ordinary_user_reads_as_cannot_tell() {
    let refused = Ended::Exited {
        code: Some(1),
        kept: String::from_utf8_lossy(&REFUSED_IN_GBK).into_owned(),
    };
    assert_eq!(
        drive_said("D:", "ReFS", &refused),
        Drive::Untold(Untold::Failed {
            command: "fsutil devdrv query",
            code: Some(1),
        })
    );
    let localized = exited(0, &String::from_utf8_lossy(&REFUSED_IN_GBK));
    assert_eq!(
        drive_said("D:", "ReFS", &localized),
        Drive::Untold(Untold::Unread {
            command: "fsutil devdrv query",
        })
    );
}

#[test]
fn the_statement_fsutil_prints_says_whether_the_volume_is_a_dev_drive() {
    let filters = "Developer volumes are protected by antivirus filter, by default.\r\n\
                   Filters currently attached to this developer volume:\r\n    WdFilter, bindFlt\r\n";
    let said = |statement: &str| {
        drive_said(
            "D:",
            "ReFS",
            &exited(0, &format!("{statement}\r\n{filters}")),
        )
    };
    assert_eq!(said("This is a trusted developer volume."), Drive::Trusted);
    assert_eq!(
        said("This is a developer volume but it is not trusted."),
        Drive::Untrusted {
            volume: "D:".to_owned()
        }
    );
    assert_eq!(
        said("This is not a trusted developer volume."),
        Drive::Untrusted {
            volume: "D:".to_owned()
        }
    );
    assert_eq!(
        said("This is not a developer volume."),
        Drive::Not {
            volume: "D:".to_owned(),
            file_system: "ReFS".to_owned(),
        }
    );
}

#[test]
fn defender_hiding_its_exclusions_reads_as_cannot_tell() {
    let hidden = exited(0, "N/A: Must be an administrator to view exclusions\r\n");
    assert_eq!(
        exclusion_said(Path::new(r"C:\cities\one"), &hidden),
        Exclusion::Untold(Untold::AdminOnly)
    );
    assert_eq!(
        exclusion_said(Path::new(r"C:\cities\one"), &exited(1, "")),
        Exclusion::Untold(Untold::Failed {
            command: "Get-MpPreference",
            code: Some(1),
        }),
        "a Defender that cannot be asked is not a Defender with no exclusions"
    );
}

#[test]
fn an_exclusion_covers_the_city_only_at_a_directory_boundary() {
    let city = Path::new(r"D:\Work\city");
    let listed = |entries: &str| exclusion_said(city, &exited(0, entries));
    assert_eq!(
        listed("C:\\temp\r\nd:\\work\\\r\n"),
        Exclusion::Inside {
            under: r"d:\work\".to_owned()
        }
    );
    assert_eq!(
        listed("D:\\Work\\city\r\n"),
        Exclusion::Inside {
            under: r"D:\Work\city".to_owned()
        }
    );
    assert_eq!(
        listed("D:\\Wor\r\n%USERPROFILE%\\x\r\nD:\\*\r\n"),
        Exclusion::Outside
    );
    assert_eq!(listed(""), Exclusion::Outside, "no exclusions at all");
}

#[test]
fn the_advice_names_the_command_that_would_exclude_the_city() {
    let city = PathBuf::from(r"D:\cities\o'neil");
    let slow = lines(&Scanning::Read {
        city: city.clone(),
        drive: Drive::Not {
            volume: "D:".to_owned(),
            file_system: "NTFS".to_owned(),
        },
        exclusion: Exclusion::Untold(Untold::AdminOnly),
    });
    assert_eq!(
        slow,
        vec![
            "  scanning - whether antivirus scanning holds up the city's writes".to_owned(),
            String::new(),
            r"    city            D:\cities\o'neil".to_owned(),
            "    dev drive       no: D: is NTFS, not a Dev Drive".to_owned(),
            "    exclusion       cannot tell: Windows shows Defender's exclusions only to an administrator"
                .to_owned(),
            "    to speed it up  move the city onto a Dev Drive (Settings > System > Storage > \
             Advanced storage settings > Disks & volumes > Create dev drive),"
                .to_owned(),
            r"                    or exclude it in an administrator PowerShell: Add-MpPreference -ExclusionPath 'D:\cities\o''neil'"
                .to_owned(),
            String::new(),
        ]
    );
    let fast = lines(&Scanning::Read {
        city,
        drive: Drive::Trusted,
        exclusion: Exclusion::Outside,
    });
    assert!(
        !fast.iter().any(|line| line.contains("to speed it up")),
        "{fast:?}"
    );
    assert!(
        lines(&Scanning::DoesNotApply)
            .iter()
            .any(|line| line.contains("does not apply")),
        "another platform is told the check is not for it"
    );
}
