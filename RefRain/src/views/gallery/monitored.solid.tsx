// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { Schema } from "effect";

import { Turn } from "../../wire";

const ESC = String.fromCharCode(27);

const said = (value: unknown): { cut: number; head: string } => ({
  cut: 0,
  head: JSON.stringify(value),
});

const TURNS: readonly Turn[] = [
  {
    number: 1,
    opened: 10,
    t: 1000,
    timing: "measured",
    notes: [],
    calls: [
      {
        at: 11,
        called: 1010,
        answered: 1040,
        timing: "measured",
        tool: "edit",
        subject: "src/ledger.rs",
        outcome: "answered",
        arguments: said({ path: "src/ledger.rs" }),
        output: said({
          path: "src/ledger.rs",
          diff: "--- a/src/ledger.rs\n+++ b/src/ledger.rs\n@@ -41,2 +41,3 @@\n-    let seq = self.next;\n-    self.next += 1;\n+    // The sequence is checked so a full ledger refuses rather than wraps.\n+    let seq = self.next;\n+    self.next = seq.checked_add(1).ok_or(Full)?;\n",
        }),
      },
      {
        at: 12,
        called: 1050,
        answered: 3200,
        timing: "measured",
        tool: "exec",
        outcome: "answered",
        arguments: said({ arm: { shell: { text: "cargo test -p ledger" } } }),
        output: said({
          arm: "shell",
          stdout: `running 3 tests\ntest append ... ${ESC}[32mok${ESC}[0m\ntest full ... ${ESC}[31mFAILED${ESC}[0m\n`,
          stderr:
            "memory allocation of 2147483648 bytes failed\nerror: test failed, to rerun pass `-p ledger --lib`\n",
          exit_code: 101,
          memory_ceiling: {
            state: "hit",
            limit_bytes: 1073741824,
            detail: "an allocation of this run's processes was refused at the memory ceiling",
          },
        }),
      },
      {
        at: 13,
        called: 3300,
        timing: "measured",
        tool: "exec",
        outcome: "waiting",
        arguments: said({
          arm: { program: { path: "cargo", args: ["clippy", "-p", "ledger"] } },
        }),
      },
    ],
  },
].map((turn) => Schema.decodeUnknownSync(Turn)(turn));

import RunMonitor from "../monitor/run_monitor";
import { NO_TAIL } from "../../core/live_output";
import { GalleryCase } from "./case";

const ignore = () => undefined;

export default function RunMonitorCases() {
  return (
    <>
      <GalleryCase label="monitor · a run's code beside its terminal">
        <div class="flex h-[32rem] min-h-0 flex-col rounded-card border border-edge">
          <RunMonitor turns={TURNS} tail={NO_TAIL} live={true} onDraft={ignore} onSteer={ignore} />
        </div>
      </GalleryCase>
      <GalleryCase label="monitor · the terminal folded">
        <div class="flex h-[32rem] min-h-0 flex-col rounded-card border border-edge">
          <RunMonitor
            turns={TURNS}
            tail={NO_TAIL}
            live={true}
            following="paused"
            terminal="folded"
            onDraft={ignore}
            onSteer={ignore}
          />
        </div>
      </GalleryCase>
      <GalleryCase label="monitor · a finished run">
        <div class="flex h-[32rem] min-h-0 flex-col rounded-card border border-edge">
          <RunMonitor turns={TURNS} tail={NO_TAIL} live={false} onDraft={ignore} onSteer={ignore} />
        </div>
      </GalleryCase>
    </>
  );
}
