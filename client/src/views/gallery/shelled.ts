// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The made-up city the shell's specimens stand in (`shell.svelte` at
// 1440, `mob.svelte` on one column): a session whose last turn used a
// fifth of a 204,800-token window, the two reminders at 30% and 65%,
// and a page of commits.
import type { Answer, Call, CommitAnswer, Query, Turn } from "../../wire";
import { Address, GitOid, RunId, Seq, TimeMs, Tokens, UsdMicros } from "../../wire";
import { ENDPOINTS } from "./served";

// The room the conversation is in: it has a run waiting, one finished
// and one going (`resulted.svelte` deals its rooms out in turn).
export const ROOM = Address.make("release/ledger");
export const EMPTY_ROOM = Address.make("lab/fresh");
const START = 1_790_000_000_000;
const MODEL = "anthropic/claude-fable-5.1";

function call(n: number, tool: string, subject: string): Call {
  return {
    tool,
    subject,
    arguments: null,
    outcome: "answered",
    at: Seq.make(n),
    output: { head: `${tool} ${subject}\nok`, cut: 0 },
    called: TimeMs.make(START + n * 1_000),
    answered: TimeMs.make(START + n * 1_000 + 412),
    timing: "measured",
    // What the city registers the two tools as (`kernel::ToolMeta`),
    // which every call carries on the wire.
    ...(tool === "edit"
      ? { effect: { write: { domain: ROOM } }, render: { diff: { locations: [] } } }
      : { effect: "read", render: "generic" }),
  };
}

function turnsOf(calling: boolean): readonly Turn[] {
  return [1, 2].map((number) => ({
    calls: calling && number === 2 ? [call(20, "read", "crates/city/city-SPEC.md"), call(21, "edit", "crates/city/src/document.rs")] : [],
    notes: [],
    number,
    opened: Seq.make(number * 10),
    t: TimeMs.make(START + number * 60_000),
    timing: "measured",
    model: MODEL,
    said: number === 2 ? "Done: the document reads against the version the person saw, and refuses a stale one." : "Reading the city's specification first.",
    used: { input: Tokens.make(20_000 + number * 10_000), output: Tokens.make(1_100), cached: Tokens.make(12_000) },
    spent: UsdMicros.make(41_000),
  }));
}

const COMMITS: readonly CommitAnswer[] = [
  ["a1b2c3d4e5f60718293a4b5c6d7e8f9012345678", "edit document.rs", "release/ledger"],
  ["7c41e09aa1b2c3d4e5f60718293a4b5c6d7e8f90", "read the specification", "release/ledger"],
  ["5e90b12c3d4e5f60718293a4b5c6d7e8f9012345", "nextest coverage for city", "memory/gate"],
  ["c3d8a40d4e5f60718293a4b5c6d7e8f901234567", "merge room2 into lab", "docs/tidy"],
].map(([oid, message, actor], n) => ({
  actor: Address.make(actor ?? "hall/mayor"),
  at: TimeMs.make(START - n * 600_000),
  lineage: [],
  message,
  model: MODEL,
  oid: GitOid.make(oid ?? "0".repeat(40)),
  run: RunId.make(`0199c0de-0000-4000-8000-${n.toString(16).padStart(12, "0")}`),
  seq: Seq.make(1_000 - n),
  spent: UsdMicros.make(12_000),
}));

// What the made-up city answers. `calling` decides whether the session's
// run has done anything a right pane would open on.
export function answering(calling: boolean): (query: Query) => Answer | undefined {
  return (query) => {
    if (typeof query !== "object") return undefined;
    if ("rounds" in query) return { rounds: { run: query.rounds.run, turns: turnsOf(calling) } };
    if ("endpoint_view" in query) return { endpoints: ENDPOINTS };
    if ("commits" in query) return { commits: { building: null, before: null, commits: COMMITS, more: false } };
    if ("config" in query) {
      return {
        config: {
          addr: query.config.addr,
          first: 30,
          second: { percent: 65, domain: { min: 31, max: 90 }, from: "city" },
          tuning: { from: "default", proxying: "except_local", timeout_ms: 600_000 },
        },
      };
    }
    return undefined;
  };
}
