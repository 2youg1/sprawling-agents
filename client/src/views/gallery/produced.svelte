<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->
<script lang="ts" module>

  import type { Call, Output, Turn } from "../../wire";
  import { Address, RunId, Seq, TimeMs } from "../../wire";

  const RUN: RunId = RunId.make("0199c0de-1a2b-4c3d-8e4f-5a6b7c8d9e00");

  function said(head: string, cut: number): Output {
    return { cut, head };
  }

  const REGISTERED: Readonly<Record<string, Pick<Call, "effect" | "render">>> = {
    read: { effect: "read", render: "generic" },
    search: { effect: "read", render: "generic" },
    edit: {
      effect: { write: { domain: Address.make("release") } },
      render: { diff: { locations: [] } },
    },
    exec: { effect: "egress", render: "terminal" },
  };

  function call(at: number, tool: string, subject: string | null, output: Output | null): Call {
    const t = TimeMs.make(at);
    return {
      at: Seq.make(at),
      outcome: "answered",
      output,
      subject,
      tool,
      called: t,
      answered: t,
      timing: "measured",
      ...REGISTERED[tool],
    };
  }

  function turnOf(number: number, calls: readonly Call[]): Turn {
    return {
      calls,
      notes: [],
      number,
      opened: Seq.make(number),
      t: TimeMs.make(number),
      timing: "measured",
    };
  }

  const ONE_COMMAND: readonly Call[] = [
    call(1, "exec", "just check", said("21 gates, 21 green\n", 0)),
  ];

  const GIVEN: readonly Call[] = [
    {
      ...call(1, "edit", "city/hall/PLAN.md", said("1 file changed\n", 0)),
      arguments: said(
        '{"file_path": "city/hall/PLAN.md", "old_string": "- [ ] gate", "new_string": "- [x] gate"',
        212,
      ),
    },
  ];

  const A_WAVE: readonly Call[] = [
    call(1, "read", "Sprawling/kernel/src/gate/door.rs", null),
    call(2, "read", "Sprawling/kernel/src/gate/item.rs", null),
    call(3, "read", "Sprawling/kernel/src/gate/dedup.rs", null),
    call(4, "search", "GateOutcome", null),
    call(5, "read", "Sprawling/kernel/src/approval.rs", null),
    call(6, "read", "Sprawling/kernel/Spec.lean", null),
    call(7, "search", "may_answer", null),
    call(8, "edit", "Sprawling/kernel/src/gate/door.rs", null),
    call(9, "exec", "cargo fmt --all", null),
    call(10, "exec", "cargo clippy -p sprawling-kernel --all-targets", null),
    call(11, "exec", "cargo test -p sprawling-kernel --lib gate", null),
    call(12, "exec", "cargo xtask specalign", null),
  ];

  const NO_VERB: readonly Call[] = [
    call(1, "mcp/github", "list the open pull requests", null),
    call(2, "plan", "split the gallery before adding to it", null),
  ];

  const A_FILE: Call = call(
    1,
    "read",
    "Sprawling/kernel/src/gate/door.rs",
    said(
      `pub fn answer(door: Door, asked: &Asked) -> GateOutcome {
    // A door answers Allow or Deny and never asks a third thing.
    match door {
        Door::Undoable => GateOutcome::Deny,
        Door::Governance => GateOutcome::Allow,
    }
}
`,
      412,
    ),
  );

  const TRAILED = "Sprawling/app/src/assembly/dispatch/lanes/admission.rs";

  const BLOCK_COMMENT = `/* Two lines of one comment, so the colouring has to
   carry state from the end of one line to the start of the next. */
const LIMIT: usize = 400;
/* An unterminated block runs to the end of the file:
const HIDDEN: usize = 0;
`;

  const UNNAMED = `error: the city refused to start
  because: CONFIG.toml names a provider with no key filed for it
  try: sprawling attach --provider example
`;
</script>

<script lang="ts">
  import Calls from "../talk/calls.svelte";
  import Code from "../parts/code.svelte";
  import Case from "./case.svelte";
</script>

{#snippet framed(path: string, text: string)}

  <div class="flex h-output flex-col rounded-card border border-edge-panel">
    <Code {path} {text} />
  </div>
{/snippet}

<Case label="calls · one command">
  <Calls
    calls={ONE_COMMAND}
    run={RUN}
    turn={turnOf(3, ONE_COMMAND)}
    onFork={() => undefined}
    onHover={() => undefined}
  />
</Case>

<Case label="calls · a write and what it was given">
  <Calls
    calls={GIVEN}
    run={RUN}
    turn={turnOf(3, GIVEN)}
    onFork={() => undefined}
    onHover={() => undefined}
  />
</Case>

<Case label="calls · explored, wrote and ran">
  <Calls
    calls={A_WAVE}
    run={RUN}
    turn={turnOf(3, A_WAVE)}
    onFork={() => undefined}
    onHover={() => undefined}
  />
</Case>

<Case label="calls · a class of work the fold has no verb for">
  <Calls
    calls={NO_VERB}
    run={RUN}
    turn={turnOf(3, NO_VERB)}
    onFork={() => undefined}
    onHover={() => undefined}
  />
</Case>

<Case label="code · line numbers and a trail that wraps">

  {@render framed(TRAILED, A_FILE.output?.head ?? "")}
</Case>

<Case label="code · a comment crossing lines, and one that never closes">

  {@render framed("Sprawling/kernel/src/limit.rs", BLOCK_COMMENT)}
</Case>

<Case label="code · no file was named, so no trail and no colour">

  {@render framed("", UNNAMED)}
</Case>
