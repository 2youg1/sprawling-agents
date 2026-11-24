// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo, createSignal, Match, Show, Switch } from "solid-js";
import { readAnswer } from "../core/answered";
import { QUERIES } from "../core/asking";
import { halt, release } from "../core/commands";
import { say } from "../core/lang";
import { cityIsShut, CITY } from "../core/scope";
import { RELEASE_ALL } from "../core/slash";
import { storeValue } from "../core/solid_store";
import { ui } from "../ui";
import type { Address } from "../wire";
import AskMayor from "./city/ask_mayor.solid";
import Bar from "./city/bar.solid";
import { legendOf } from "./city/legend";
import Legend from "./city/legend.look.solid";
import Panel from "./city/panel.solid";
import Results from "./city/results.solid";
import Skyline from "./city/skyline.solid";
import Table from "./city/table.solid";
import Button from "./parts/button.solid";
import EmptyState from "./parts/empty.solid";
import Page from "./parts/page.solid";
import type { Rank } from "./parts/page";
import Segmented from "./parts/segmented.solid";
import Unanswered from "./parts/unanswered.solid";
import Board from "./runs/board.solid";
import { boardRuns } from "./runs/lineage";
import Showing from "./shared/showing.solid";
import WorkspaceNav from "./shared/workspace.solid";

const VIEWS = ["table", "drawing"] as const;
const WAITING = "…";

export interface CityPageProps {
  readonly rank?: Rank | undefined;
}

export default function CityPage(props: CityPageProps) {
  const u = ui();
  const lang = storeValue(u.lang);
  const belief = storeValue(u.conn.belief);
  const answer = storeValue(u.conn.asking.ask(QUERIES.city));
  const prefs = storeValue(u.prefs.held);
  const [picked, setPicked] = createSignal<Address | null>(null);
  const [view, setView] = createSignal<(typeof VIEWS)[number]>("table");

  const read = createMemo(() =>
    readAnswer(answer(), (held) => ("city" in held ? held.city : undefined)),
  );
  const city = () => {
    const held = read();
    return held.kind === "held" ? held.value : undefined;
  };
  const unavailable = () => {
    const held = read();
    return held.kind === "unavailable" ? held : undefined;
  };
  const pickedCity = () => {
    const current = city();
    const addr = picked();
    return current && addr ? { city: current, addr } : undefined;
  };
  const board = createMemo(() => ({ runs: boardRuns(belief().runs), now: u.now() }));
  const halted = () => cityIsShut(belief().halted);
  const hasBuildings = () => {
    const held = city();
    return held?.buildings.length ? held : undefined;
  };

  return (
    <Page
      title={belief().city ?? say(lang(), "nav_city")}
      rank={props.rank}
      aside={
        <Button
          label={halted() ? RELEASE_ALL : say(lang(), "city_stop")}
          tone={halted() ? "secondary" : "quiet"}
          onPress={() => {
            u.send(halted() ? release(CITY) : halt(CITY));
          }}
        />
      }
    >
      <Bar />
      <div class="grid grid-cols-11 items-start gap-x-gutter gap-y-wide narrow:grid-cols-1">
        <section
          class={[
            "flex min-w-0 flex-col gap-base narrow:col-span-full",
            pickedCity() ? "col-[1/9]" : "col-[1/12]",
          ]}
          aria-label={say(lang(), "city_buildings")}
        >
          <div class="flex flex-wrap items-center justify-between gap-base">
            <h2 class="text-note text-text-faint">{say(lang(), "city_buildings")}</h2>
            <div class="flex flex-wrap items-center gap-base">
              <WorkspaceNav />
              <Show when={prefs().showing !== "results"}>
                <Segmented
                  label={say(lang(), "city_view")}
                  options={VIEWS.map((value) => ({
                    value,
                    label: say(lang(), `city_view_${value}`),
                  }))}
                  held={view()}
                  onPick={setView}
                />
              </Show>
              <Showing />
            </div>
          </div>
          <Switch fallback={<EmptyState missing="city_no_buildings" action={<AskMayor />} />}>
            <Match when={prefs().showing === "results"}>
              <Results />
            </Match>
            <Match when={unavailable()}>
              {(lost) => <Unanswered query={lost().query} asked={QUERIES.city} />}
            </Match>
            <Match when={city() === undefined}>
              <p class="text-text-faint">{WAITING}</p>
            </Match>
            <Match when={hasBuildings()}>
              {(held) => (
                <Show
                  when={view() === "table"}
                  fallback={
                    <>
                      <Skyline city={held()} picked={picked()} onPick={setPicked} />
                      <Legend {...legendOf(lang())} />
                    </>
                  }
                >
                  <Table
                    city={held()}
                    runs={board().runs}
                    now={board().now}
                    picked={picked()}
                    onPick={setPicked}
                  />
                </Show>
              )}
            </Match>
          </Switch>
        </section>
        <Show when={pickedCity()}>
          {(held) => (
            <aside
              class="col-[9/12] flex min-w-0 flex-col narrow:col-span-full"
              aria-label={say(lang(), "city_panel")}
            >
              <Panel addr={held().addr} city={held().city} onClose={() => setPicked(null)} />
            </aside>
          )}
        </Show>
        <Show when={prefs().showing !== "results" && board().runs.length > 0}>
          <div class="col-[1/12] min-w-0 narrow:col-span-full">
            <Board runs={board().runs} now={board().now} level={2} />
          </div>
        </Show>
      </div>
    </Page>
  );
}
