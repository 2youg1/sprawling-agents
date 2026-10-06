<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // What a run did and what it left behind: the wave of tool calls
  // folded to one line, and the read-only view of a file a call handed
  // over. The inspector that opens one call in full has its own section
  // (`ins.svelte`).
  //
  // Both read the same `Call` values, so the calls are written once at
  // the top of this file and each fixture names the ones it wants.

  import type { Call, Output } from "../../wire";
  import { Address, RunId, Seq, TimeMs } from "../../wire";

  // The run every fixture here speaks for. One id, because the link a
  // cut result offers points at a run page and two ids would point at
  // two.
  //
  // A real run id and not a readable label: `RunId` is a `Uuid` on the
  // wire, so its schema states the hyphenated shape and this line is
  // checked when the module loads. A label here threw a `ParseError`
  // at import time and blanked the whole route - which is the schema
  // doing its job, on a fixture that had been spelling an id the city
  // cannot mint.
  const RUN: RunId = RunId.make("0199c0de-1a2b-4c3d-8e4f-5a6b7c8d9e00");

  function said(head: string, cut: number): Output {
    return { cut, head };
  }

  // What the city registers each tool as (`kernel::ToolMeta`), which is
  // what a call carries on the wire; a tool missing here is one the city
  // could not place, and its call carries neither.
  const REGISTERED: Readonly<Record<string, Pick<Call, "effect" | "render">>> = {
    read: { effect: "read", render: "generic" },
    search: { effect: "read", render: "generic" },
    edit: { effect: { write: { domain: Address.make("release") } }, render: { diff: { locations: [] } } },
    exec: { effect: "egress", render: "terminal" },
  };

  function call(
    at: number,
    tool: string,
    subject: string | null,
    output: Output | null,
  ): Call {
    const t = TimeMs.make(at);
    return { at: Seq.make(at), outcome: "answered", output, subject, tool, called: t, answered: t, timing: "measured", ...REGISTERED[tool] };
  }

  // One command and nothing else, which is the shortest sentence the
  // fold can make.
  const ONE_COMMAND: readonly Call[] = [
    call(1, "exec", "just check", said("21 gates, 21 green\n", 0)),
  ];

  // A write carries what it was given as well as what it said, both cut
  // the same way; the arguments here are cut so the count shows.
  const GIVEN: readonly Call[] = [
    {
      ...call(1, "edit", "city/hall/PLAN.md", said("1 file changed\n", 0)),
      arguments: said('{"path": "city/hall/PLAN.md", "old": "- [ ] gate", "new": "- [x] gate"', 212),
    },
  ];

  // The wave the fold was written for: seven files read, one written,
  // four commands run. Four clauses would be wrong here - a class of
  // work that did not happen contributes no clause - so this one
  // produces three.
  const A_WAVE: readonly Call[] = [
    call(1, "read", "crates/kernel/src/gate/door.rs", null),
    call(2, "read", "crates/kernel/src/gate/item.rs", null),
    call(3, "read", "crates/kernel/src/gate/dedup.rs", null),
    call(4, "search", "GateOutcome", null),
    call(5, "read", "crates/kernel/src/approval.rs", null),
    call(6, "read", "crates/kernel/Spec.lean", null),
    call(7, "search", "may_answer", null),
    call(8, "edit", "crates/kernel/src/gate/door.rs", null),
    call(9, "exec", "cargo fmt --all", null),
    call(10, "exec", "cargo clippy -p sprawling-kernel --all-targets", null),
    call(11, "exec", "cargo test -p sprawling-kernel --lib gate", null),
    call(12, "exec", "cargo xtask specalign", null),
  ];

  // A class of work the fold has no verb for: governing, planning,
  // spawning, or reaching an outside server. Real work, and the
  // summary says so by counting it rather than by naming a deed it
  // does not have.
  const NO_VERB: readonly Call[] = [
    call(1, "mcp/github", "list the open pull requests", null),
    call(2, "plan", "split the gallery before adding to it", null),
  ];

  // A file a call handed back, in a family this build can colour.
  const A_FILE: Call = call(
    1,
    "read",
    "crates/kernel/src/gate/door.rs",
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

  // A file with a trail long enough to wrap, so the crumbs are
  // measured as a row that has to fold rather than as three words.
  const TRAILED = "crates/sprawling/src/assembly/dispatch/lanes/admission.rs";

  const BLOCK_COMMENT = `/* Two lines of one comment, so the colouring has to
   carry state from the end of one line to the start of the next. */
const LIMIT: usize = 400;
/* An unterminated block runs to the end of the file:
const HIDDEN: usize = 0;
`;

  // No path at all, which is what a call that named no file hands
  // over: no trail is drawn and nothing is coloured, and the line
  // numbers are the only thing left to read by.
  const UNNAMED = `error: the city refused to start
  because: CONFIG.toml names a provider with no key filed for it
  try: sprawling attach --provider zenmux
`;
</script>

<script lang="ts">
  import Calls from "../talk/calls.svelte";
  import Code from "../parts/code.svelte";
  import Case from "./case.svelte";
</script>

{#snippet framed(path: string, text: string)}
  <!-- The room the inspector gives the code view. Stated here so the code
  fixtures are measured at a height a person actually meets, rather
  than each growing to the length of whatever file it holds. -->
  <div class="flex h-output flex-col rounded-card border border-edge-panel">
    <Code {path} {text} />
  </div>
{/snippet}

<Case label="calls · one command">
  <Calls calls={ONE_COMMAND} run={RUN} />
</Case>

<Case label="calls · a write and what it was given">
  <Calls calls={GIVEN} run={RUN} />
</Case>

<Case label="calls · explored, wrote and ran">
  <Calls calls={A_WAVE} run={RUN} />
</Case>

<Case label="calls · a class of work the fold has no verb for">
  <Calls calls={NO_VERB} run={RUN} />
</Case>

<Case label="code · line numbers and a trail that wraps">
  <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression (a snippet call is the render itself; the typechecker types local snippet calls as returning void) -->
  {@render framed(TRAILED, A_FILE.output?.head ?? "")}
</Case>

<Case label="code · a comment crossing lines, and one that never closes">
  <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression (a snippet call is the render itself; the typechecker types local snippet calls as returning void) -->
  {@render framed("crates/kernel/src/limit.rs", BLOCK_COMMENT)}
</Case>

<Case label="code · no file was named, so no trail and no colour">
  <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression (a snippet call is the render itself; the typechecker types local snippet calls as returning void) -->
  {@render framed("", UNNAMED)}
</Case>
