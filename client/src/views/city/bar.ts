// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// How the city stands, as one row of figures under the page header:
// six counts and what the city has spent, already in words. Every fact
// is stated once and in one place; the city's name and the control that
// stops or releases it are the page header's (client/Spec.lean §4-50).
// The look draws `BarLook` and decides nothing (client D95).
//
// **The six figures come from `Query::Metrics` and from nothing
// else.** Each of them is also provable from a view this page could
// have asked for separately - the approval queue, the city view, the
// recycle bin - and a bar that assembled them that way would hold six
// numbers under six refresh rules, which is how two figures on one
// line start disagreeing. That the city is stopped is not among them:
// the shell states it once, as a banner over every page.

import type { Key, Lang } from "../../core/lang";
import { say } from "../../core/lang";
import { MAYOR, toFragment } from "../../core/route";
import type { View } from "../../core/route";
import { count, usd } from "../../core/time";
import type { MetricsAnswer } from "../../wire";

// One figure on the bar: the word it is offered under, the field it
// reads, the page a person acts on it from, and whether a figure
// above zero is something waiting for them rather than something the
// city is getting on with.
interface Figure {
  readonly label: Key;
  readonly of: (held: MetricsAnswer) => number;
  readonly to: View | null;
  readonly waits: boolean;
}

// The six, in the order a person reads them: what the city has
// recorded, what is moving, what has stopped, and the three queues.
//
// `buildings` is the seventh field and is deliberately not here: the
// drawing under this bar and the list beside it are already the count
// of buildings, and a number repeating them would be a second home
// for one fact.
const FIGURES: readonly Figure[] = [
  { label: "metric_events", of: (held) => held.events, to: { kind: "record", lens: "ledger" }, waits: false },
  { label: "metric_runs_active", of: (held) => held.runs_active, to: null, waits: false },
  { label: "metric_runs_frozen", of: (held) => held.runs_frozen, to: null, waits: false },
  { label: "metric_approvals", of: (held) => held.approvals_waiting, to: { kind: "talk", address: MAYOR }, waits: true },
  { label: "metric_signals", of: (held) => held.signals_waiting, to: null, waits: true },
  { label: "metric_discards", of: (held) => held.discards_outstanding, to: { kind: "record", lens: "bin" }, waits: true },
];

// One figure and its word. A figure somebody has to answer is in the
// alert tone only while it is above zero, so the bar of a city with
// nothing outstanding carries no red at all.
export interface ReadingLook {
  readonly key: string;
  readonly label: string;
  readonly value: string;
  readonly tone: "alert" | "plain";
  // Spread on the figure's link to the page a person acts on it from;
  // `null` where the figure has no such page.
  readonly wire: { readonly href: string } | null;
}

export interface BarLook {
  readonly label: string;
  readonly readings: readonly ReadingLook[];
}

// `null` before the city has answered either question: an empty grid
// would still draw its bottom rule, a second line under the page head
// with nothing between the two.
export function barLookOf(metrics: MetricsAnswer | null, spent: number | null, lang: Lang): BarLook | null {
  if (metrics === null && spent === null) return null;
  const figures = metrics === null ? [] : FIGURES.map((figure) => readingOf(figure, metrics, lang));
  const total: readonly ReadingLook[] =
    spent === null
      ? []
      : [{ key: "cost_total", label: say(lang, "cost_total"), value: usd(spent), tone: "plain", wire: { href: toFragment({ kind: "cost" }) } }];
  return { label: say(lang, "city_bar"), readings: [...figures, ...total] };
}

function readingOf(figure: Figure, metrics: MetricsAnswer, lang: Lang): ReadingLook {
  const n = figure.of(metrics);
  return {
    key: figure.label,
    label: say(lang, figure.label),
    value: count(n),
    tone: figure.waits && n > 0 ? "alert" : "plain",
    wire: figure.to === null ? null : { href: toFragment(figure.to) },
  };
}
