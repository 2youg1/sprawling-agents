<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // The thread as the template draws it (client-SPEC 4-44): message heads
  // with the frozen model on the first and the time to first content on
  // each reply, tool lines of every kind a registration names - with a
  // failed one and one whose line carried no registration - and a call
  // still running with the steer pin on it; then the same room waiting on
  // the person past ten seconds, the head with the rhythm this page
  // watched a reply arrive in, and the words a person just sent in each
  // state they can stand in.
  //
  // The running calls are placed against the page's clock when the route
  // opens, so their timers read a few seconds and move.

  import type { Answer, Call, EventKind, EventRecord, Query, Turn } from "../../wire";
  import { Address, B3Hash, RunId, Seq, TimeMs, Tokens, UsdMicros } from "../../wire";
  import type { Delivery } from "../talk/delivery";

  export const WORKING = Address.make("lab/thread");
  export const ASKING = Address.make("lab/asking");
  const MODEL = "anthropic/claude-sonnet-5";
  const EARLIER = RunId.make("0199c0de-7700-4000-8000-000000000001");
  const GOING = RunId.make("0199c0de-7700-4000-8000-000000000002");
  const WAITING = RunId.make("0199c0de-7700-4000-8000-000000000003");

  function record(run: RunId, seq: number, at: number, kind: EventKind, data: Record<string, unknown>): EventRecord {
    return {
      run,
      seq: Seq.make(seq),
      kind,
      t: TimeMs.make(at),
      who: "city",
      addr: run === WAITING ? ASKING : WORKING,
      prev: B3Hash.make("0".repeat(64)),
      v: 1,
      data,
    };
  }

  export function recordsAt(now: number): readonly EventRecord[] {
    return [
      record(EARLIER, 1, now - 600_000, "run_started", { task: "Read the city's specification and say where documents are saved." }),
      record(EARLIER, 2, now - 598_000, "model_called", { model: MODEL }),
      record(EARLIER, 3, now - 560_000, "run_frozen", { completion: "done" }),
      record(GOING, 10, now - 90_000, "run_started", { task: "Make a stale save refuse instead of overwrite, and run the checks." }),
      record(GOING, 11, now - 89_000, "model_called", { model: MODEL }),
      record(GOING, 12, now - 4_700, "tool_called", { name: "exec", subject: "just check-client" }),
      record(WAITING, 20, now - 60_000, "run_started", { task: "Publish the crate." }),
      record(WAITING, 21, now - 59_000, "model_called", { model: MODEL }),
      record(WAITING, 22, now - 14_000, "tool_called", { name: "exec", subject: "cargo publish" }),
      record(WAITING, 23, now - 13_800, "approval_requested", { action_desc: "run cargo publish outside the sandbox" }),
    ];
  }

  function call(at: number, called: number, took: number | null, shape: Partial<Call>): Call {
    return {
      tool: "exec",
      subject: null,
      arguments: null,
      outcome: took === null ? "waiting" : "answered",
      at: Seq.make(at),
      output: null,
      called: TimeMs.make(called),
      answered: took === null ? null : TimeMs.make(called + took),
      timing: "measured",
      ...shape,
    };
  }

  function turn(number: number, t: number, said: string | null, calls: readonly Call[], ttft: number | null): Turn {
    return {
      calls,
      notes: [],
      number,
      opened: Seq.make(number * 100),
      t: TimeMs.make(t),
      timing: "measured",
      first_at: ttft === null ? null : TimeMs.make(t + ttft),
      model: MODEL,
      said,
      used: { input: Tokens.make(20_000 + number * 4_000), output: Tokens.make(900), cached: Tokens.make(12_000) },
      spent: UsdMicros.make(41_000),
      stopped: said === null ? null : "end_turn",
    };
  }

  // What the made-up city answers: the three runs' rounds.
  export function answering(now: number): (query: Query) => Answer | undefined {
    const going = now - 89_000;
    const rounds: Readonly<Record<string, readonly Turn[]>> = {
      [EARLIER]: [turn(1, now - 598_000, "Documents are saved through `edit_against`, which compares the version you read with the one on disk.", [], 512)],
      [GOING]: [
        turn(1, going, "I'll read the specification first, then change where the save compares versions.", [
          call(101, going + 2_000, 31, { tool: "read", subject: "crates/city/city-SPEC.md", effect: "read", render: "generic" }),
          call(102, going + 2_100, 140, { tool: "search", subject: "edit_against", effect: "read", render: "generic" }),
          call(103, going + 2_400, 3_412, { tool: "exec", subject: "cargo nextest -p city", effect: "read", render: "terminal" }),
          call(104, going + 6_000, 87, {
            tool: "edit",
            subject: "crates/city/src/document.rs",
            effect: { write: { domain: Address.make("lab") } },
            render: { diff: { locations: [] } },
          }),
          call(105, going + 6_200, 220, { tool: "fetch_docs", subject: "docs.rs/git2", outcome: "failed", effect: null, render: null }),
        ], 412),
        turn(2, going + 40_000, "Done: a save against a stale version is refused and your draft stays. Running the client checks now.", [
          call(201, now - 4_700, null, { tool: "exec", subject: "just check-client", effect: "read", render: "terminal" }),
        ], 398),
      ],
      [WAITING]: [
        turn(1, now - 59_000, "The crate is ready; publishing needs the network outside the sandbox.", [
          call(301, now - 14_000, null, { tool: "exec", subject: "cargo publish", effect: "egress", render: "terminal" }),
        ], 640),
      ],
    };
    return (query) => {
      if (typeof query !== "object" || !("rounds" in query)) return undefined;
      const turns = rounds[query.rounds.run];
      return turns === undefined ? undefined : { rounds: { run: query.rounds.run, turns } };
    };
  }

  // A reply watched arriving in a steady stream with one stall.
  export const RHYTHM: readonly number[] = [
    0.6, 0.8, 1, 0.9, 0.7, 0.8, 0.6, 0, 0, 0, 0.2, 0.7, 0.9, 1, 0.8, 0.9, 0.7, 0.6, 0.5, 0.3,
  ];

  const WORDS = "Also keep the old draft when the save is refused.";
  export const DELIVERIES: readonly Exclude<Delivery, { readonly kind: "none" }>[] = [
    { kind: "held", words: WORDS, refusal: null },
    { kind: "pending", words: WORDS, refusal: null, mark: 0 },
    { kind: "accepted", words: WORDS },
    { kind: "unknown", words: WORDS, refusal: null, mark: 0 },
  ];
</script>

<script lang="ts">
  import { ui } from "../../ui";
  import Talk from "../talk.svelte";
  import Delivered from "../talk/delivered.svelte";
  import Head from "../talk/head.svelte";
  import Case from "./case.svelte";
  import Stand from "./stand.svelte";

  const now = ui().now();
  const records = recordsAt(now);
  const answers = answering(now);
</script>

<Case label="thread · heads, tool lines of every kind, a call running with the steer pin" width={760}>
  <Stand link={{ kind: "live", city: "sprawling" }} unread={[]} waiting={[]} {answers} {records}>
    <div class="h-[760px] px-wide">
      <Talk address={WORKING} band={false} />
    </div>
  </Stand>
</Case>

<Case label="thread · a call past ten seconds, waiting on the person" width={760}>
  <Stand link={{ kind: "live", city: "sprawling" }} unread={[]} waiting={[]} {answers} {records}>
    <div class="h-[420px] px-wide">
      <Talk address={ASKING} band={false} />
    </div>
  </Stand>
</Case>

<Case label="thread · a head with the time to first content and the rhythm the reply arrived in" width={760}>
  <Head who="Cat" at={now} model={null} ttft={412} rhythm={RHYTHM} />
</Case>

<Case label="thread · words just sent: held, pending, accepted, unknown" width={760}>
  {#each DELIVERIES as delivery (delivery.kind)}
    <Delivered {delivery} />
  {/each}
</Case>
