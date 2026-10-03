// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";
import type { DoctorCustodyLifetime } from "../../wire";
import { restartOf, VERBS } from "./remote";
import { IDLE, answerOf, step, type Door, type DoorEvent, type DoorSent } from "./remote_door";

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

describe("door step (client/spec/Views/Door.lean)", () => {
  const run = (events: readonly DoorEvent[]): { door: Door; sent: DoorSent[] } => {
    let door = IDLE;
    const sent: DoorSent[] = [];
    for (const event of events) {
      const [next, out] = step(door, event);
      door = next;
      if (out !== null) sent.push(out);
    }
    return { door, sent };
  };

  test("a pending answer opens the input with the focus in it", () => {
    expect(run([{ kind: "press", opener: "door" }, { kind: "pending" }])).toEqual({
      door: { phase: { kind: "awaiting", opener: "door" }, typed: "", focus: { kind: "input" } },
      sent: [{ kind: "open" }],
    });
  });

  test("escape cancels and returns the focus to the control that was pressed", () => {
    expect(run([{ kind: "press", opener: "key" }, { kind: "pending" }, { kind: "type", text: "ABCD" }, { kind: "escape" }])).toEqual({
      door: { phase: { kind: "idle" }, typed: "", focus: { kind: "opener", opener: "key" } },
      sent: [{ kind: "replace" }],
    });
  });

  test("a denied code clears the input and waits for a new press", () => {
    expect(
      run([
        { kind: "press", opener: "door" },
        answerOf("E_APPROVAL_PENDING"),
        { kind: "type", text: "ABCD EFGH" },
        { kind: "submit" },
        answerOf("E_GATE_DENIED"),
      ]),
    ).toEqual({
      door: { phase: { kind: "refused", opener: "door", code: "E_GATE_DENIED" }, typed: "", focus: { kind: "opener", opener: "door" } },
      sent: [{ kind: "open" }, { kind: "confirm", code: "ABCD EFGH" }],
    });
  });

  test("a code is sent only from the input, and an empty one is not sent", () => {
    expect(run([{ kind: "submit" }, { kind: "press", opener: "door" }, { kind: "submit" }, { kind: "pending" }, { kind: "submit" }]).sent).toEqual([
      { kind: "open" },
    ]);
  });

  test("a refusal that arrives while nothing is in flight changes nothing", () => {
    expect(run([{ kind: "refusal", code: "E_GATE_DENIED" }])).toEqual({ door: IDLE, sent: [] });
  });
});
