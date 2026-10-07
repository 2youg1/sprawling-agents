// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import type { PrivacyAnswer, PrivacyControl } from "../../../wire";
import { ELSEWHERE, HOME, IDEM, REFUSAL, UNRESOLVED } from "../../gallery/privacy_answer";
import type { Action } from "./entry";
import { actionOf, confirmOf, lookOf, outcomeOf, sectionsOf, type EntryModel } from "./page";

function model(answer: PrivacyAnswer, control: PrivacyControl): EntryModel {
  const found = sectionsOf(answer)
    .flatMap((section) => section.entries)
    .find((each) => each.row.control === control);
  if (found === undefined) return expect.unreachable(`${control} is in the fixture`);
  return found;
}

const actions = (answer: PrivacyAnswer, control: PrivacyControl): Action[] =>
  model(answer, control).offers.map((offer) => offer.action);

describe("the privacy page", () => {
  test("keeps the host's page order and groups it by category", () => {
    const sections = sectionsOf(HOME);
    expect(sections.map((section) => section.category)).toEqual([
      "diagnostics", "speech_input", "location_sensors", "content", "activity_sync", "app_permissions", "windows_ai",
    ]);
    expect(sections.flatMap((section) => section.entries.map((entry) => entry.row.control))).toEqual(
      HOME.controls.map((row) => row.control),
    );
  });

  test("offers apply only where the value read differs from the value written", () => {
    expect(actions(HOME, "powershell_telemetry_optout")).toEqual(["apply"]);
    expect(actions(HOME, "device_census_task")).toEqual(["apply"]);
    // No task, nothing to write; a target this account may not read.
    expect(actions(HOME, "ceip_usb_task")).toEqual([]);
    expect(actions(HOME, "app_location")).toEqual([]);
  });

  test("offers restore only while the value read is the one this app wrote", () => {
    expect(actions(HOME, "recall_snapshots")).toEqual(["restore"]);
    const moved: PrivacyAnswer = {
      ...HOME,
      controls: HOME.controls.map((row) =>
        row.control === "recall_snapshots" ? { ...row, current: { read: { value: { value: "dword", number: 7 } } } } : row,
      ),
    };
    expect(actions(moved, "recall_snapshots")).toEqual(["apply"]);
  });

  test("offers only the check of a change with no conclusion, on its own entry", () => {
    expect(actions(UNRESOLVED, "tailored_experiences")).toEqual(["reconcile"]);
    expect(actions(UNRESOLVED, "powershell_telemetry_optout")).toEqual([]);
    expect(actions(UNRESOLVED, "recall_snapshots")).toEqual([]);
  });

  test("offers nothing while the history is withheld, or on another system", () => {
    const withheld: PrivacyAnswer = { ...HOME, history: { withheld: { error: REFUSAL } } };
    expect(sectionsOf(withheld).flatMap((section) => section.entries.flatMap((entry) => entry.offers))).toEqual([]);
    expect(sectionsOf(ELSEWHERE).flatMap((section) => section.entries.flatMap((entry) => entry.offers))).toEqual([]);
  });

  test("sends the value it shows as the expected one", () => {
    const entry = model(HOME, "device_census_task");
    const [offer] = entry.offers;
    if (offer === undefined) return expect.unreachable("apply is offered");
    expect(entry.row.current).toEqual({ read: { value: offer.expected } });
    expect(actionOf("device_census_task", offer)).toEqual({ apply: { control: "device_census_task", expected: offer.expected } });
    expect(actionOf("tailored_experiences", { action: "reconcile", expected: { value: "absent" } })).toEqual({
      reconcile: { expected: { value: "absent" } },
    });
  });

  test("reads only the result kept under its own idem", () => {
    const kept: PrivacyAnswer = {
      ...HOME,
      outcomes: [{ idem: IDEM, action: { apply: { control: "diagnostic_data", expected: { value: "absent" } } }, result: "already_written" }],
    };
    expect(outcomeOf(kept, IDEM)).toBe("already_written");
    expect(outcomeOf(HOME, IDEM)).toBeNull();
  });

  test("hands the look one press per offer, each pressing its own action", () => {
    const pressed: Action[] = [];
    const look = lookOf("en", model(HOME, "recall_snapshots"), {
      root: "privacy",
      stage: { kind: "idle" },
      shown: null,
      press: (action) => pressed.push(action),
    });
    for (const press of look.presses) press.onPress();
    expect(pressed).toEqual(["restore"]);
    expect(look.id).toBe("privacy-recall_snapshots");
  });

  test("states the change, its cost and its undoing before the click", () => {
    const shown = confirmOf("en", model(HOME, "recall_snapshots"), "restore");
    expect(shown.facts.map((fact) => fact.value).slice(0, 2)).toEqual(["1", "not set"]);
    expect(shown.notes).toHaveLength(2);
  });

  test("shows a refusal of this page's own operation with the host's way out", () => {
    const look = lookOf("en", model(HOME, "tailored_experiences"), {
      root: "privacy",
      stage: { kind: "refused", idem: IDEM },
      shown: { refused: { code: "access_denied", error: REFUSAL } },
      press: () => undefined,
    });
    expect(look.status).toEqual({ text: `not changed: ${REFUSAL.recovery}`, weight: "alert" });
  });
});
