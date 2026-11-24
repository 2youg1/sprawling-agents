// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { Option, Schema } from "effect";

import { Address } from "../../wire";
import type { AxError, HaltScope, LogLine, RunId, Seq, TimeMs } from "../../wire";
import type { Doing } from "../doing";
import type { Probed } from "../probed";
import type { PolicyChange } from "../reading";
import { readRunId } from "../run_id";

export interface RunBelief {
  readonly run: RunId;
  readonly addr: Address | null;
  readonly started: TimeMs | null;

  readonly task: string | null;
  readonly goal: string | null;
  readonly sessionBegan?: Seq | undefined;
  readonly imageInput?: boolean | undefined;
  readonly lastSeq: Seq;
  readonly doing: Doing;

  readonly model: string | null;

  readonly pr: string | null;

  readonly ask: string | null;

  readonly local: boolean;

  saying: string;

  thinking: string;
}

export interface Belief {
  runs: Record<string, RunBelief>;

  live: readonly RunBelief[];

  rooms: ReadonlyMap<string, readonly RunId[]>;

  cancelled: number;
  halted: HaltScope[];

  haltedAt: Seq;

  refusal: AxError | null;

  notices: Notice[];
  city: string | null;

  sessions: Record<string, Seq>;

  policies: Record<string, PolicyChange>;

  probed: Probed | null;

  logs: LogLine[];
}

export const LOG_WINDOW = 500;

export interface Notice {
  readonly error: AxError;

  seen: boolean;

  readonly at: TimeMs;

  readonly key: string;

  count: number;

  readonly about: Address | RunId | null;
}

export const NOTICE_WINDOW = 50;

function noticeKey(error: AxError): string {
  const legacy = `${error.code}\u0000${error.subject}`;
  return error.request == null ? legacy : `${legacy}\u0000${error.request}`;
}

const readAddress = Schema.decodeOption(Address);

function about(subject: string): Address | RunId | null {
  const run = readRunId(subject);
  if (Option.isSome(run)) return run.value;
  return Option.getOrElse(readAddress(subject), () => null);
}

export function merged(notices: readonly Notice[], error: AxError, at: TimeMs): Notice[] {
  const key = noticeKey(error);
  const held = notices.find((each) => each.key === key);
  if (held === undefined) {
    const out = [
      ...notices,
      { error, seen: false, at, key, count: 1, about: about(error.subject) },
    ];
    return out.length > NOTICE_WINDOW ? out.slice(out.length - NOTICE_WINDOW) : out;
  }
  return notices.map((each) =>
    each.key === key ? { ...each, error, seen: false, count: each.count + 1 } : each,
  );
}
