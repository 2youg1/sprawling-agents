// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// One run read as what a person watching over its shoulder would see:
// a terminal record of every call in the order it was made, and a
// column of the files it changed, cut to the stretches that moved.
//
// Everything here comes from the turns the wire already carries. The
// runtime writes an exec result as `{stdout, stderr, exit_code}` and an
// edit result as `{path, base_version, diff}` (crates/runtime/src/tools/
// exec/outcome.rs and edit.rs), and the channel hands each over as
// compact JSON on one line, so the line cut never falls inside one.
// A result that does not read as that shape is still drawn, as its raw
// head, because a record that drops what it cannot parse hides the
// calls most worth looking at. A command's duration is the span between
// the moment the call was made and the moment it was answered, and a
// call still running has none yet.

import { Option, Schema } from "effect";

import type { Call, Outcome, Seq, Turn } from "../../wire";
import { tookOf } from "../run/lanes";

export type Ending =
  | { readonly kind: "code"; readonly code: number }
  | { readonly kind: "stopped"; readonly why: string }
  | { readonly kind: "running" }
  | { readonly kind: "unread" };

export type Entry =
  | {
      readonly kind: "command";
      readonly at: Seq;
      readonly text: string;
      readonly stdout: string;
      readonly stderr: string;
      readonly ending: Ending;
      readonly cut: number;
      // Milliseconds from call to answer; null while the call runs.
      readonly took: number | null;
    }
  | {
      readonly kind: "call";
      readonly at: Seq;
      readonly tool: string;
      readonly subject: string;
      readonly asked: string;
      readonly answered: string;
      readonly outcome: Outcome;
    };

export type Sign = "kept" | "added" | "removed";

export interface Line {
  readonly sign: Sign;
  readonly old: number | null;
  readonly new: number | null;
  readonly text: string;
}

export interface Hunk {
  readonly lines: readonly Line[];
}

export interface Touched {
  readonly path: string;
  readonly how: "added" | "modified";
  readonly at: Seq;
  readonly hunks: readonly Hunk[];
}

// Where following lands: the file the agent changed last, or the call
// it made last when that was not an edit.
export type Target = { readonly kind: "file"; readonly path: string } | { readonly kind: "entry"; readonly at: Seq };

export interface Trace {
  readonly entries: readonly Entry[];
  // Newest first, so the file being worked on is the one on top.
  readonly files: readonly Touched[];
  readonly latest: Target | null;
}

// `crates/runtime/src/tools/edit.rs` writes this as the base of a file
// the call created.
const CREATES = "new";

const Arm = Schema.Struct({
  arm: Schema.Union(
    Schema.Struct({ shell: Schema.Struct({ text: Schema.String }) }),
    Schema.Struct({
      program: Schema.Struct({ path: Schema.String, args: Schema.optional(Schema.Array(Schema.String)) }),
    }),
    Schema.Struct({ python: Schema.Struct({ code: Schema.String }) }),
  ),
});

const Ran = Schema.Struct({
  stdout: Schema.optional(Schema.String),
  stderr: Schema.optional(Schema.String),
  exit_code: Schema.optional(Schema.Int),
  outcome: Schema.optional(Schema.String),
});

const Edited = Schema.Struct({ path: Schema.String, base_version: Schema.String, diff: Schema.String });

const read = <A, I>(schema: Schema.Schema<A, I>, text: string | undefined): Option.Option<A> =>
  text === undefined ? Option.none() : Schema.decodeUnknownOption(Schema.parseJson(schema))(text);

export function traceOf(turns: readonly Turn[]): Trace {
  const calls = turns.flatMap((turn) => turn.calls);
  const files = new Map<string, Touched>();
  const entries: Entry[] = [];
  let latest: Target | null = null;
  for (const call of calls) {
    const edited = call.tool === "edit" && call.outcome === "answered" ? read(Edited, call.output?.head) : Option.none();
    if (Option.isSome(edited)) {
      const { path, base_version: base, diff } = edited.value;
      const before = files.get(path);
      files.set(path, {
        path,
        how: before?.how ?? (base === CREATES ? "added" : "modified"),
        at: call.at,
        hunks: [...(before?.hunks ?? []), ...hunksOf(diff)],
      });
      latest = { kind: "file", path };
    } else {
      entries.push(call.tool === "exec" ? commandOf(call) : callOf(call));
      latest = { kind: "entry", at: call.at };
    }
  }
  return { entries, files: [...files.values()].sort((a, b) => b.at - a.at), latest };
}

export function commandOf(call: Call): Entry {
  const asked = call.arguments?.head ?? "";
  const text = Option.match(read(Arm, asked), {
    onNone: () => asked,
    onSome: ({ arm }) =>
      "shell" in arm
        ? arm.shell.text
        : "program" in arm
          ? [arm.program.path, ...(arm.program.args ?? [])].join(" ")
          : arm.python.code,
  });
  const ran = read(Ran, call.output?.head);
  const raw = call.output?.head ?? "";
  return {
    kind: "command",
    at: call.at,
    text,
    stdout: Option.match(ran, { onNone: () => raw, onSome: (r) => r.stdout ?? "" }),
    stderr: Option.match(ran, { onNone: () => "", onSome: (r) => r.stderr ?? "" }),
    ending: endingOf(call.outcome, ran),
    cut: call.output?.cut ?? 0,
    took: tookOf(call),
  };
}

function endingOf(outcome: Outcome, ran: Option.Option<typeof Ran.Type>): Ending {
  if (outcome === "waiting") return { kind: "running" };
  if (Option.isNone(ran)) return { kind: "unread" };
  const { exit_code: code, outcome: why } = ran.value;
  if (code !== undefined) return { kind: "code", code };
  return why === undefined ? { kind: "unread" } : { kind: "stopped", why };
}

function callOf(call: Call): Entry {
  return {
    kind: "call",
    at: call.at,
    tool: call.tool,
    subject: call.subject ?? "",
    asked: call.arguments?.head ?? "",
    answered: call.output?.head ?? "",
    outcome: call.outcome,
  };
}

const HEADER = /^@@ -(\d+)(?:,\d+)? \+(\d+)(?:,\d+)? @@/;

// A unified diff cut into hunks, each line carrying the number it has
// on the side it exists on. Lines before the first header are the file
// names, which the column already states. A line that opens with a
// backslash is the "\ No newline at end of file" marker, which belongs
// to neither side and so is neither drawn nor counted.
function hunksOf(diff: string): readonly Hunk[] {
  const hunks: Line[][] = [];
  let old = 0;
  let now = 0;
  for (const text of diff.split("\n")) {
    const header = HEADER.exec(text);
    const lines = hunks.at(-1);
    if (header !== null) {
      old = Number(header[1]);
      now = Number(header[2]);
      hunks.push([]);
    } else if (lines === undefined || text === "" || text.startsWith("\\")) {
      continue;
    } else if (text.startsWith("-")) {
      lines.push({ sign: "removed", old, new: null, text: text.slice(1) });
      old += 1;
    } else if (text.startsWith("+")) {
      lines.push({ sign: "added", old: null, new: now, text: text.slice(1) });
      now += 1;
    } else {
      lines.push({ sign: "kept", old, new: now, text: text.slice(1) });
      old += 1;
      now += 1;
    }
  }
  return hunks.map((lines) => ({ lines }));
}
