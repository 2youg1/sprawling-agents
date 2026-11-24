// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo, createSignal, For, Show } from "solid-js";

import { QUERIES } from "../core/asking";
import { say } from "../core/lang";
import { storeValue } from "../core/solid_store";
import { ui } from "../ui";
import type { ReleaseAnswer } from "../wire";
import Button from "./parts/button.solid";
import Copy from "./parts/copy.solid";
import { lookOf, type Ink } from "./release/answer";
import "./release_view.css";

const RELEASES = "https://github.com/2youg1/sprawling-agents/releases";
const INK: Record<Ink, string> = {
  text: "text-text",
  quiet: "text-text-quiet",
  accent: "text-accent",
  alert: "text-alert",
};

export function ReleaseDetails(props: { readonly answer: ReleaseAnswer }) {
  const lang = storeValue(ui().lang);
  const [selection, setSelection] = createSignal<{
    readonly answer: ReleaseAnswer;
    readonly command: string | null;
  }>();
  const look = createMemo(() => {
    const chosen = selection();
    const answer = props.answer;
    return lookOf(answer, lang(), chosen?.answer === answer ? chosen.command : null, {
      choose: (command) => setSelection({ answer, command }),
    });
  });
  return (
    <div class="flex min-w-0 flex-col gap-snug">
      <Show when={look().refused}>
        {(refused) => (
          <p class="text-note text-alert">
            {refused().text}{" "}
            <code class="font-mono text-note text-text-quiet">{refused().recovery}</code>
          </p>
        )}
      </Show>
      <For each={look().statements} keyed={(statement) => statement.text}>
        {(statement) => <p class={["text-note", INK[statement().ink]]}>{statement().text}</p>}
      </For>
      <Show when={look().registries.length > 0}>
        <dl class="release-registries grid gap-x-base gap-y-tight text-note">
          <For each={look().registries} keyed={(entry) => entry.name}>
            {(entry) => (
              <>
                <dt class="text-text-faint">{entry().name}</dt>
                <dd class="min-w-0 break-words">
                  <Show when={entry().newest}>
                    {(newest) => (
                      <>
                        <span class="font-mono text-text">{newest().version}</span>{" "}
                        <span class="text-text-faint">{newest().released}</span>
                      </>
                    )}
                  </Show>
                  <Show when={entry().reason}>
                    {(reason) => (
                      <>
                        <span class="text-text-quiet">{reason().text}</span>{" "}
                        <span class="font-mono text-text-faint">{reason().said}</span>
                      </>
                    )}
                  </Show>
                </dd>
              </>
            )}
          </For>
        </dl>
      </Show>
      <Show when={look().update}>
        {(update) => (
          <div class="flex min-w-0 flex-col gap-tight">
            <span class="text-note text-text-quiet">{update().channel}</span>
            <Show when={update().choice}>
              {(choice) => (
                <select
                  class="h-control w-fit max-w-full min-w-0 rounded-control bg-raised px-snug text-note text-text"
                  aria-label={choice().wire["aria-label"]}
                  value={choice().wire.value}
                  onChange={(event) => choice().wire.onchange(event)}
                >
                  <option value="">{choice().unchosen}</option>
                  <For each={choice().commands} keyed={true}>
                    {(command) => <option value={command}>{command}</option>}
                  </For>
                </select>
              )}
            </Show>
            <Show when={update().command}>
              {(command) => (
                <div class="flex min-w-0 items-start gap-tight">
                  <code class="min-w-0 flex-1 break-words rounded-control bg-chrome px-snug py-tight font-mono text-note text-text">
                    {command()}
                  </code>
                  <Copy text={command()} />
                </div>
              )}
            </Show>
          </div>
        )}
      </Show>
    </div>
  );
}

export default function Release() {
  const u = ui();
  const lang = storeValue(u.lang);
  const [asked, setAsked] = createSignal(false);
  const held = storeValue(() => (asked() ? u.conn.asking.ask(QUERIES.release) : undefined));
  const answer = () => {
    const received = held();
    return received !== undefined && "release" in received ? received.release : undefined;
  };

  return (
    <section class="flex min-w-0 flex-col gap-snug">
      <div class="flex flex-wrap items-center gap-snug">
        <Button
          label={say(lang(), "release_check_registries")}
          tone="secondary"
          glyph="search"
          loading={asked() && answer() === undefined}
          onPress={() => {
            setAsked(true);
            u.conn.asking.refresh(QUERIES.release);
          }}
        />
        <a class="text-note text-accent underline" href={RELEASES} target="_blank" rel="noreferrer">
          {RELEASES}
        </a>
      </div>
      <Show when={answer()}>{(current) => <ReleaseDetails answer={current()} />}</Show>
    </section>
  );
}
