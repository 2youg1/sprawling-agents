// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { For, Match, Show, Switch, createMemo } from "solid-js";
import { readAnswer } from "../core/answered";
import { QUERIES } from "../core/asking";
import { fill, say, type Key, type Lang } from "../core/lang";
import { storeValue } from "../core/solid_store";
import { count, usd } from "../core/time";
import { ui } from "../ui";
import type { CostAnswer, Renewals, UnpricedCalls } from "../wire";
import AskMayor from "./city/ask_mayor.solid";
import EmptyState from "./parts/empty.solid";
import Page from "./parts/page.solid";
import Unanswered from "./parts/unanswered.solid";
import { costReading, renewalsSent } from "./pricing";

const CUTS = ["by_run", "by_actor", "by_segment", "by_tool", "by_skill"] as const;
const TITLES: Record<(typeof CUTS)[number], Key> = {
  by_run: "cost_by_run",
  by_actor: "cost_by_actor",
  by_segment: "cost_by_segment",
  by_tool: "cost_by_tool",
  by_skill: "cost_by_skill",
};

function Unpriced(props: { readonly value: UnpricedCalls; readonly lang: Lang }) {
  return (
    <p class="text-note text-text-quiet">
      {fill(say(props.lang, "cost_unpriced"), {
        calls: count(props.value.calls),
        tokens: count(props.value.tokens),
      })}
    </p>
  );
}

function Renewed(props: { readonly value: Renewals; readonly lang: Lang }) {
  return (
    <p class="text-note text-text-quiet">
      {fill(say(props.lang, "cost_renewals"), {
        spent: usd(props.value.billed),
        sent: count(renewalsSent(props.value)),
        late: count(props.value.late),
      })}
    </p>
  );
}

function Breakdowns(props: { readonly answer: CostAnswer; readonly lang: Lang }) {
  return (
    <div class="grid grid-fit gap-x-gutter gap-y-wide">
      <For each={CUTS}>
        {(cut) => (
          <section
            class="flex min-w-0 flex-col gap-snug border-t border-edge pt-base"
            aria-label={say(props.lang, TITLES[cut])}
          >
            <h2 class="text-note text-text-faint">{say(props.lang, TITLES[cut])}</h2>
            <Show
              when={props.answer[cut].length > 0}
              fallback={<p class="text-note text-text-faint">—</p>}
            >
              <ul class="text-note">
                <For
                  each={[...props.answer[cut]].sort((left, right) => right[1] - left[1])}
                  keyed={(entry) => entry[0]}
                >
                  {(entry) => (
                    <li class="settled-row py-tight">
                      <div class="flex justify-between gap-base">
                        <span class="truncate font-mono text-text-quiet">{entry()[0]}</span>
                        <span class="shrink-0 figure text-text">{usd(entry()[1])}</span>
                      </div>
                      <div class="summary mt-tight h-hair overflow-hidden rounded-pill bg-track">
                        <div
                          class="h-full bg-accent"
                          style={{
                            width: `${String(props.answer.total > 0 ? (entry()[1] / props.answer.total) * 100 : 0)}%`,
                          }}
                        />
                      </div>
                    </li>
                  )}
                </For>
              </ul>
            </Show>
          </section>
        )}
      </For>
    </div>
  );
}

export default function Cost(props: { readonly rank?: "page" | "section" | undefined }) {
  const u = ui();
  const lang = storeValue(u.lang);
  const asked = storeValue(u.conn.asking.ask(QUERIES.cost));
  const read = createMemo(() =>
    readAnswer(asked(), (held) => ("cost" in held ? held.cost : undefined)),
  );
  const unavailable = createMemo(() => {
    const state = read();
    return state.kind === "unavailable" ? state.query : undefined;
  });
  const answer = createMemo(() => {
    const state = read();
    return state.kind === "held" ? state.value : undefined;
  });
  const kind = createMemo(() => {
    const held = answer();
    return held === undefined ? undefined : costReading(held).kind;
  });
  return (
    <Page
      title={say(lang(), "cost_title")}
      rank={props.rank}
      aside={
        <Show when={kind() === "idle" ? undefined : answer()}>
          {(held) => (
            <span class="flex items-baseline gap-snug">
              <span class="text-note text-text-faint">{say(lang(), "cost_total")}</span>
              <span class="text-figure font-figure figure">
                {kind() === "priced" ? usd(held().total) : say(lang(), "cost_none")}
              </span>
            </span>
          )}
        </Show>
      }
    >
      <Switch fallback={<p class="text-text-faint">…</p>}>
        <Match when={unavailable()}>
          {(query) => <Unanswered query={query()} asked={QUERIES.cost} />}
        </Match>
        <Match when={kind() === "idle"}>
          <EmptyState missing="cost_empty" seat="region" action={<AskMayor />} />
        </Match>
        <Match when={answer()}>
          {(held) => (
            <>
              <Show when={held().unpriced.calls > 0}>
                <Unpriced value={held().unpriced} lang={lang()} />
              </Show>
              <Show when={kind() === "priced"}>
                <Show when={renewalsSent(held().renewals) > 0}>
                  <Renewed value={held().renewals} lang={lang()} />
                </Show>
                <Breakdowns answer={held()} lang={lang()} />
              </Show>
            </>
          )}
        </Match>
      </Switch>
    </Page>
  );
}
