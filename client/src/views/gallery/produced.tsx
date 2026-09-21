// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What a run did and what it left behind: the wave of tool calls folded
// to one line, the panel beside the conversation, and the read-only
// view of a file a call handed over.
//
// The three read the same `Call` values, so the calls are written once
// at the top of this file and each fixture names the ones it wants. A
// fold that counted one set of calls while the panel drew another would
// be two answers to "what did this run just do".

import type { JSX } from "solid-js";

import type { Call, Output } from "../../wire";
import { RunId, Seq } from "../../wire";
import type { Artifacts } from "../talk/trace";
import { Artifact } from "../talk/artifact";
import { Calls } from "../talk/calls";
import { Code } from "../parts/code";
import { Case } from "./case";

// The run every fixture here speaks for. One id, because the link a cut
// result offers points at a run page and two ids would point at two.
const RUN: RunId = RunId.make("run-gallery");

function said(head: string, cut: number): Output {
  return { cut, head };
}

function call(at: number, tool: string, subject: string | null, output: Output | null): Call {
  return { at: Seq.make(at), outcome: "answered", output, subject, tool };
}

// One command and nothing else, which is the shortest sentence the fold
// can make.
const ONE_COMMAND: readonly Call[] = [
  call(1, "exec", "just check", said("21 gates, 21 green\n", 0)),
];

// The wave the fold was written for: seven files read, one written,
// four commands run. Four clauses would be wrong here - a class of work
// that did not happen contributes no clause - so this one produces
// three.
const A_WAVE: readonly Call[] = [
  call(1, "read", "crates/kernel/src/gate/door.rs", null),
  call(2, "read", "crates/kernel/src/gate/item.rs", null),
  call(3, "read", "crates/kernel/src/gate/dedup.rs", null),
  call(4, "search", "GateOutcome", null),
  call(5, "read", "crates/kernel/src/approval.rs", null),
  call(6, "read", "crates/kernel/kernel-SPEC.md", null),
  call(7, "search", "may_answer", null),
  call(8, "edit", "crates/kernel/src/gate/door.rs", null),
  call(9, "exec", "cargo fmt --all", null),
  call(10, "exec", "cargo clippy -p kernel --all-targets", null),
  call(11, "exec", "cargo test -p kernel --lib gate", null),
  call(12, "exec", "cargo xtask specalign", null),
];

// A class of work the fold has no verb for: governing, planning,
// spawning, or reaching an outside server. Real work, and the summary
// says so by counting it rather than by naming a deed it does not have.
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

// A file whose last part this build's table does not hold. Drawn in one
// ink rather than guessed at: a comment marker borrowed from another
// family hides a line instead of dimming it.
const UNKNOWN_KIND: Call = call(
  2,
  "read",
  "docs/gate.adoc",
  said(
    `= What a door answers
:toc:

// In AsciiDoc this line is a comment; in six other families it is not,
// which is why this build colours none of it.
A door answers *Allow* or *Deny*.
`,
    0,
  ),
);

const A_TERMINAL: Call = call(
  3,
  "exec",
  "cargo clippy -p kernel --all-targets -- -D warnings",
  said(
    `    Checking kernel v0.0.5 (C:\\sprawling\\crates\\kernel)
    Finished \`dev\` profile [unoptimized + debuginfo] in 4.12s
`,
    0,
  ),
);

function panel(file: Call | null, terminal: Call | null): Artifacts {
  return { file, terminal };
}

// A file with a trail long enough to wrap, so the crumbs are measured
// as a row that has to fold rather than as three words.
const TRAILED = "crates/sprawling/src/assembly/dispatch/lanes/admission.rs";

const BLOCK_COMMENT = `/* Two lines of one comment, so the colouring has to
   carry state from the end of one line to the start of the next. */
const LIMIT: usize = 400;
/* An unterminated block runs to the end of the file:
const HIDDEN: usize = 0;
`;

// No path at all, which is what a call that named no file hands over:
// no trail is drawn and nothing is coloured, and the line numbers are
// the only thing left to read by.
const UNNAMED = `error: the city refused to start
  because: CONFIG.toml names a provider with no key filed for it
  try: sprawling attach --provider zenmux
`;

export function Produced() {
  return (
    <>
      <Case label="calls · one command">
        <Calls calls={ONE_COMMAND} run={RUN} />
      </Case>

      <Case label="calls · explored, wrote and ran">
        <Calls calls={A_WAVE} run={RUN} />
      </Case>

      <Case label="calls · a class of work the fold has no verb for">
        <Calls calls={NO_VERB} run={RUN} />
      </Case>

      <Case label="artifact · a file and what a command printed">
        <Panel artifacts={panel(A_FILE, A_TERMINAL)} />
      </Case>

      <Case label="artifact · a file this build cannot colour">
        <Panel artifacts={panel(UNKNOWN_KIND, null)} />
      </Case>

      <Case label="artifact · a run that only ran commands">
        <Panel artifacts={panel(null, A_TERMINAL)} />
      </Case>

      <Case label="code · line numbers and a trail that wraps">
        <Frame>
          <Code path={TRAILED} text={A_FILE.output?.head ?? ""} />
        </Frame>
      </Case>

      <Case label="code · a comment crossing lines, and one that never closes">
        <Frame>
          <Code path="crates/kernel/src/limit.rs" text={BLOCK_COMMENT} />
        </Frame>
      </Case>

      <Case label="code · no file was named, so no trail and no colour">
        <Frame>
          <Code path="" text={UNNAMED} />
        </Frame>
      </Case>
    </>
  );
}

// The room the talk page gives the panel: a column at a narrow window
// and a second column beside the thread once there is width for one.
// Without it the panel has no height to divide between its two halves
// and neither half would scroll.
function Panel(props: { readonly artifacts: Artifacts }) {
  return (
    <div class="flex h-output min-h-0 flex-col @lg/page:flex-row">
      <Artifact artifacts={props.artifacts} open />
    </div>
  );
}

// The room the panel gives the code view. Stated here so the three code
// fixtures are measured at a height a person actually meets, rather
// than each growing to the length of whatever file it holds.
function Frame(props: { readonly children: JSX.Element }) {
  return <div class="flex h-output flex-col rounded-card border border-g3">{props.children}</div>;
}
