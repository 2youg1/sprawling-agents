// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo, createUniqueId, For, Match, Switch } from "solid-js";

import { QUERIES } from "../../core/asking";
import { readAnswer } from "../../core/answered";
import { fill, say, type Key } from "../../core/lang";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import type { Cadence } from "../../wire";
import EmptyState from "../parts/empty.solid";
import Unanswered from "../parts/unanswered.solid";

const MINUTES_A_DAY = 1_440;
const ROW =
  "grid grid-cols-[minmax(0,10rem)_minmax(0,1fr)] gap-x-base gap-y-hair border-b border-edge py-snug";

function clock(minute: number): string {
  const hour = Math.floor(minute / 60) % 24;
  return `${String(hour).padStart(2, "0")}:${String(minute % 60).padStart(2, "0")}`;
}

export default function Automation() {
  const u = ui();
  const lang = storeValue(u.lang);
  const asked = storeValue(u.conn.asking.ask(QUERIES.automation));
  const read = createMemo(() =>
    readAnswer(asked(), (answer) => ("automation" in answer ? answer.automation : undefined)),
  );
  const held = () => {
    const answer = read();
    return answer.kind === "held" ? answer.value : undefined;
  };
  const unavailable = () => {
    const answer = read();
    return answer.kind === "unavailable" ? answer.query : undefined;
  };
  const uid = createUniqueId();
  const word = (key: Key) => say(lang(), key);

  function weekday(minute: number): string {
    const day = Math.floor(minute / MINUTES_A_DAY);
    return new Intl.DateTimeFormat(lang(), { weekday: "short", timeZone: "UTC" }).format(
      Date.UTC(2024, 0, 1 + day),
    );
  }

  function cadence(each: Cadence): string {
    if ("every_minutes" in each)
      return fill(word("automation_every"), { n: String(each.every_minutes.minutes) });
    if ("daily_at" in each)
      return fill(word("automation_daily"), { at: clock(each.daily_at.minute) });
    return fill(word("automation_weekly"), {
      day: weekday(each.weekly_at.minute),
      at: clock(each.weekly_at.minute % MINUTES_A_DAY),
    });
  }

  return (
    <Switch>
      <Match when={unavailable()}>
        {(missing) => <Unanswered query={missing()} asked={QUERIES.automation} />}
      </Match>
      <Match when={held()}>
        {(answer) => (
          <div class="flex flex-col gap-wide">
            <For each={answer().unreadable}>
              {(file) => (
                <p class="asks rounded-card bg-raised px-base py-snug font-mono text-note text-text-quiet">
                  {file}
                </p>
              )}
            </For>
            <section class="flex flex-col" aria-labelledby={`${uid}-jobs`}>
              <h3 id={`${uid}-jobs`} class="pb-tight text-label font-label text-text">
                {word("automation_jobs")}
              </h3>
              <For
                each={answer().jobs}
                keyed={(job) => job.name}
                fallback={<EmptyState missing={"automation_no_jobs"} />}
              >
                {(job) => (
                  <div class={ROW}>
                    <span class="truncate font-mono text-note text-text">{job().name}</span>
                    <span class="truncate text-note text-text-quiet">{`${job().addr} · ${cadence(job().cadence)}`}</span>
                    <span />
                    <span class="text-note text-text-faint">{job().task}</span>
                  </div>
                )}
              </For>
            </section>
            <section class="flex flex-col" aria-labelledby={`${uid}-sources`}>
              <h3 id={`${uid}-sources`} class="pb-tight text-label font-label text-text">
                {word("automation_sources")}
              </h3>
              <For
                each={answer().sources}
                keyed={(source) => source.name}
                fallback={<EmptyState missing={"automation_no_sources"} />}
              >
                {(source) => (
                  <div class={ROW}>
                    <span class="truncate font-mono text-note text-text">{source().name}</span>
                    <span class="truncate text-note text-text-quiet">
                      {`${source().addr} · ${word(source().starts_work ? "automation_starts_work" : "automation_tells")}`}
                    </span>
                    <span />
                    <code class="truncate font-mono text-note text-text-faint">
                      {source().matches}
                    </code>
                  </div>
                )}
              </For>
            </section>
            <p class="text-note text-text-faint">{word("automation_by_hand")}</p>
          </div>
        )}
      </Match>
    </Switch>
  );
}
