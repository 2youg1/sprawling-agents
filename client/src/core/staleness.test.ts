// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import type { EventKind } from "../wire";
import { reachOf } from "./staleness";

// The settings panel calls a save saved only when the city answers
// again (client/Spec.lean §7L), so each write the panel makes has to reach
// the answer the panel reads it back from.
describe("a write reaches the answer that reads it back", () => {
  test("a saved identity card makes the identity answer stale", () => {
    expect(reachOf("identity", "governed_document_written")).toBe("every");
  });

  test("a written rules or city layer file makes its document and the config answer stale", () => {
    expect(reachOf("document", "rules_changed")).toBe("every");
    expect(reachOf("config", "rules_changed")).toBe("every");
    expect(reachOf("config", "building_configured")).toBe("every");
  });

  test("a record that writes none of them leaves them held", () => {
    expect(reachOf("identity", "model_returned")).toBe("none");
    expect(reachOf("config", "model_returned")).toBe("none");
    expect(reachOf("automation", "rules_changed")).toBe("none");
  });
});

// The sessions pane lists a room's stretches from this answer, so a
// session opened, a run started in one, or a run ended has to reach it.
test("a session opened or a run starting or ending makes the sessions answers stale", () => {
  const kinds: EventKind[] = ["session_opened", "run_started", "run_frozen", "model_returned"];
  expect(kinds.map((kind) => reachOf("sessions", kind))).toEqual(
    ["every", "every", "every", "none"],
  );
});
