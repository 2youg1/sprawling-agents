// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";
import type { DoctorUntold } from "../../wire";
import { scanningOf, untoldOf } from "./scanning";

describe("scanningOf", () => {
  test("macOS and Linux say why there is nothing to read, and a stopped scanner says it is off", () => {
    expect([scanningOf("does_not_apply"), scanningOf("stopped")]).toEqual([
      [{ subject: "machine_scan_realtime", found: { key: "machine_scan_not_windows", said: null } }],
      [{ subject: "machine_scan_realtime", found: { key: "machine_scan_stopped", said: null } }],
    ]);
  });

  test("a Windows reading names the directory, the drive and the exclusion", () => {
    expect(scanningOf({ read: { city: "D:/city", drive: "trusted", exclusion: { inside: { under: "D:/" } } } })).toEqual([
      { subject: "machine_scan_city", found: { key: "machine_scan_city_read", said: "D:/city" } },
      { subject: "machine_scan_drive", found: { key: "machine_scan_drive_trusted", said: null } },
      { subject: "machine_scan_exclusion", found: { key: "machine_scan_inside", said: "D:/" } },
    ]);
  });

  test("every drive arm and every exclusion arm has its own found line", () => {
    const why: DoctorUntold = "admin_only";
    const lines = [
      { untrusted: { volume: "D:" } },
      { not: { volume: "C:", file_system: "NTFS" } },
      { untold: { why } },
    ].map((drive) => scanningOf({ read: { city: "c", drive, exclusion: "outside" } }));
    expect(lines.map((each) => [each[1]?.found, each[2]?.found])).toEqual([
      [{ key: "machine_scan_drive_untrusted", said: "D:" }, { key: "machine_scan_outside", said: null }],
      [{ key: "machine_scan_drive_not", said: "C: NTFS" }, { key: "machine_scan_outside", said: null }],
      [{ key: "machine_untold_admin_only", said: null }, { key: "machine_scan_outside", said: null }],
    ]);
    expect(scanningOf({ read: { city: "c", drive: "trusted", exclusion: { untold: { why: "no_disk" } } } })[2]?.found).toEqual({
      key: "machine_untold_no_disk",
      said: null,
    });
  });
});

describe("untoldOf", () => {
  test("each reason names the command a User can run by hand", () => {
    const whys: DoctorUntold[] = [
      { unread: { command: "fsutil devdrv query" } },
      { failed: { command: "fsutil devdrv query", code: 5 } },
      { failed: { command: "fsutil devdrv query" } },
      { unstarted: { command: "powershell" } },
      { unanswered: { command: "powershell", stopping: "killed" } },
      { unanswered: { command: "powershell" } },
    ];
    expect(whys.map(untoldOf)).toEqual([
      { key: "machine_untold_unread", said: "fsutil devdrv query" },
      { key: "machine_untold_failed", said: "fsutil devdrv query (5)" },
      { key: "machine_untold_failed", said: "fsutil devdrv query" },
      { key: "machine_untold_unstarted", said: "powershell" },
      { key: "machine_untold_unanswered", said: "powershell (killed)" },
      { key: "machine_untold_unanswered", said: "powershell" },
    ]);
  });
});
