<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // On the real conversation: one round that froze, then a later
  // `session_opened` in the same room. The current session holds no run
  // yet, so it is visibly empty rather than reopening the earlier round
  // (client D80); the toggle at the top of the case flips the reading a
  // probe reads.

  import type { EventKind, EventRecord } from "../../wire";
  import { Address, B3Hash, RunId, Seq, TimeMs } from "../../wire";

  export const FOLDED = Address.make("lab/folded");
  const ROUND = RunId.make("0199c0de-a23a-4000-8000-000000000001");
  const MODEL = "anthropic/claude-sonnet-5";

  function record(
    seq: number,
    at: number,
    kind: EventKind,
    data: Record<string, unknown>,
  ): EventRecord {
    return {
      run: ROUND,
      seq: Seq.make(seq),
      kind,
      t: TimeMs.make(at),
      who: "city",
      addr: FOLDED,
      prev: B3Hash.make("0".repeat(64)),
      v: 1,
      data,
    };
  }

  export function foldedAt(now: number): readonly EventRecord[] {
    return [
      record(1, now - 300_000, "run_started", { task: "Say what the city keeps in its ledger." }),
      record(2, now - 299_000, "model_called", { model: MODEL }),
      record(3, now - 280_000, "run_frozen", { completion: "done" }),
      record(4, now - 60_000, "session_opened", {}),
    ];
  }

  // A run still waiting on its model: the thinking mark stays mounted.
  export function thinkingAt(now: number): readonly EventRecord[] {
    return [
      record(1, now - 3_000, "run_started", { task: "Think through the next small step." }),
      record(2, now - 2_000, "model_called", { model: MODEL }),
    ];
  }
</script>

<script lang="ts">
  import { ui } from "../../ui";
  import Talk from "../talk.svelte";
  import { SPECIMEN } from "../talk/seat";
  import Case from "./case.svelte";
  import Stand from "./stand.svelte";

  const records = foldedAt(ui().now());
  const thinking = thinkingAt(ui().now());
</script>

<Case label="conversation · an empty new session after one round" width={760}>
  <Stand link={{ kind: "live", city: "sprawling" }} unread={[]} waiting={[]} {records}>
    <div class="h-[520px] px-wide">
      <Talk address={FOLDED} band={false} seat={SPECIMEN} />
    </div>
  </Stand>
</Case>

<Case label="conversation · the thinking mark at rest and gathered" width={760}>
  <Stand link={{ kind: "live", city: "sprawling" }} unread={[]} waiting={[]} records={thinking}>
    <div class="h-[520px] px-wide">
      <Talk address={FOLDED} band={false} seat={SPECIMEN} />
    </div>
  </Stand>
</Case>
