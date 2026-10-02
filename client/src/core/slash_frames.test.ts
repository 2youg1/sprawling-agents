// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What every `/` verb does, driven through the doors a typed line goes
// through - `parse`, `find`, `run` - and judged as the city would meet
// it: each frame the verb sends is written the way the socket writes
// it and read back through the wire's own `ClientFrame` schema, then
// compared whole (Roadmap G1c). The table is keyed by the verbs `SLASH`
// holds, so a verb added there without a case here turns this file red.
// The finer behaviour of the verbs with a seam of their own - `/new`
// and `/fork`, `/halt` and `/release`, `/stop` - is `slash.test.ts`'s.

import { describe, expect, test } from "bun:test";
import { Schema } from "effect";

import { Address, ClientFrame, IdemKey, ProviderName, RunId, Seq, TemplateName } from "../wire";
import type { Effort, RunPolicy } from "../wire";
import { encodeFrame } from "./frames";
import type { View } from "./route";
import { SLASH, find, parse } from "./slash";
import type { SlashHands } from "./slash_hands";

const ROOM = Address.make("lab/room1");
const MAYOR = Address.make("hall/mayor");
const LIVE = RunId.make("00000000-0000-0000-0000-000000000001");
const NEWEST = RunId.make("00000000-0000-0000-0000-000000000002");
// Each frame is minted a fresh key; on the way back it is given this
// one, so a whole frame can be compared.
const IDEM = IdemKey.make(`idem1-${"0".repeat(32)}`);
const IDEM_RE = /"idem":"idem1-[0-9a-f]{32}"/gu;

const readFrame = Schema.decodeUnknownSync(ClientFrame);

// Everything a verb did through its hands.
interface Done {
  readonly sent: readonly ClientFrame[];
  readonly went: readonly View[];
  readonly written: readonly string[];
  readonly efforts: readonly (Effort | null)[];
  readonly policies: readonly RunPolicy[];
}

const NOTHING: Done = { sent: [], went: [], written: [], efforts: [], policies: [] };

// What the person chose beside the box: a create-only experiment that
// must show its tests, so a verb that drops a value is seen dropping it.
const CHOSEN: RunPolicy = { mode: "work", write: "create", admit: "tested", landing: "experiment" };

interface Case {
  readonly line: string;
  readonly done: Done;
}

function ran(line: string): Done {
  const sent: ClientFrame[] = [];
  const went: View[] = [];
  const written: string[] = [];
  const efforts: (Effort | null)[] = [];
  const policies: RunPolicy[] = [];
  const hands: SlashHands = {
    command: (command) => {
      sent.push(readFrame(JSON.parse(encodeFrame({ command }).replace(IDEM_RE, `"idem":"${IDEM}"`))));
      return true;
    },
    go: (view) => went.push(view),
    here: ROOM,
    live: { run: LIVE, at: Seq.make(3) },
    newest: (room) => (room === MAYOR ? { run: NEWEST, at: Seq.make(9) } : null),
    models: [{ endpoint: "local", model: "m-local" }],
    effort: "high",
    setEffort: (effort) => efforts.push(effort),
    policy: CHOSEN,
    setPolicy: (policy) => policies.push(policy),
    goal: "the goal",
    write: (next) => written.push(next),
  };
  const call = parse(line);
  const verb = find(call?.verb ?? "");
  if (call !== null && verb !== undefined) verb.run(hands, call);
  return { sent, went, written, efforts, policies };
}

// A verb that did its one thing and emptied the line.
const sending = (...sent: ClientFrame[]): Done => ({ ...NOTHING, sent, written: [""] });
const going = (view: View): Done => ({ ...NOTHING, went: [view], written: [""] });
const setting = (effort: Effort | null): Done => ({ ...NOTHING, efforts: [effort], written: [""] });
const admitting = (admit: RunPolicy["admit"]): Done => ({ ...NOTHING, policies: [{ ...CHOSEN, admit }], written: [""] });

const CASES: Readonly<Record<string, readonly Case[]>> = {
  "/dispatch": [
    {
      line: "/dispatch add the parser",
      done: sending({
        command: {
          dispatch: {
            addr: ROOM,
            task: "add the parser",
            goal: "the goal",
            policy: CHOSEN,
            session: null,
            effort: "high",
            idem: IDEM,
          },
        },
      }),
    },
    { line: "/dispatch", done: NOTHING },
  ],
  "/steer": [
    { line: "/steer read the city first", done: sending({ command: { steer: { run: LIVE, text: "read the city first", idem: IDEM } } }) },
  ],
  "/stop": [{ line: "/stop", done: sending({ command: { cancel: { run: LIVE, idem: IDEM } } }) }],
  "/halt": [
    { line: "/halt lab", done: sending({ command: { halt: { scope: { building: Address.make("lab") }, idem: IDEM } } }) },
    { line: "/halt --all", done: sending({ command: { halt: { scope: "city", idem: IDEM } } }) },
  ],
  "/release": [{ line: "/release", done: sending({ command: { release: { scope: "city", idem: IDEM } } }) }],
  "/raise": [
    { line: "/raise lab", done: sending({ command: { create_building: { addr: Address.make("lab"), template: TemplateName.make("minimal"), idem: IDEM } } }) },
    { line: "/raise lab hall", done: sending({ command: { create_building: { addr: Address.make("lab"), template: TemplateName.make("hall"), idem: IDEM } } }) },
    { line: "/raise lab palace", done: NOTHING },
  ],
  "/new": [{ line: "/new", done: sending({ command: { open_session: { addr: ROOM, carry: "nothing", from: null, idem: IDEM } } }) }],
  "/clear": [{ line: "/clear", done: sending({ command: { open_session: { addr: ROOM, carry: "nothing", from: null, idem: IDEM } } }) }],
  "/fork": [
    {
      line: "/fork hall/mayor",
      done: sending({
        command: { open_session: { addr: MAYOR, carry: "nothing", from: { run: NEWEST, at_seq: Seq.make(9) }, idem: IDEM } },
      }),
    },
    { line: "/fork", done: { ...NOTHING, written: [""] } },
  ],
  "/model": [
    {
      line: "/model m-local",
      done: sending({
        command: {
          select_model: {
            endpoint: ProviderName.make("local"),
            model: "m-local",
            tag: "main",
            context_tokens: null,
            max_output_tokens: null,
            input: null,
            idem: IDEM,
          },
        },
      }),
    },
    { line: "/model nope", done: NOTHING },
  ],
  "/effort": [
    { line: "/effort max", done: setting("max") },
    { line: "/effort", done: setting(null) },
    { line: "/effort unstated", done: setting(null) },
    { line: "/effort bogus", done: NOTHING },
  ],
  "/admit": [
    { line: "/admit contract", done: admitting("contract_kept") },
    { line: "/admit double", done: admitting("double_validated") },
    { line: "/admit", done: admitting("standing") },
    { line: "/admit everything", done: NOTHING },
  ],
  "/room": [
    { line: "/room hall/mayor", done: going({ kind: "talk", address: MAYOR }) },
    { line: "/room", done: NOTHING },
  ],
  "/go": [
    { line: "/go cost", done: going({ kind: "cost" }) },
    { line: "/go nowhere", done: NOTHING },
  ],
  "/mcp": [{ line: "/mcp", done: going({ kind: "mcp" }) }],
  "/doctor": [{ line: "/doctor", done: going({ kind: "welcome" }) }],
  "/help": [{ line: "/help", done: { ...NOTHING, written: ["/"] } }],
  "/diff": [{ line: "/diff", done: going({ kind: "run", run: LIVE }) }],
};

describe("every / verb", () => {
  test("has its frames written down here", () => {
    expect(Object.keys(CASES).sort()).toEqual(SLASH.map((verb) => verb.spelling).sort());
  });

  for (const [verb, cases] of Object.entries(CASES)) {
    for (const each of cases) {
      test(`${verb}: ${each.line}`, () => {
        expect(ran(each.line)).toEqual(each.done);
      });
    }
  }
});
