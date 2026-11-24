// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { expect, test } from "bun:test";

import type { LinkState } from "../../core/link";
import type { CommandReceipt } from "../../core/socket";
import { get, writable } from "../../core/store";
import { Address, IdemKey, RunId, Seq } from "../../wire";
import type { Command } from "../../wire";
import { attachmentsAt, keepAttachment } from "./attachments";
import { draftPlace } from "./draft";
import { reconcileSpeech, recheckSpeech, speechAt, submitSpeech } from "./speech";

const choice = {
  goal: "",
  effort: null,
  policy: { mode: "chat", write: "full", admit: "standing", landing: "ordinary" },
} satisfies Parameters<typeof submitSpeech>[3];

test("a disconnected historical send keeps its material and retries the same request key", async () => {
  const room = Address.make("test/historical");
  const session = Seq.make(12);
  const key = draftPlace(room, session);
  const commandLog: Command[] = [];
  const state = writable<LinkState>({ kind: "live", city: null });
  const receipts = writable<ReadonlyMap<IdemKey, CommandReceipt>>(new Map());
  const door = {
    state,
    receipts,
    command: (command: Command) => {
      commandLog.push(command);
      return true;
    },
  };
  keepAttachment(key, {
    workspace: "book",
    file: "3",
    version: "abc",
    label: "paper.md",
    selection: { kind: "markdown", from: 0, to: 4, text: "mine" },
  });
  const target = { room, session, run: undefined, historical: true };
  expect(await submitSpeech(door, target, "Explain this", choice)).toBe("sent");
  const first = commandLog[0];
  expect(first).toMatchObject({ dispatch: { resume: session } });
  if (first === undefined || !("dispatch" in first)) return;
  expect(first.dispatch.resume).toBe(session);
  expect(first.dispatch.task).toContain('"text":"mine"');
  const unrelated = IdemKey.make(`idem1-${"a".repeat(32)}`);
  receipts.set(new Map([[unrelated, { kind: "accepted" }]]));
  expect(reconcileSpeech(door, key)).toBeNull();
  state.set({ kind: "backoff", attempt: 0 });
  expect(reconcileSpeech(door, key)).toBeNull();
  expect(get(speechAt(key)).kind).toBe("unknown");
  expect(get(attachmentsAt(key))).toHaveLength(1);
  state.set({ kind: "live", city: null });
  recheckSpeech(door, key);
  expect(commandLog[1]).toEqual(first);
  receipts.set(new Map([[first.dispatch.idem, { kind: "accepted" }]]));
  expect(reconcileSpeech(door, key)).toBe("Explain this");
  expect(get(attachmentsAt(key))).toEqual([]);
});

test("a known text-only run leaves a selected PNG attached and never sends", async () => {
  const room = Address.make("test/text-only");
  const key = draftPlace(room, null);
  const state = writable<LinkState>({ kind: "live", city: null });
  const receipts = writable<ReadonlyMap<IdemKey, CommandReceipt>>(new Map());
  let sends = 0;
  const door = {
    state,
    receipts,
    command: () => {
      sends += 1;
      return true;
    },
  };
  keepAttachment(key, {
    workspace: "book",
    file: "4",
    version: "def",
    label: "paper.pdf",
    selection: {
      kind: "pdf-region",
      page: 2,
      region: { x: 0.1, y: 0.1, width: 0.4, height: 0.1 },
      image: new Blob([], { type: "image/png" }),
    },
  });
  const target = {
    room,
    session: null,
    run: RunId.make("00000000-0000-4000-8000-000000000001"),
    imageInput: false,
  };
  expect(await submitSpeech(door, target, "Compare this formula", choice)).toBe(
    "image-unavailable",
  );
  expect(sends).toBe(0);
  expect(get(attachmentsAt(key))).toHaveLength(1);
});

test("an ended run leaves its draft ready to send as a follow-up", async () => {
  const room = Address.make("test/ended");
  const key = draftPlace(room, null);
  const run = RunId.make("00000000-0000-4000-8000-000000000002");
  const state = writable<LinkState>({ kind: "live", city: null });
  const receipts = writable<ReadonlyMap<IdemKey, CommandReceipt>>(new Map());
  const commandLog: Command[] = [];
  const door = {
    state,
    receipts,
    command: (command: Command) => {
      commandLog.push(command);
      return true;
    },
  };
  keepAttachment(key, {
    workspace: "book",
    file: "5",
    version: "base",
    label: "paper.md",
    selection: { kind: "markdown", from: 0, to: 7, text: "excerpt" },
  });
  expect(await submitSpeech(door, { room, session: null, run }, "Elaborate", choice)).toBe("sent");
  const sent = commandLog[0];
  expect(sent).toMatchObject({ steer: { run } });
  if (sent === undefined || !("steer" in sent)) return;
  const error = {
    code: "E_INVALID_ARGS",
    action: "steer a run",
    subject: run,
    retry: "no",
    recovery: "the run ended before it accepted the steer",
    nearby: [],
    request: sent.steer.idem,
  } as const;
  receipts.set(new Map([[sent.steer.idem, { kind: "refused", error }]]));
  expect(reconcileSpeech(door, key)).toBeNull();
  expect(get(speechAt(key))).toEqual({ kind: "refused", error });
  expect(get(attachmentsAt(key))).toHaveLength(1);
  expect(
    await submitSpeech(door, { room, session: null, run: undefined }, "Elaborate", choice),
  ).toBe("sent");
  const followUp = commandLog[1];
  expect(followUp).toMatchObject({ dispatch: { addr: room } });
  expect(followUp !== undefined && "dispatch" in followUp ? followUp.dispatch.task : "").toContain(
    "excerpt",
  );
});
