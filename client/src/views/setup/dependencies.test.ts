// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";
import type { DoctorAbsence, DoctorItem } from "../../wire";
import { absenceOf, rowOf } from "./dependencies";

const card = (name: string, here: boolean): DoctorItem => ({
  name,
  install: "unknown_platform",
  need: "required",
  tier: "use",
  state: here
    ? { present: { at: name, version: "silent" } }
    : { absent: { absence: "not_on_search_path" } },
});

const drivers = [card("gecko", true), card("webkit", false), card("chromedriver", false)];

describe("absenceOf", () => {
  test("a driver the tier did without is a spare once the verdict names no group", () => {
    expect(drivers.map((each) => absenceOf(each, [], drivers))).toEqual(["wanted", "spare", "spare"]);
  });

  test("a card the verdict names, and every card of a group still short, are wanted", () => {
    const none = [card("webkit", false), card("git", false)];
    expect(none.map((each) => absenceOf(each, ["git", "browser"], none))).toEqual(["wanted", "wanted"]);
    expect(absenceOf(card("webkit", false), null, drivers)).toEqual("wanted");
  });
});

const item = (state: DoctorItem["state"], install: DoctorItem["install"]): DoctorItem => ({
  name: "git",
  install,
  need: "required",
  tier: "use",
  state,
});

const winget = { command: { spelled: "winget install Git.Git" } } as const;

describe("rowOf", () => {
  test("a present item gives its version and offers nothing to install", () => {
    expect(rowOf(item({ present: { at: "C:/git.exe", version: { said: { text: "git version 2.55.0.windows.1" } } } }, winget))).toEqual({
      state: "machine_present",
      reason: null,
      version: "2.55.0",
      install: null,
      offer: "held",
    });
  });

  test("a present item that gave no version says how it did not answer", () => {
    const reasons = (["silent", "unreadable", "late"] as const).map(
      (version) => rowOf(item({ present: { at: "git", version } }, winget)).reason,
    );
    expect(reasons).toEqual([
      { key: "machine_version_silent", said: null },
      { key: "machine_version_unreadable", said: null },
      { key: "machine_version_late", said: null },
    ]);
  });

  test("a broken item gives its fault and the command that gets it again", () => {
    const rows = [
      { will_not_start: { said: "exit 3" } },
      "half_written" as const,
      { unreadable: { said: "denied" } },
    ].map((fault) => rowOf(item({ broken: { at: "git", fault } }, winget)));
    expect(rows.map((row) => [row.state, row.reason, row.install, row.offer])).toEqual([
      ["machine_broken", { key: "machine_fault_will_not_start", said: "exit 3" }, "winget install Git.Git", "press"],
      ["machine_broken", { key: "machine_fault_half_written", said: null }, "winget install Git.Git", "press"],
      ["machine_broken", { key: "machine_fault_unreadable", said: "denied" }, "winget install Git.Git", "press"],
    ]);
  });

  test("an absent item says where the city looked and how to get it", () => {
    const absences: DoctorAbsence[] = [
      "not_on_search_path",
      "no_home",
      "not_in_this_build",
      { variable_names_nothing: { variable: "JAVA_HOME", path: "D:/jdk" } },
      { no_component: { dir: "D:/rust" } },
    ];
    expect(absences.map((absence) => rowOf(item({ absent: { absence } }, { manual: { how: "see the site" } })))).toEqual([
      { state: "machine_absent", reason: { key: "machine_absence_search_path", said: null }, version: null, install: "see the site", offer: "by_hand" },
      { state: "machine_absent", reason: { key: "machine_absence_no_home", said: null }, version: null, install: "see the site", offer: "by_hand" },
      { state: "machine_absent", reason: { key: "machine_absence_build", said: null }, version: null, install: "see the site", offer: "by_hand" },
      { state: "machine_absent", reason: { key: "machine_absence_variable", said: "JAVA_HOME=D:/jdk" }, version: null, install: "see the site", offer: "by_hand" },
      { state: "machine_absent", reason: { key: "machine_absence_component", said: "D:/rust" }, version: null, install: "see the site", offer: "by_hand" },
    ]);
  });

  test("an item with no recipe on this platform has no command", () => {
    expect(rowOf(item({ absent: { absence: "not_on_search_path" } }, "unknown_platform")).install).toBeNull();
  });
});
