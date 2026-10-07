// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The mailbox key keeps its three facts apart (refrain 3-6): a number is
// only what needs the person, something merely new is a dot with no
// number, and the link is a bar of its own; the name says every one of
// them, because the marks are hidden from a screen reader.

import { describe, expect, test } from "bun:test";

import type { LinkState } from "../../core/link";
import { mailboxKey } from "./key";
import type { MailboxWire } from "./key";

const WIRE: MailboxWire = { "aria-expanded": false, "aria-controls": "m-column", onclick: () => undefined };
const LIVE: LinkState = { kind: "live", city: "sprawling" };
const words = (name: string): string => name;

describe("the mailbox key", () => {
  test("counts only what needs the person, and a count hides the dot", () => {
    const look = mailboxKey({ needs: 3, fresh: true, link: LIVE }, "en", words, WIRE);
    expect(look.corner).toEqual({ kind: "count", text: "3" });
    expect(look.hint).toBe("mailbox · 3 waiting");
  });

  test("marks something new with a dot and no number", () => {
    const look = mailboxKey({ needs: 0, fresh: true, link: LIVE }, "en", words, WIRE);
    expect(look.corner).toEqual({ kind: "fresh" });
    expect(look.hint).toBe("mailbox · something new, nothing waiting for you");
  });

  test("carries nothing when nothing waits and the link is live", () => {
    const look = mailboxKey({ needs: 0, fresh: false, link: LIVE }, "en", words, WIRE);
    expect([look.corner, look.foot, look.hint]).toEqual([{ kind: "none" }, { kind: "none" }, "mailbox"]);
  });

  test("bars the link that is not live, pulsing only while it is on its way up", () => {
    const states: readonly [LinkState, "connecting" | "refused"][] = [
      [{ kind: "idle" }, "connecting"],
      [{ kind: "opening" }, "connecting"],
      [{ kind: "handshaking" }, "connecting"],
      [{ kind: "backoff", attempt: 2 }, "connecting"],
      [
        {
          kind: "refused",
          error: { action: "open", code: "E_WIRE_MISMATCH", gate: null, nearby: [], recovery: "reload", retry: "no", subject: "wire" },
        },
        "refused",
      ],
    ];
    for (const [link, bar] of states) {
      const look = mailboxKey({ needs: 1, fresh: false, link }, "en", words, WIRE);
      expect(look.foot).toEqual({ kind: "link", link: bar });
      expect(look.corner).toEqual({ kind: "count", text: "1" });
      expect(look.hint.startsWith("mailbox · 1 waiting · ")).toBe(true);
    }
  });

  test("is a button named by its whole reading, wired by the seat", () => {
    const look = mailboxKey({ needs: 0, fresh: false, link: LIVE }, "en", words, WIRE);
    expect(look.key).toEqual({ as: "button", wire: { ...WIRE, type: "button", "aria-label": "mailbox" } });
  });
});
