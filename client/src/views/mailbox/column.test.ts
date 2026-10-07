// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The head of the mailbox column offers a way to mend the link only when
// the city refused the page, and that way answers the refusal it came
// from; a link on its way up mends itself and offers nothing.

import { describe, expect, test } from "bun:test";

import type { LinkState } from "../../core/link";
import type { Recovery } from "../../core/recovering";
import type { AxError } from "../../wire";
import { linkOf } from "./column";

const MISMATCH: AxError = {
  action: "open the link to the city",
  code: "E_WIRE_MISMATCH",
  gate: null,
  nearby: [],
  recovery: "reload the page",
  retry: "no",
  subject: "wire",
};

describe("the mailbox column's head", () => {
  test("says nothing of a live link", () => {
    expect(linkOf({ kind: "live", city: null }, "en", () => undefined)).toBeUndefined();
  });

  test("says a link on its way up and offers no recovery", () => {
    const ways: readonly LinkState[] = [{ kind: "idle" }, { kind: "opening" }, { kind: "handshaking" }, { kind: "backoff", attempt: 1 }];
    for (const link of ways) {
      expect(linkOf(link, "en", () => undefined)).toEqual({ word: "connecting…", recovery: undefined });
    }
  });

  test("offers the city's way to mend a refusal, and pressing it answers that refusal", () => {
    const pressed: [Recovery, AxError][] = [];
    const look = linkOf({ kind: "refused", error: MISMATCH }, "en", (lever, error) => {
      pressed.push([lever, error]);
    });
    expect(look?.word).toBe("refused");
    expect(look?.recovery?.label).toBe("reload");
    look?.recovery?.onPress();
    expect(pressed).toEqual([[{ kind: "reload", verb: "link_reload" }, MISMATCH]]);
  });
});
