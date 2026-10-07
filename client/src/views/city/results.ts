// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What the results city lists under its filter, already in words: one
// section per recency band, one row per run, and what the end of a row
// says - what a finished run produced and its pull request, what a run
// waiting for the person asks, or why a failed run stopped. The look
// draws `ResultsLook` and decides nothing (client D95).

import type { RunBelief } from "../../core/belief";
import type { Lang } from "../../core/lang";
import { fill, say } from "../../core/lang";
import { outcomeOf } from "../../core/results";
import type { Band, Outcome } from "../../core/results";
import { toFragment } from "../../core/route";
import { hhmm } from "../../core/time";
import type { RunId } from "../../wire";

// The end of a row. A finished run's produced figures are asked for by
// the row itself (`./produced.svelte`), so the look is handed the run
// rather than a number.
export type Tail =
  | { readonly kind: "produced"; readonly run: RunId; readonly pr: string | null }
  | { readonly kind: "asks"; readonly text: string }
  | { readonly kind: "stopped"; readonly text: string }
  | { readonly kind: "none" };

export interface ResultRowLook {
  readonly key: string;
  readonly at: string;
  readonly outcome: Outcome | null;
  readonly addr: string;
  readonly title: string;
  readonly tail: Tail;
  // Spread on the row's link to the run's page.
  readonly wire: { readonly href: string };
}

export interface ResultsLook {
  readonly bands: readonly {
    readonly key: string;
    readonly heading: string;
    readonly count: number;
    readonly rows: readonly ResultRowLook[];
  }[];
  // Said where the filter keeps no run at all.
  readonly none: string | null;
}

export function resultsLookOf(bands: readonly Band[], lang: Lang): ResultsLook {
  return {
    bands: bands.map((band) => ({
      key: band.recency,
      heading: say(lang, `results_${band.recency}`),
      count: band.runs.length,
      rows: band.runs.map((run) => rowOf(run, lang)),
    })),
    none: bands.length === 0 ? say(lang, "results_none") : null,
  };
}

function rowOf(run: RunBelief, lang: Lang): ResultRowLook {
  const outcome = outcomeOf(run);
  return {
    key: run.run,
    at: run.started === null ? "" : hhmm(run.started),
    outcome,
    addr: run.addr ?? "",
    title: run.task ?? run.run,
    tail: tailOf(run, outcome, lang),
    wire: { href: toFragment({ kind: "run", run: run.run }) },
  };
}

function tailOf(run: RunBelief, outcome: Outcome | null, lang: Lang): Tail {
  switch (outcome) {
    case "done":
      return { kind: "produced", run: run.run, pr: run.pr === null ? null : fill(say(lang, "results_row_pr"), { pr: run.pr }) };
    case "waiting":
      return run.ask === null ? { kind: "none" } : { kind: "asks", text: fill(say(lang, "results_row_ask"), { ask: run.ask }) };
    case "failed":
      return run.doing.kind === "frozen" && run.doing.completion !== null
        ? { kind: "stopped", text: run.doing.completion }
        : { kind: "none" };
    case "ended":
    case null:
      return { kind: "none" };
  }
}
