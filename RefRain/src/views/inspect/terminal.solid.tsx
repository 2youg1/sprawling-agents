// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo, createSignal, For, Match, Show, Switch } from "solid-js";
import { readAnswer } from "../../core/answered";
import { fill, say, type Lang } from "../../core/lang";
import type { Tail } from "../../core/live_output";
import { storeValue } from "../../core/solid_store";
import { count, hhmmss, isoInstant, kib } from "../../core/time";
import { ui } from "../../ui";
import type { Call, Query } from "../../wire";
import { INK, rows } from "../monitor/ansi";
import Ceilings from "../monitor/ceilings.solid";
import type { Ending } from "../monitor/trace";
import Unanswered from "../parts/unanswered.solid";
import { printedOf } from "./terminal";
import TextKey from "./text_key.look.solid";

export interface InspectorTerminalProps {
  readonly call: Call;
  readonly tail: Tail;
}

const WAITING = "…";

function ended(lang: Lang, ending: Ending): { readonly word: string; readonly ink: string } {
  switch (ending.kind) {
    case "code":
      return {
        word: fill(say(lang, "mon_exited"), { code: String(ending.code) }),
        ink: ending.code === 0 ? "text-accent" : "text-alert",
      };
    case "stopped":
      return { word: fill(say(lang, "mon_stopped"), { why: ending.why }), ink: "text-alert" };
    case "running":
      return { word: say(lang, "mon_running"), ink: "text-accent" };
    case "unread":
      return { word: say(lang, "mon_unread"), ink: "" };
  }
}

export default function InspectorTerminal(props: InspectorTerminalProps) {
  const u = ui();
  const lang = storeValue(u.lang);
  const printed = createMemo(() => printedOf(props.call, props.tail));
  const drawn = createMemo(() => [
    ...(printed().out === "" ? [] : rows(printed().out).map((row) => ({ alert: false, row }))),
    ...(printed().err === "" ? [] : rows(printed().err).map((row) => ({ alert: true, row }))),
  ]);
  const [raw, setRaw] = createSignal(false);
  const question = createMemo<Query | null>(() => {
    const pinned = printed().pinned;
    return raw() && pinned !== null ? { content: { locator: pinned } } : null;
  });
  const content = storeValue(() => {
    const asked = question();
    return asked === null ? undefined : u.conn.asking.ask(asked);
  });
  const original = createMemo(() =>
    readAnswer(content(), (held) => ("content" in held ? held.content : undefined)),
  );
  const denied = () => {
    const held = original();
    return held.kind === "unavailable" ? held : undefined;
  };
  const result = () => {
    const held = original();
    return held.kind === "held" ? held.value : undefined;
  };
  const ending = () => {
    const current = printed().ending;
    return current === null ? null : ended(lang(), current);
  };

  return (
    <div class="flex min-h-0 flex-1 flex-col bg-chrome">
      <div class="grid h-[calc(8*var(--spacing-baseline))] shrink-0 grid-cols-[minmax(0,1fr)_auto] grid-rows-[calc(3*var(--spacing-baseline))_calc(2*var(--spacing-baseline))] content-center gap-x-pane border-b border-edge pr-snug pl-wide">
        <p class="truncate font-mono text-note text-text">
          <span class="mr-base font-label">{props.call.tool}</span>
          {printed().line}
        </p>
        <Show when={printed().pinned !== null}>
          <div class="row-span-2 self-center">
            <TextKey
              label={say(lang(), "inspect_raw")}
              wire={{
                type: "button",
                "aria-pressed": raw(),
                onclick: () => setRaw((held) => !held),
              }}
            />
          </div>
        </Show>
        <p class="col-start-1 flex gap-pane overflow-hidden font-mono text-note leading-[calc(2*var(--spacing-baseline))] whitespace-nowrap text-text-faint">
          <Show when={printed().took !== null}>
            <span class="figure">
              {fill(say(lang(), "inspect_took"), { ms: count(printed().took ?? 0) })}
            </span>
          </Show>
          <Show when={ending()}>{(end) => <span class={end().ink}>{end().word}</span>}</Show>
          <Show when={printed().finished !== null}>
            <time class="figure" datetime={isoInstant(printed().finished ?? 0)}>
              {hhmmss(printed().finished ?? 0)}
            </time>
          </Show>
          <Show when={printed().cut > 0}>
            <span>{fill(say(lang(), "inspect_cut"), { n: count(printed().cut) })}</span>
            <Show when={printed().pinned === null}>
              <span>{say(lang(), "inspect_no_original")}</span>
            </Show>
          </Show>
        </p>
      </div>
      <Show when={printed().ceilings.length > 0}>
        <div class="shrink-0 border-b border-edge px-wide py-snug font-mono text-note">
          <Ceilings notes={printed().ceilings} />
        </div>
      </Show>
      <div class="min-h-0 flex-1 overflow-auto">
        <Show
          when={question()}
          fallback={
            <div
              class="w-max min-w-full px-wide py-snug font-mono text-note leading-[calc(3*var(--spacing-baseline))] text-text-quiet"
              aria-live="off"
            >
              <For each={drawn()} keyed={false}>
                {(line) => (
                  <div class="min-h-[1lh] whitespace-pre">
                    <For each={line().row} keyed={false}>
                      {(piece) => (
                        <span
                          class={[
                            line().alert ? "text-alert" : INK[piece().tone],
                            piece().bold && "font-semibold",
                          ]}
                        >
                          {piece().text}
                        </span>
                      )}
                    </For>
                  </div>
                )}
              </For>
            </div>
          }
        >
          {(asked) => (
            <Switch fallback={<p class="px-wide py-snug text-note text-text-faint">{WAITING}</p>}>
              <Match when={result()}>
                {(held) => (
                  <Show
                    when={!held().binary}
                    fallback={
                      <p class="px-wide py-snug text-note text-text-faint">
                        {fill(say(lang(), "inspect_not_text"), { bytes: kib(held().bytes) })}
                      </p>
                    }
                  >
                    <Show when={held().truncated}>
                      <p class="px-wide pt-snug text-note text-text-faint">
                        {fill(say(lang(), "inspect_raw_head"), { bytes: kib(held().bytes) })}
                      </p>
                    </Show>
                    <pre class="w-max min-w-full px-wide py-snug font-mono text-note leading-[calc(3*var(--spacing-baseline))] text-text-quiet">
                      {held().text}
                    </pre>
                  </Show>
                )}
              </Match>
              <Match when={denied()}>
                {(lost) => (
                  <div class="px-wide py-snug">
                    <Unanswered query={lost().query} asked={asked()} />
                  </div>
                )}
              </Match>
            </Switch>
          )}
        </Show>
      </div>
    </div>
  );
}
