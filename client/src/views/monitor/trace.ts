// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { Option, Schema } from "effect";

import type { Call, Outcome, Seq, Turn } from "../../wire";
import { lastedOf } from "../talk/timing";

export type CeilingWhy = "platform" | "not_delegated" | "refused" | "unjoined";

export type Ceiling =
  | { readonly state: "hit"; readonly limit: number }
  | { readonly state: "unapplied"; readonly limit: number; readonly why: CeilingWhy }
  | { readonly state: "unread"; readonly limit: number };

export interface CeilingNote {
  readonly handle: string | null;
  readonly ceiling: Ceiling;
}

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

      readonly took: number | null;
      readonly ceilings: readonly CeilingNote[];
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

export type Target =
  { readonly kind: "file"; readonly path: string } | { readonly kind: "entry"; readonly at: Seq };

export interface Trace {
  readonly entries: readonly Entry[];

  readonly files: readonly Touched[];
  readonly latest: Target | null;
}

const CREATED_FROM = "--- /dev/null\n";
const CREATED_BASE = "new";

const Asked = Schema.Union([
  Schema.Struct({ command: Schema.String }),
  Schema.Struct({ python: Schema.String }),
  Schema.Struct({
    arm: Schema.Union([
      Schema.Struct({ shell: Schema.Struct({ text: Schema.String }) }),
      Schema.Struct({
        program: Schema.Struct({
          path: Schema.String,
          args: Schema.optional(Schema.Array(Schema.String)),
        }),
      }),
      Schema.Struct({ python: Schema.Struct({ code: Schema.String }) }),
    ]),
  }),
]);

function askedText(asked: typeof Asked.Type): string {
  if ("command" in asked) return asked.command;
  if ("python" in asked) return asked.python;
  const { arm } = asked;
  if ("shell" in arm) return arm.shell.text;
  if ("program" in arm) return [arm.program.path, ...(arm.program.args ?? [])].join(" ");
  return arm.python.code;
}

const MemoryCeiling = Schema.Union([
  Schema.Struct({ state: Schema.Literals(["hit", "unread"]), limit_bytes: Schema.Number }),
  Schema.Struct({
    state: Schema.Literal("unapplied"),
    limit_bytes: Schema.Number,
    why: Schema.Literals(["platform", "not_delegated", "refused", "unjoined"]),
  }),
]);

const Ran = Schema.Struct({
  stdout: Schema.optional(Schema.String),
  stderr: Schema.optional(Schema.String),
  outcome: Schema.optional(Schema.String),
  memory_ceiling: Schema.optional(MemoryCeiling),
  background: Schema.optional(
    Schema.Array(
      Schema.Struct({ handle: Schema.String, memory_ceiling: Schema.optional(MemoryCeiling) }),
    ),
  ),
});

const Edited = Schema.Struct({
  path: Schema.String,
  base_version: Schema.optional(Schema.String),
  diff: Schema.String,
});

const read = <A>(schema: Schema.Decoder<A>, text: string | undefined): Option.Option<A> =>
  text === undefined
    ? Option.none()
    : Schema.decodeUnknownOption(Schema.fromJsonString(schema))(text);

export function traceOf(turns: readonly Turn[]): Trace {
  const calls = turns.flatMap((turn) => turn.calls);
  const files = new Map<string, Touched>();
  const entries: Entry[] = [];
  let latest: Target | null = null;
  for (const call of calls) {
    const edited =
      call.tool === "edit" && call.outcome === "answered"
        ? read(Edited, call.output?.head)
        : Option.none();
    if (Option.isSome(edited)) {
      const { path, base_version: base, diff } = edited.value;
      const before = files.get(path);
      files.set(path, {
        path,
        how:
          before?.how ??
          (base === CREATED_BASE || diff.startsWith(CREATED_FROM) ? "added" : "modified"),
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
  const text = Option.match(read(Asked, asked), { onNone: () => asked, onSome: askedText });
  const ran = read(Ran, call.output?.head);
  const raw = call.output?.head ?? "";
  return {
    kind: "command",
    at: call.at,
    text,
    stdout: Option.match(ran, { onNone: () => raw, onSome: (r) => r.stdout ?? "" }),
    stderr: Option.match(ran, { onNone: () => "", onSome: (r) => r.stderr ?? "" }),
    ending: endingOf(call, ran),
    cut: call.output?.cut ?? 0,
    took: lastedOf(call),
    ceilings: Option.match(ran, { onNone: () => [], onSome: ceilingsOf }),
  };
}

function ceilingsOf(ran: typeof Ran.Type): readonly CeilingNote[] {
  const own =
    ran.memory_ceiling === undefined
      ? []
      : [{ handle: null, ceiling: ceilingOf(ran.memory_ceiling) }];
  const rows = (ran.background ?? []).flatMap((row) =>
    row.memory_ceiling === undefined
      ? []
      : [{ handle: row.handle, ceiling: ceilingOf(row.memory_ceiling) }],
  );
  return [...own, ...rows];
}

function ceilingOf(read: typeof MemoryCeiling.Type): Ceiling {
  return read.state === "unapplied"
    ? { state: read.state, limit: read.limit_bytes, why: read.why }
    : { state: read.state, limit: read.limit_bytes };
}

function endingOf(call: Call, ran: Option.Option<typeof Ran.Type>): Ending {
  if (call.outcome === "waiting") return { kind: "running" };
  if (call.exit_code !== undefined && call.exit_code !== null)
    return { kind: "code", code: call.exit_code };
  const why = Option.isSome(ran) ? ran.value.outcome : undefined;
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
