// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What the chosen session's timeline look is given
// (`timeline.look.svelte`), and how the seat (`timeline.svelte`) reads a
// session's rounds into it (client/Spec.lean §7K): a turn, then what
// happened inside it in Ledger order - its calls and its checkpoints
// sorted together by the sequence that wrote them - each at the instant
// the Ledger gave it, with a date said above the first row of every day
// after the head's.

import type { Attachment } from "svelte/attachments";

import { fill, say } from "../../core/lang";
import type { Key, Lang } from "../../core/lang";
import { isoDay, isoTime, kilo } from "../../core/time";
import type { Call, CommitAnswer, GitOid, Turn } from "../../wire";
import { tookOf, tookWords } from "../talk/timing";
import { HOLD } from "./drawn";
import { speedOf } from "./speed";

// Every row says its instant, and the date above it when it falls on a
// later day than the row before.
interface Dated {
  readonly key: string;
  readonly day: string | undefined;
  readonly at: string;
}

export interface TurnRow extends Dated {
  readonly kind: "turn";
  // "turn n", how soon the model answered, and the tokens in and out.
  readonly label: string;
  readonly subject: string;
  readonly measure: string;
}

// The bag spread on a call row's button: it opens the call on the
// right side.
export interface CallWire {
  readonly type: "button";
  readonly disabled: boolean;
  readonly onclick: () => void;
}

export interface CallRow extends Dated {
  readonly kind: "call";
  readonly tool: string;
  readonly subject: string;
  readonly waiting: boolean;
  readonly took: string;
  // A command's exit code, and whether it is the code of success.
  readonly exit: { readonly said: string; readonly ok: boolean } | undefined;
  // "running" or "failed"; absent for a call that answered.
  readonly outcome: { readonly said: string; readonly failed: boolean } | undefined;
  readonly wire: CallWire;
}

// The bag spread on a checkpoint row's button: it picks its commit.
export interface CheckpointWire {
  readonly type: "button";
  readonly "data-oid": GitOid;
  readonly "aria-current": "true" | undefined;
  readonly disabled: boolean;
  readonly onclick: () => void;
}

export interface CheckpointRow extends Dated {
  readonly kind: "checkpoint";
  readonly word: string;
  readonly short: string;
  readonly picked: boolean;
  // The commit's change against its first parent, which the row's
  // measure draws (`talk/produced.svelte`).
  readonly produced: { readonly base: GitOid; readonly head: GitOid } | undefined;
  readonly wire: CheckpointWire;
}

export type Row = TurnRow | CallRow | CheckpointRow;

export interface TimelineLook {
  // "Timeline", and the session's day as the head says it; "" when the
  // session has no turn yet.
  readonly title: string;
  readonly day: string;
  // The bag spread on the list, which hands the seat the element a
  // picked checkpoint is scrolled into view in.
  readonly list: Readonly<Record<symbol, Attachment<HTMLElement>>>;
  readonly rows: readonly Row[];
}

export interface TimelineHands {
  // The call on the right side; absent while the session holds no run.
  readonly open: ((at: Call["at"]) => void) | undefined;
  readonly pick: (commit: CommitAnswer) => void;
  readonly list: Attachment<HTMLElement>;
}

export interface TimelineInput {
  readonly lang: Lang;
  readonly turns: readonly Turn[];
  // The commits the page holds, for a checkpoint's time and parent.
  readonly commits: readonly CommitAnswer[];
  readonly picked: GitOid | null;
}

const OUTCOME: Record<Call["outcome"], Key | null> = { waiting: "world_running", answered: null, failed: "results_failed" };

const DASH = "—";

function instant(at: number | null): string {
  return at === null ? DASH : isoTime(at);
}

type Undated = Omit<TurnRow, "day"> | Omit<CallRow, "day"> | Omit<CheckpointRow, "day">;

export function timelineOf(input: TimelineInput, hands: TimelineHands): TimelineLook {
  const { lang, turns } = input;
  const day = turns[0] === undefined ? "" : isoDay(turns[0].t);
  const undated = turns.flatMap((turn): { readonly at: number | null; readonly row: Undated }[] => {
    const inside = [
      ...turn.calls.map((call) => ({ seq: call.at, made: callRow(call, input, hands) })),
      ...turn.notes.flatMap((note) =>
        "checkpointed" in note ? [{ seq: note.checkpointed.at, made: checkpointRow(note.checkpointed.oid, input, hands) }] : [],
      ),
    ].sort((a, b) => a.seq - b.seq);
    return [turnRow(turn, lang), ...inside.map((each) => each.made)];
  });
  // A session that ran past midnight says each new date once, above the
  // first row on it, so no time of day reads as the head's day when it
  // is not.
  let last = day;
  const rows = undated.map(({ at, row }): Row => {
    const on = at === null ? last : isoDay(at);
    const said = on === last ? undefined : fill(say(lang, "world_timeline_day"), { day: on });
    last = on;
    return { ...row, day: said };
  });
  return {
    title: say(lang, "world_timeline"),
    day: day === "" ? "" : fill(say(lang, "world_timeline_day"), { day }),
    list: { [HOLD]: hands.list },
    rows,
  };
}

function turnRow(turn: Turn, lang: Lang): { readonly at: number; readonly row: Omit<TurnRow, "day"> } {
  const speed = speedOf([turn]);
  const used = turn.used ?? null;
  return {
    at: turn.t,
    row: {
      kind: "turn",
      key: `t${String(turn.opened)}`,
      at: instant(turn.t),
      label: fill(say(lang, "run_turn_n"), { n: String(turn.number) }),
      subject: speed === null ? "" : `${say(lang, "talk_ttft")} ${fill(say(lang, "talk_took_ms"), { n: String(speed.ttft) })}`,
      measure: used === null ? "" : fill(say(lang, "world_tokens_io"), { input: kilo(used.input), output: kilo(used.output) }),
    },
  };
}

// How long a call took, read and written the way the thread's tool
// line reads and writes it (client/Spec.lean §4-59).
function callRow(call: Call, input: TimelineInput, hands: TimelineHands): { readonly at: number | null; readonly row: Omit<CallRow, "day"> } {
  const { lang } = input;
  const lasted = tookOf(call);
  const outcome = OUTCOME[call.outcome];
  const code = call.exit_code ?? null;
  const open = hands.open;
  const at = call.answered ?? null;
  return {
    at,
    row: {
      kind: "call",
      key: `c${String(call.at)}`,
      at: instant(at),
      tool: call.tool,
      subject: call.subject ?? "",
      waiting: call.outcome === "waiting",
      took: lasted === null ? "" : tookWords(lasted, lang),
      exit: code === null ? undefined : { said: fill(say(lang, "mon_exited"), { code: String(code) }), ok: code === 0 },
      outcome: outcome === null ? undefined : { said: say(lang, outcome), failed: call.outcome === "failed" },
      wire: {
        type: "button",
        disabled: open === undefined,
        onclick: () => {
          open?.(call.at);
        },
      },
    },
  };
}

function checkpointRow(
  oid: GitOid,
  input: TimelineInput,
  hands: TimelineHands,
): { readonly at: number | null; readonly row: Omit<CheckpointRow, "day"> } {
  const commit = input.commits.find((each) => each.oid === oid);
  const parent = commit?.parents?.[0];
  const picked = oid === input.picked;
  const at = commit?.at ?? null;
  return {
    at,
    row: {
      kind: "checkpoint",
      key: `k${oid}`,
      at: instant(at),
      word: say(input.lang, "world_checkpoint"),
      short: oid.slice(0, 7),
      picked,
      produced: parent === undefined ? undefined : { base: parent, head: oid },
      wire: {
        type: "button",
        "data-oid": oid,
        "aria-current": picked ? "true" : undefined,
        disabled: commit === undefined,
        onclick: () => {
          if (commit !== undefined) hands.pick(commit);
        },
      },
    },
  };
}
