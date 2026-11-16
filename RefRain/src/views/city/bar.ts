// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors


import type { Key, Lang } from "../../core/lang";
import { say } from "../../core/lang";
import { MAYOR, toFragment } from "../../core/route";
import type { View } from "../../core/route";
import { count, usd } from "../../core/time";
import type { MetricsAnswer } from "../../wire";


interface Figure {
  readonly label: Key;
  readonly of: (held: MetricsAnswer) => number;
  readonly to: View | null;
  readonly waits: boolean;
}


const FIGURES: readonly Figure[] = [
  {
    label: "metric_events",
    of: (held) => held.events,
    to: { kind: "record", lens: "ledger" },
    waits: false,
  },
  { label: "metric_runs_active", of: (held) => held.runs_active, to: null, waits: false },
  { label: "metric_runs_frozen", of: (held) => held.runs_frozen, to: null, waits: false },
  {
    label: "metric_approvals",
    of: (held) => held.approvals_waiting,
    to: { kind: "talk", address: MAYOR },
    waits: true,
  },
  { label: "metric_signals", of: (held) => held.signals_waiting, to: null, waits: true },
  {
    label: "metric_discards",
    of: (held) => held.discards_outstanding,
    to: { kind: "record", lens: "bin" },
    waits: true,
  },
];


export interface ReadingLook {
  readonly key: string;
  readonly label: string;
  readonly value: string;
  readonly tone: "alert" | "plain";
  
  
  readonly wire: { readonly href: string } | null;
}

export interface BarLook {
  readonly label: string;
  readonly readings: readonly ReadingLook[];
}


export function barLookOf(
  metrics: MetricsAnswer | null,
  spent: number | null,
  lang: Lang,
): BarLook | null {
  if (metrics === null && spent === null) return null;
  const figures = metrics === null ? [] : FIGURES.map((figure) => readingOf(figure, metrics, lang));
  const total: readonly ReadingLook[] =
    spent === null
      ? []
      : [
          {
            key: "cost_total",
            label: say(lang, "cost_total"),
            value: usd(spent),
            tone: "plain",
            wire: { href: toFragment({ kind: "cost" }) },
          },
        ];
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
