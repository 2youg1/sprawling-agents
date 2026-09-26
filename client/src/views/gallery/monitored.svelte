<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // The monitor over a run that has edited a file, run a command that
  // failed with coloured output, and is still running a second one, so
  // one fixture shows every ending a terminal row draws and a hunk with
  // both line numbers. The results are the compact JSON the runtime
  // writes, which is the shape `monitor/trace.ts` reads.

  import { Schema } from "effect";

  import { Turn } from "../../wire";

  const ESC = String.fromCharCode(27);

  const said = (value: unknown): { cut: number; head: string } => ({ cut: 0, head: JSON.stringify(value) });

  const TURNS: readonly Turn[] = [
    {
      number: 1,
      opened: 10,
      t: 1000,
      notes: [],
      calls: [
        {
          at: 11,
          tool: "edit",
          subject: "src/ledger.rs",
          outcome: "answered",
          arguments: said({ path: "src/ledger.rs" }),
          output: said({
            path: "src/ledger.rs",
            base_version: "b1",
            new_version: "b2",
            diff: "--- a/src/ledger.rs\n+++ b/src/ledger.rs\n@@ -41,2 +41,3 @@\n-    let seq = self.next;\n-    self.next += 1;\n+    // The sequence is checked so a full ledger refuses rather than wraps.\n+    let seq = self.next;\n+    self.next = seq.checked_add(1).ok_or(Full)?;\n",
          }),
        },
        {
          at: 12,
          tool: "exec",
          outcome: "answered",
          arguments: said({ arm: { shell: { text: "cargo test -p ledger" } } }),
          output: said({
            arm: "shell",
            stdout: `running 3 tests\ntest append ... ${ESC}[32mok${ESC}[0m\ntest full ... ${ESC}[31mFAILED${ESC}[0m\n`,
            stderr: "error: test failed, to rerun pass `-p ledger --lib`\n",
            exit_code: 101,
          }),
        },
        {
          at: 13,
          tool: "exec",
          outcome: "waiting",
          arguments: said({ arm: { program: { path: "cargo", args: ["clippy", "-p", "ledger"] } } }),
        },
      ],
    },
  ].map((turn) => Schema.decodeUnknownSync(Turn)(turn));
</script>

<script lang="ts">
  import Monitor from "../monitor/monitor.svelte";
  import { NO_TAIL } from "../../core/live_output";
  import Case from "./case.svelte";
</script>

<Case label="monitor · a run's code beside its terminal">
  <div class="flex h-[32rem] min-h-0 flex-col rounded-card border border-edge">
    <Monitor turns={TURNS} tail={NO_TAIL} live={true} onDraft={() => undefined} onSteer={() => undefined} />
  </div>
</Case>
