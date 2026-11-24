// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo, Match, Show, Switch } from "solid-js";

import { readAnswer } from "../../core/answered";
import { QUERIES } from "../../core/asking";
import { fill, say } from "../../core/lang";
import { storeValue } from "../../core/solid_store";
import { kilo, usd } from "../../core/time";
import { ui } from "../../ui";
import type { Address, RoundsAnswer, RunId } from "../../wire";
import { costReading, runSpend } from "../pricing";
import Bounds from "../talk/bounds_view";
import { frozenSaid } from "../talk/frozen";
import { contextOf, lockOfConfig, remindersOf, remindersSaid, usedPercent } from "../talk/gauge";
import { speedOf } from "./speed";

export interface SheetProps {
  readonly here: Address;
  readonly run: RunId | null;
  readonly rounds: RoundsAnswer | undefined;
}

const DASH = "—";

export function SessionSheet(props: SheetProps) {
  const u = ui();
  const lang = storeValue(u.lang);
  const endpoints = storeValue(u.conn.asking.ask(QUERIES.endpoints));
  const cost = storeValue(u.conn.asking.ask(QUERIES.cost));
  const config = storeValue(() => u.conn.asking.ask({ config: { addr: props.here } }));
  const served = createMemo(() => {
    const answer = endpoints();
    return answer !== undefined && "endpoints" in answer ? answer.endpoints.endpoints : [];
  });
  const turns = createMemo(() => props.rounds?.turns ?? []);
  const context = createMemo(() =>
    contextOf(turns(), served(), remindersOf(config()), lockOfConfig(config())),
  );
  const policy = () => props.rounds?.opening?.policy ?? null;
  const model = createMemo(
    () =>
      turns()
        .filter((turn) => typeof turn.model === "string")
        .at(-1)?.model ?? null,
  );
  const price = createMemo(() => {
    const facts = served()
      .flatMap((endpoint) => endpoint.models)
      .find((row) => row.id === model());
    return typeof facts?.input_price === "string" && typeof facts.output_price === "string"
      ? fill(say(lang(), "world_price"), { input: facts.input_price, output: facts.output_price })
      : "";
  });
  const frozen = createMemo(() => frozenSaid(props.rounds?.opening, lang()));
  const used = createMemo(() =>
    turns().reduce(
      (sum, turn) => ({
        input: sum.input + (turn.used?.input ?? 0),
        output: sum.output + (turn.used?.output ?? 0),
        cached: sum.cached + (turn.used?.cached ?? 0),
        hitInput: sum.hitInput + (turn.used?.cached == null ? 0 : turn.used.input),
        toldCached: sum.toldCached || turn.used?.cached != null,
        written: sum.written + (turn.used?.cache_write ?? 0),
        told: sum.told || turn.used != null,
        toldWritten: sum.toldWritten || turn.used?.cache_write != null,
      }),
      {
        input: 0,
        output: 0,
        cached: 0,
        hitInput: 0,
        toldCached: false,
        written: 0,
        told: false,
        toldWritten: false,
      },
    ),
  );
  const read = createMemo(() =>
    readAnswer(cost(), (held) => ("cost" in held ? held.cost : undefined)),
  );
  const spent = createMemo(() => {
    const spend = runSpend(read(), props.run);
    switch (spend.kind) {
      case "none":
        return DASH;
      case "unreadable":
        return say(lang(), "world_unreadable");
      case "unpriced":
        return say(lang(), "cost_none");
      case "spent":
        return usd(spend.micros);
    }
  });
  const city = createMemo(() => {
    const answer = read();
    return answer.kind === "held" && costReading(answer.value).kind === "priced"
      ? fill(say(lang(), "world_cost_city"), { usd: usd(answer.value.total) })
      : "";
  });
  const speed = createMemo(() => speedOf(turns()));
  const throughput = () => speed()?.tps;
  const cacheNote = () =>
    [
      fill(say(lang(), "world_cache_read"), { n: kilo(used().cached) }),
      used().toldWritten ? fill(say(lang(), "world_cache_write"), { n: kilo(used().written) }) : "",
    ]
      .filter((part) => part !== "")
      .join(" · ");

  return (
    <div class="@container shrink-0">
      <dl class="grid grid-flow-row-dense grid-cols-2 gap-x-gutter gap-y-pane border-y border-edge py-pane @min-cells:grid-cols-3">
        <div class="flex min-w-0 flex-col">
          <dt class="text-note text-text-faint">{say(lang(), "world_model")}</dt>
          <dd class="truncate text-text">{model() ?? DASH}</dd>
          <Show when={frozen()}>
            {(note) => <dd class="truncate text-note text-text-faint">{note()}</dd>}
          </Show>
        </div>
        <div class="col-span-2 flex min-w-0 flex-col">
          <dt class="text-note text-text-faint">{say(lang(), "ring_name")}</dt>
          <Show when={context()} fallback={<dd class="text-text-faint">{DASH}</dd>}>
            {(window) => (
              <>
                <dd class="figure truncate text-text">
                  {fill(say(lang(), "ring_used"), {
                    used: kilo(window().used),
                    window: kilo(window().window),
                  })}
                  <span class="text-text-faint">
                    · {fill(say(lang(), "ring_share"), { n: String(usedPercent(window())) })}
                  </span>
                </dd>
                <dd class="truncate text-note text-text-faint">
                  {remindersSaid(lang(), window()).join(" · ")}
                </dd>
              </>
            )}
          </Show>
        </div>
        <div class="flex min-w-0 flex-col">
          <dt class="text-note text-text-faint">{say(lang(), "world_tokens")}</dt>
          <dd class="figure truncate text-text">
            {used().told
              ? fill(say(lang(), "world_tokens_io"), {
                  input: kilo(used().input),
                  output: kilo(used().output),
                })
              : DASH}
          </dd>
          <Show when={price()}>
            {(note) => <dd class="truncate text-note text-text-faint">{note()}</dd>}
          </Show>
        </div>
        <div class="flex min-w-0 flex-col">
          <dt class="text-note text-text-faint">{say(lang(), "world_cache")}</dt>
          <Switch fallback={<dd class="text-text-faint">{DASH}</dd>}>
            <Match when={used().input > 0 && !used().toldCached}>
              <dd class="truncate text-text-faint">{say(lang(), "world_cache_unknown")}</dd>
            </Match>
            <Match when={used().hitInput > 0}>
              <dd class="figure truncate text-text">
                {fill(say(lang(), "world_cache_hit"), {
                  n: String(Math.round((used().cached / used().hitInput) * 100)),
                })}
              </dd>
              <dd class="truncate text-note text-text-faint">{cacheNote()}</dd>
            </Match>
          </Switch>
        </div>
        <div class="flex min-w-0 flex-col">
          <dt class="text-note text-text-faint">{say(lang(), "world_cost")}</dt>
          <dd class="figure truncate text-text">{spent()}</dd>
          <Show when={city()}>
            {(note) => <dd class="truncate text-note text-text-faint">{note()}</dd>}
          </Show>
        </div>
        <div class="flex min-w-0 flex-col">
          <dt class="text-note text-text-faint">{say(lang(), "world_ttft_name")}</dt>
          <Show when={speed()} fallback={<dd class="text-text-faint">{DASH}</dd>}>
            {(metrics) => (
              <>
                <dd class="figure truncate text-text">
                  {say(lang(), "talk_ttft")}{" "}
                  {fill(say(lang(), "talk_took_ms"), { n: String(metrics().ttft) })}
                </dd>
                <dd class="truncate text-note text-text-faint">
                  {fill(say(lang(), "world_ttft_note"), {
                    mean: String(metrics().ttftMean),
                    n: String(metrics().turns),
                  })}
                </dd>
              </>
            )}
          </Show>
        </div>
        <div class="flex min-w-0 flex-col">
          <dt class="text-note text-text-faint">{say(lang(), "world_tps_name")}</dt>
          <Show when={throughput()} fallback={<dd class="text-text-faint">{DASH}</dd>}>
            {(metrics) => (
              <>
                <dd class="figure truncate text-text">
                  {fill(say(lang(), "world_tps"), { n: String(Math.round(metrics().p50)) })}
                </dd>
                <dd class="truncate text-note text-text-faint">
                  {fill(say(lang(), "world_tps_note"), {
                    p1: String(Math.round(metrics().p1)),
                    n: String(metrics().turns),
                  })}
                </dd>
              </>
            )}
          </Show>
        </div>
        <div class="flex min-w-0 flex-col">
          <dt class="text-note text-text-faint">{say(lang(), "world_bounds")}</dt>
          <dd class="figure truncate text-text">
            <Show when={policy()} fallback={DASH}>
              {(limits) => fill(say(lang(), "world_write"), { limit: limits().write })}
            </Show>
          </dd>
          <dd class="-ml-snug flex min-w-0 flex-wrap items-center">
            <Bounds room={props.here} />
          </dd>
        </div>
        <div class="flex min-w-0 flex-col">
          <dt class="text-note text-text-faint">{say(lang(), "world_admission")}</dt>
          <Show when={policy()} fallback={<dd class="text-text-faint">{DASH}</dd>}>
            {(limits) => (
              <>
                <dd class="figure truncate text-text">{limits().admit}</dd>
                <dd class="truncate text-note text-text-faint">
                  {fill(say(lang(), "world_landing"), { landing: limits().landing })}
                </dd>
              </>
            )}
          </Show>
        </div>
      </dl>
    </div>
  );
}
