// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createContext, getOwner, useContext } from "solid-js";
import { derived, get, readable, writable } from "./core/store";
import type { Readable, Writable } from "./core/store";

import { QUERIES } from "./core/asking";
import { FIRST_POLICY } from "./core/commands";
import { createBelief } from "./core/belief";
import type { Lang } from "./core/lang";
import type { LinkState } from "./core/link";
import { loadPreferences } from "./core/prefs";
import type { PreferenceDoor } from "./core/prefs";
import { memory } from "./core/rows";
import type { AddressBar, View } from "./core/route";
import { go } from "./core/route";
import type { Connection } from "./core/socket";
import { keepTags } from "./core/tags";
import type { Tagging } from "./core/tags";
import type { Answer, ApprovalItem, Command, Effort, RunPolicy } from "./wire";
import { keepBranches } from "./views/talk/branching";
import type { Branches } from "./views/talk/branching";
import { NOT_CONVERSING } from "./views/talk/handing";
import type { Conversing } from "./views/talk/handing";

export interface Ui {
  readonly conn: Connection;
  readonly prefs: PreferenceDoor;
  readonly tags: Tagging;
  readonly lang: Readable<Lang>;
  readonly effort: Readable<Effort | null>;
  readonly policy: Readable<RunPolicy>;
  readonly conversing: Writable<Conversing>;
  readonly branches: Branches;
  readonly approvals: Readable<readonly ApprovalItem[] | undefined>;
  readonly bar: AddressBar;
  readonly origin: string;
  readonly credential: () => string | null;
  readonly now: () => number;
  readonly chooseEffort: (level: Effort | null) => void;
  readonly choosePolicy: (policy: RunPolicy) => void;
  readonly go: (view: View) => void;
  readonly send: (command: Command) => boolean;
  readonly hearing: () => boolean;
}

export interface Opening {
  readonly conn: Connection;
  readonly prefs: PreferenceDoor;
  readonly bar: AddressBar;
  readonly origin: string;
  readonly credential: () => string | null;
  readonly now: () => number;
}

export function readied(value: Opening): Ui {
  const effort = writable<Effort | null>(null);
  const policy = writable<RunPolicy>(FIRST_POLICY);
  const hearing = value.conn.asking.ask(QUERIES.endpoints);
  const going = (view: View): void => {
    go(value.bar, view);
  };
  return {
    ...value,
    lang: derived(value.prefs.held, (held) => held.lang),
    effort,
    policy,
    conversing: writable<Conversing>(NOT_CONVERSING),
    branches: keepBranches({ belief: value.conn.belief, send: value.conn.command, go: going }),
    tags: keepTags(value.conn),
    approvals: derived(value.conn.asking.ask(QUERIES.approvals), (answer) =>
      answer !== undefined && "approvals" in answer ? answer.approvals.items : undefined,
    ),
    chooseEffort: effort.set,
    choosePolicy: policy.set,
    go: going,
    send: value.conn.command,
    hearing: () => {
      const answer: Answer | undefined = get(hearing);
      return (
        answer !== undefined &&
        "endpoints" in answer &&
        answer.endpoints.chosen.some((each) => each.tag === "transcribe")
      );
    },
  };
}

let opened: Ui | null = null;

export function setUi(value: Opening): void {
  opened = readied(value);
}

export const UiScope = createContext<Ui | null>(null);

export function scopedUi(): Ui | undefined {
  return getOwner() === null ? undefined : (useContext(UiScope) ?? undefined);
}

function disconnected(): Opening {
  return {
    conn: {
      state: readable<LinkState>({ kind: "idle" }),
      belief: createBelief(() => 0).belief,
      live: readable({}),
      asking: {
        ask: () => readable<Answer | undefined>(undefined),
        refresh: () => undefined,
        answered: () => undefined,
        invalidate: () => undefined,
        reconnected: () => undefined,
        resumed: () => undefined,
      },
      unsent: readable(0),
      receipts: readable(new Map()),
      command: () => false,
      retry: () => undefined,
      closeCity: () => false,
      closing: readable(null),
      dismissRefusal: () => undefined,
      markNoticesSeen: () => undefined,
      monitor: {
        samples: readable([]),
        watch: () => () => undefined,
        watchSummary: () => () => undefined,
        setBeat: () => undefined,
      },
    },
    prefs: loadPreferences(memory(), ""),
    bar: { hash: "" },
    origin: "",
    credential: () => null,
    now: () => 0,
  };
}

export function galleryUi(outer: Ui): Ui {
  const fixture = disconnected();
  fixture.prefs.adopt(get(outer.prefs.held), []);
  return readied({ ...fixture, now: outer.now });
}

export function ui(): Ui {
  return scopedUi() ?? opened ?? readied(disconnected());
}
