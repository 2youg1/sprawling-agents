// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";
import type { DoctorCustodyLifetime } from "../../wire";
import { restartOf, VERBS } from "./remote";

describe("restartOf", () => {
  test("each vault lifetime says what a restart does to a pairing", () => {
    const lifetimes: (DoctorCustodyLifetime | null)[] = [null, "across_reboots", "with_passphrase", "until_reboot", "this_process"];
    expect(lifetimes.map(restartOf)).toEqual([
      "remote_restart_unknown",
      "remote_restart_kept",
      "remote_restart_passphrase",
      "remote_restart_until_reboot",
      "remote_restart_process",
    ]);
  });
});


describe("VERBS", () => {
  test("names every sub-verb the console's /remote owns", () => {
    expect(VERBS.map(([spelling]) => spelling.split(" ")[1])).toEqual([
      "open",
      "pair",
      "devices",
      "revoke",
      "close",
      "replace-key",
    ]);
  });
});
