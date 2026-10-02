<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // The conversation's frozen facts (client/Spec.lean §4-44, §4-59): the Mayor's
  // session heads its first message with the model, the effort and the
  // mode it froze, under the name it froze - Cat - while the city has
  // since renamed the Mayor to Ada, which the box below already says,
  // because the next session will speak with Ada; each reply's head
  // ends with the time to first content and the output rate. Then the
  // same city before its first send: the empty room is titled with the
  // name a new session would freeze.

  import { QUERIES } from "../../core/asking";
  import { MAYOR } from "../../core/route";
  import type { Answer, EventKind, EventRecord, IdentityAnswer, Opening, Query, Turn } from "../../wire";
  import { B3Hash, RunId, Seq, TimeMs, Tokens, UsdMicros } from "../../wire";

  const MODEL = "anthropic/claude-sonnet-5";
  const RUN = RunId.make("0199c0de-7700-4000-8000-0000000000a1");

  const RENAMED: IdentityAnswer = {
    stated: {
      about: "",
      user_id: null,
      mayor: "Ada",
      mayor_text: "",
      preferences_text: "",
      version: `b3:${"0".repeat(63)}2`,
    },
  };

  function record(seq: number, at: number, kind: EventKind, data: Record<string, unknown>): EventRecord {
    return {
      run: RUN,
      seq: Seq.make(seq),
      kind,
      t: TimeMs.make(at),
      who: "city",
      addr: MAYOR,
      prev: B3Hash.make("0".repeat(64)),
      v: 2,
      data,
    };
  }

  export function recordsAt(now: number): readonly EventRecord[] {
    return [
      record(1, now - 120_000, "run_started", { task: "Say which crates read the ledger." }),
      record(2, now - 119_000, "model_called", { model: MODEL }),
      record(3, now - 60_000, "run_frozen", { completion: "done" }),
    ];
  }

  function turn(number: number, t: number, said: string, first: number, took: number, output: number): Turn {
    return {
      calls: [],
      notes: [],
      number,
      opened: Seq.make(number * 10),
      t: TimeMs.make(t),
      timing: "measured",
      first_at: TimeMs.make(t + first),
      returned: TimeMs.make(t + first + took),
      model: MODEL,
      said,
      used: { input: Tokens.make(18_000), output: Tokens.make(output), cached: Tokens.make(9_000) },
      spent: UsdMicros.make(32_000),
      stopped: "end_turn",
    };
  }

  export function answering(now: number, named: boolean): (query: Query) => Answer | undefined {
    const opening: Opening = {
      at: TimeMs.make(now - 120_000),
      goal: "",
      task: "Say which crates read the ledger.",
      policy: { admit: "standing", landing: "ordinary", mode: "work", write: "full" },
      effort: "high",
      names: { mayor: "Cat" },
    };
    const turns: readonly Turn[] = [
      turn(1, now - 119_000, "Four crates read the ledger: storage writes it, and accounting, wire and citysim read it.", 412, 2_400, 140),
      turn(2, now - 100_000, "accounting folds it into the views; wire only carries the records a client asks for.", 388, 1_600, 96),
    ];
    return (query) => {
      if (query === QUERIES.identity) return { identity: RENAMED };
      if (!named || typeof query !== "object" || !("rounds" in query)) return undefined;
      return { rounds: { run: query.rounds.run, turns, opening } };
    };
  }
</script>

<script lang="ts">
  import { ui } from "../../ui";
  import Talk from "../talk.svelte";
  import Case from "./case.svelte";
  import Stand from "./stand.svelte";

  const now = ui().now();
</script>

<Case label="frozen facts · the Mayor's session under the name it froze, its first head with model · effort · mode, each reply with TTFT and t/s" width={760}>
  <Stand link={{ kind: "live", city: "sprawling" }} unread={[]} waiting={[]} answers={answering(now, true)} records={recordsAt(now)}>
    <div class="h-[520px] px-wide">
      <Talk address={MAYOR} band={false} />
    </div>
  </Stand>
</Case>

<Case label="frozen facts · an empty Mayor's room titled with the name a new session would freeze" width={760}>
  <Stand link={{ kind: "live", city: "sprawling" }} unread={[]} waiting={[]} answers={answering(now, false)}>
    <div class="h-[420px] px-wide">
      <Talk address={MAYOR} band={false} />
    </div>
  </Stand>
</Case>
