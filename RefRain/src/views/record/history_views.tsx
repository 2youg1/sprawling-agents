// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { For, Match, Show, Switch, createMemo, createSignal } from "solid-js";
import { readAnswer } from "../../core/answered";
import { QUERIES } from "../../core/asking";
import { restoreDiscard } from "../../core/commands";
import { say } from "../../core/lang";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import type { Query } from "../../wire";
import Button from "../parts/button.solid";
import EmptyState from "../parts/empty.solid";
import Field from "../parts/field.solid";
import Path from "../parts/path.solid";
import Unanswered from "../parts/unanswered.solid";
import ArchiveLook from "./archive.look.solid";
import { lookOf as archiveOf } from "./archive";
import { lookOf as discardOf, type Standing } from "./bin";
import { lookOf as entryOf, type EntryProps } from "./entry";

export function Archive() {
  const u = ui();
  const lang = storeValue(u.lang);
  const [needle, setNeedle] = createSignal("");
  const question = createMemo<Query>(() => ({ archive_search: { needle: needle() } }));
  const search = storeValue(() => u.conn.asking.ask(question()));
  const read = createMemo(() =>
    readAnswer(search(), (held) => ("archive" in held ? held.archive.hits : undefined)),
  );
  const refusal = createMemo(() => {
    const item = read();
    return item.kind === "unavailable" ? item.query : undefined;
  });
  const hits = createMemo(() => {
    const item = read();
    return item.kind === "held" ? item.value : undefined;
  });
  return (
    <div class="flex flex-col gap-base">
      <div class="max-w-talk">
        <Field
          label={say(lang(), "rec_search")}
          labelling="hidden"
          placeholder={say(lang(), "rec_search")}
          value={needle()}
          onInput={setNeedle}
        />
      </div>
      <Switch fallback={<p class="text-text-faint">…</p>}>
        <Match when={refusal()}>
          {(query) => <Unanswered query={query()} asked={question()} />}
        </Match>
        <Match when={hits()?.length === 0}>
          <EmptyState missing="rec_archive_nothing" seat="region" />
        </Match>
        <Match when={hits()}>{(items) => <ArchiveLook {...archiveOf(items())} />}</Match>
      </Switch>
    </div>
  );
}

const INK: Record<Standing, string> = { restored: "text-text-faint", gone: "text-alert" };

export function Bin() {
  const u = ui();
  const lang = storeValue(u.lang);
  const held = storeValue(u.conn.asking.ask(QUERIES.discards));
  const look = createMemo(() => {
    const answer = held();
    return answer === undefined || !("discards" in answer)
      ? undefined
      : discardOf(answer.discards.rows, lang(), (restoration) => {
          u.send(restoreDiscard(restoration));
        });
  });
  return (
    <Switch fallback={<p class="text-text-faint">…</p>}>
      <Match when={look()?.rows.length === 0}>
        <EmptyState missing="bin_nothing" seat="region" />
      </Match>
      <Match when={look()}>
        {(view) => (
          <ul class="text-note">
            <For each={view().rows} keyed={(row) => row.key}>
              {(row) => (
                <li class="flex flex-col gap-tight border-b border-edge py-snug">
                  <div class="flex min-h-control items-center gap-base">
                    <span class="min-w-0 flex-1">
                      <Path path={row().path} />
                    </span>
                    <span class="figure text-text-faint">{row().when}</span>
                    <span class={INK[row().standing]}>{row().standingWord}</span>
                    <Show when={row().restore}>
                      {(restore) => (
                        <Button label={restore().label} tone="quiet" onPress={restore().onPress} />
                      )}
                    </Show>
                  </div>
                  <div class="summary truncate font-mono text-text-faint">{row().way}</div>
                </li>
              )}
            </For>
          </ul>
        )}
      </Match>
    </Switch>
  );
}

const ROW =
  "grid grid-cols-[14ch_6ch_minmax(0,1fr)] items-baseline gap-x-base py-tight text-note @min-[48rem]:grid-cols-[14ch_6ch_minmax(0,1fr)_minmax(0,24ch)] @max-[30rem]:grid-cols-[14ch_minmax(0,1fr)]";
const SEQ = "figure text-right text-text-faint @max-[30rem]:text-left";
const WHAT = "min-w-0 truncate @max-[30rem]:col-span-full @max-[30rem]:whitespace-normal";
const WHERE = "hidden min-w-0 truncate text-right @min-[48rem]:block";

export function RecordEntry(props: EntryProps) {
  const lang = storeValue(ui().lang);
  const look = createMemo(() => entryOf(props, lang()));
  const record = createMemo(() => {
    const row = look().row;
    return row.kind === "record" ? row : undefined;
  });
  const log = createMemo(() => {
    const row = look().row;
    return row.kind === "log" ? row : undefined;
  });
  return (
    <>
      <Show when={look().day}>
        {(day) => (
          <li class="flex justify-between border-b border-edge pt-base pb-tight text-note text-text-faint">
            <span class="figure">{day().day}</span>
            <span>{day().zone}</span>
          </li>
        )}
      </Show>
      <Show when={record()}>
        {(row) => (
          <li>
            <button
              type="button"
              data-control="surface"
              aria-expanded={row().open ? "true" : "false"}
              onClick={() => row().wire.onclick()}
              class={[
                ROW,
                "w-full rounded-card px-snug text-left transition-colors still:transition-none",
                row().open ? "wash ease-arrive" : "ease-leave hover:wash hover:ease-arrive",
              ]}
            >
              <time class="figure text-text-faint" datetime={row().when.at}>
                {row().when.text}
              </time>
              <span class={SEQ}>{row().seq}</span>
              <span class={WHAT}>
                <span class="text-text">{row().what}</span>
                <span class="text-text-quiet">{row().facts}</span>
              </span>
              <span class={`${WHERE} font-mono text-text-quiet`}>{row().where}</span>
            </button>
            <Show when={row().raw !== undefined}>
              <div class="flex flex-col gap-tight pt-tight pb-base pl-[calc(20ch+2*var(--spacing-base))] @max-[30rem]:pl-0">
                <span class="text-note text-text-faint">{row().rawLabel}</span>
                <pre class="max-h-output overflow-auto rounded-card border border-edge p-base text-note text-text-quiet">
                  {row().raw}
                </pre>
              </div>
            </Show>
          </li>
        )}
      </Show>
      <Show when={log()}>
        {(row) => (
          <li class={`${ROW} px-snug font-mono`}>
            <Show when={row().when} fallback={<span class="text-text-faint" />}>
              {(time) => (
                <time class="figure text-text-faint" datetime={time().at}>
                  {time().text}
                </time>
              )}
            </Show>
            <span class={SEQ}>{row().seq}</span>
            <span class={WHAT}>
              <span class="text-text-faint">{row().level}</span>
              <span class="text-text-quiet">{row().line}</span>
            </span>
            <span class={`${WHERE} text-text-faint`}>{row().module}</span>
          </li>
        )}
      </Show>
    </>
  );
}
