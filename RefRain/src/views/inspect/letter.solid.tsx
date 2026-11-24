// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo, createSignal, Match, onSettled, Show, Switch, untrack } from "solid-js";
import { readAnswer } from "../../core/answered";
import { adopted } from "../../core/belief/adopted";
import { fill, say } from "../../core/lang";
import { buildingOf, toFragment } from "../../core/route";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import type { Address, B3Hash } from "../../wire";
import Card from "../editor/proposals_card_view";
import RefRain from "../editor/document_view";
import Segmented from "../parts/segmented.solid";
import Thread from "../talk/thread";
import TextKey from "./text_key.look.solid";

type Reading = "diff" | "talk" | "before" | "now";
const READINGS: readonly Reading[] = ["diff", "talk", "before", "now"];
const WAITING = "…";

export interface LetterProps {
  readonly doc: Address;
  readonly card: B3Hash;
  readonly opening?: Reading | undefined;
}

export default function Letter(props: LetterProps) {
  const u = ui();
  const lang = storeValue(u.lang);
  const belief = storeValue(u.conn.belief);
  const [reading, setReading] = createSignal<Reading>(untrack(() => props.opening ?? "diff"));
  let heading: HTMLHeadingElement | undefined;
  onSettled(() => heading?.focus());

  const asked = storeValue(() => u.conn.asking.ask({ proposals: props.doc }));
  const read = createMemo(() =>
    readAnswer(asked(), (answer) => ("proposals" in answer ? answer.proposals : undefined)),
  );
  const review = createMemo(() => {
    const answer = read();
    if (answer.kind !== "held") return null;
    return answer.value.open.find((card) => card.id === props.card) ?? null;
  });
  const version = () => {
    const answer = read();
    return answer.kind === "held" ? (answer.value.version ?? null) : null;
  };
  const building = () => buildingOf(props.doc);
  const path = () => props.doc.slice(building().length + 1);
  const sender = storeValue(() => {
    const held = review();
    return held === null ? undefined : u.conn.asking.ask({ run_view: { run: held.run } });
  });
  const sent = createMemo(() => {
    const held = review();
    if (held === null) return undefined;
    const answer = sender();
    const mine = belief().runs[held.run];
    return answer !== undefined && "run" in answer && answer.run !== null
      ? adopted(answer.run, mine)
      : mine;
  });

  return (
    <div class="flex min-h-0 flex-1 flex-col bg-page">
      <div class="flex flex-wrap items-center gap-x-base gap-y-tight border-b border-edge px-wide py-snug">
        <h2
          ref={(node) => {
            heading = node;
          }}
          tabindex={-1}
          class="min-w-0 flex-1 font-mono text-note break-all text-text-quiet"
        >
          {props.doc}
        </h2>
        <Show when={review()}>
          {(held) => (
            <TextKey
              label={fill(say(lang(), "letter_sender"), {
                who: belief().runs[held().run]?.addr ?? held().run.slice(0, 8),
              })}
              wire={{ href: toFragment({ kind: "run", run: held().run }) }}
            />
          )}
        </Show>
        <Segmented
          label={say(lang(), "letter_reading")}
          options={READINGS.map((value) => ({ value, label: say(lang(), `letter_${value}`) }))}
          held={reading()}
          onPick={setReading}
        />
      </div>
      <Switch>
        <Match when={read().kind !== "held"}>
          <p class="px-wide py-snug text-note text-text-faint">{WAITING}</p>
        </Match>
        <Match when={review() === null}>
          <p class="px-wide py-snug text-note text-text-faint">{say(lang(), "letter_gone")}</p>
        </Match>
        <Match when={review()}>
          {(held) => (
            <Switch>
              <Match when={reading() === "diff"}>
                <div class="min-h-0 flex-1 overflow-auto px-wide py-snug">
                  <Card doc={props.doc} card={held()} version={version()} />
                </div>
              </Match>
              <Match when={reading() === "talk"}>
                <div class="min-h-0 flex-1 overflow-auto px-wide py-snug">
                  <Show when={sent()} fallback={<p class="text-note text-text-faint">{WAITING}</p>}>
                    {(run) => <Thread run={run()} around={held().offered ?? null} />}
                  </Show>
                </div>
              </Match>
              <Match when={reading() === "before" || reading() === "now"}>
                <div class="min-h-0 flex-1 overflow-auto">
                  <Show when={reading()} keyed>
                    {(lens) => (
                      <RefRain
                        building={building()}
                        path={path()}
                        version={lens === "before" ? held().baseline : null}
                      />
                    )}
                  </Show>
                </div>
              </Match>
            </Switch>
          )}
        </Match>
      </Switch>
    </div>
  );
}
