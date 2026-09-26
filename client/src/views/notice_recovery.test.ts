// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";
import { Option, Schema } from "effect";

import type { Recovery } from "../core/recovering";
import { toFragment } from "../core/route";
import { ui } from "../ui";
import type { Ui } from "../ui";
import { Address } from "../wire";
import type { Command } from "../wire";
import { recover, recoveryWhy } from "./notice_recovery";

const NEW: Recovery = { kind: "command", spelled: "/new" };
const FORK: Recovery = { kind: "command", spelled: "/fork" };

// The subject a frozen-model refusal carries: a sentence, which the
// address grammar still reads as a room, so the belief folds it into
// `about` as one.
const SENTENCE = "the model changed from `fake-small` to `fake-chat`";
const FOLDED = Option.getOrNull(Schema.decodeOption(Address)(SENTENCE));

const COMPOSER = Address.make("hall/mayor");

// A page standing in the composer's room, whose sends are kept rather
// than written to a socket.
function standingIn(room: Address): { readonly u: Ui; readonly sent: Command[] } {
  const sent: Command[] = [];
  const u: Ui = {
    ...ui(),
    bar: { hash: toFragment({ kind: "talk", address: room }) },
    send: (command) => {
      sent.push(command);
      return true;
    },
    go: () => undefined,
  };
  return { u, sent };
}

describe("a notice's way out", () => {
  test("opens the new session in the composer's room, not the subject", () => {
    const { u, sent } = standingIn(COMPOSER);
    recover(u, NEW, FOLDED);
    expect(sent.map((command) => ("open_session" in command ? command.open_session.addr : null))).toEqual([
      COMPOSER,
    ]);
  });

  test("is offered where there is a composer, whatever the subject says", () => {
    const { u } = standingIn(COMPOSER);
    expect(recoveryWhy(u, FORK, null)).toBeUndefined();
  });
});
