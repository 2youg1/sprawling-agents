// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What the machine page says about scanning in front of the city's
// directory (`crates/wire/spec/Answer/Doctor.lean` D25): one line per
// fact the city read, each a subject and what was found.
//
// Windows is the only platform with an answer: Defender's real-time
// scanning and the Dev Drive question exist there alone, and macOS and
// Linux answer `does_not_apply` because they run no system scanner in
// front of every write. The page says that reason rather than hiding the
// section, so a User on those platforms does not wonder whether the read
// failed.

import type { Key } from "../../core/lang";
import type { DoctorDrive, DoctorExclusion, DoctorScanning, DoctorUntold } from "../../wire";
import type { Reason } from "../setup/dependencies";

export interface ScanLine {
  readonly subject: Key;
  readonly found: Reason;
}

export function scanningOf(scanning: DoctorScanning): readonly ScanLine[] {
  if (typeof scanning === "object") {
    return [
      { subject: "machine_scan_city", found: { key: "machine_scan_city_read", said: scanning.read.city } },
      { subject: "machine_scan_drive", found: driveOf(scanning.read.drive) },
      { subject: "machine_scan_exclusion", found: exclusionOf(scanning.read.exclusion) },
    ];
  }
  switch (scanning) {
    case "does_not_apply":
      return [{ subject: "machine_scan_realtime", found: { key: "machine_scan_not_windows", said: null } }];
    case "stopped":
      return [{ subject: "machine_scan_realtime", found: { key: "machine_scan_stopped", said: null } }];
  }
}

function driveOf(drive: DoctorDrive): Reason {
  if (typeof drive === "string") return { key: "machine_scan_drive_trusted", said: null };
  if ("untrusted" in drive) return { key: "machine_scan_drive_untrusted", said: drive.untrusted.volume };
  if ("not" in drive) return { key: "machine_scan_drive_not", said: `${drive.not.volume} ${drive.not.file_system}` };
  return untoldOf(drive.untold.why);
}

function exclusionOf(exclusion: DoctorExclusion): Reason {
  if (typeof exclusion === "string") return { key: "machine_scan_outside", said: null };
  if ("inside" in exclusion) return { key: "machine_scan_inside", said: exclusion.inside.under };
  return untoldOf(exclusion.untold.why);
}

// Why the city could not read one of the two: the command it ran is
// named as the platform spells it, so a User can run it by hand.
export function untoldOf(why: DoctorUntold): Reason {
  if (typeof why === "string") {
    switch (why) {
      case "no_disk":
        return { key: "machine_untold_no_disk", said: null };
      case "admin_only":
        return { key: "machine_untold_admin_only", said: null };
    }
  }
  if ("unread" in why) return { key: "machine_untold_unread", said: why.unread.command };
  if ("failed" in why) {
    const code = why.failed.code ?? null;
    return { key: "machine_untold_failed", said: code === null ? why.failed.command : `${why.failed.command} (${String(code)})` };
  }
  if ("unstarted" in why) return { key: "machine_untold_unstarted", said: why.unstarted.command };
  const stopping = why.unanswered.stopping ?? null;
  return {
    key: "machine_untold_unanswered",
    said: stopping === null ? why.unanswered.command : `${why.unanswered.command} (${stopping})`,
  };
}
