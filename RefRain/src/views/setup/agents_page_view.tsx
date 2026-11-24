// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo, createSignal, createUniqueId, For, Show } from "solid-js";

import { fill, say, type Key } from "../../core/lang";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import type { AgentCatalogAnswer, AgentOffer, AxError } from "../../wire";
import Badge from "../parts/badge.solid";
import Glyph from "../parts/glyph.solid";
import { shownOf, snapshotDay, standingOf, signsInElsewhere, type BoxReading } from "./agents";
import Consent from "./consent_view";

export interface AgentsPageProps {
  readonly answer: AgentCatalogAnswer;
  readonly text: string;
  readonly reading: BoxReading;
  readonly parsed: AgentOffer | null;
  readonly room: string | null;
  readonly adding: ReadonlySet<string>;
  readonly unknown?: ReadonlySet<string>;
  readonly errors?: ReadonlyMap<string, AxError>;
  readonly onText: (text: string) => void;
  readonly onPaste: () => void;
  readonly onAdd: (offer: AgentOffer, here: boolean) => void;
  readonly onCheck?: (() => void) | undefined;
}

export default function AgentsPage(props: AgentsPageProps) {
  const lang = storeValue(ui().lang);
  const word = (key: Key) => say(lang(), key);
  const uid = createUniqueId();
  const [opened, setOpened] = createSignal<string>();
  const [summary, setSummary] = createSignal(false);
  const shown = createMemo(() =>
    shownOf(props.answer, props.reading.kind === "search" ? props.reading.typed : ""),
  );
  function Card(entry: { readonly offer: AgentOffer; readonly named: boolean }) {
    return (
      <Consent
        offer={entry.offer}
        named={entry.named}
        seats={props.room !== null}
        adding={props.adding.has(entry.offer.id)}
        unknown={props.unknown?.has(entry.offer.id) === true}
        error={props.errors?.get(entry.offer.id)}
        onAdd={props.onAdd}
        onCheck={props.onCheck}
      />
    );
  }
  return (
    <div class="flex min-w-0 flex-col gap-wide">
      <Show when={props.room} keyed>
        {(room) => <p class="text-note text-text-quiet">{fill(word("acp_room"), { room })}</p>}
      </Show>
      <Show when={props.answer.added.length > 0}>
        <section class="flex flex-col gap-tight">
          <button
            type="button"
            class="flex min-h-touch items-center gap-base rounded-control px-snug text-left text-label text-text"
            data-control="surface"
            aria-expanded={summary() ? "true" : "false"}
            aria-controls={`${uid}-added`}
            onClick={() => setSummary((open) => !open)}
          >
            <span class="min-w-0 flex-1 truncate">
              {fill(word("acp_added_summary"), {
                names: props.answer.added.map((line) => line.name).join(", "),
              })}
            </span>
            <Glyph
              name="chevron"
              size="sm"
              class={["shrink-0 text-text-faint", summary() && "rotate-90"]}
            />
          </button>
          <ul
            id={`${uid}-added`}
            class="flex flex-col border-t border-edge"
            hidden={!summary()}
            inert={!summary()}
          >
            <For each={props.answer.added} keyed={(line) => line.id}>
              {(line) => {
                const standing = () => standingOf(line());
                return (
                  <>
                    <li class="flex min-h-touch flex-wrap items-center gap-base border-b border-edge py-tight pl-snug">
                      <span class="w-[9rem] shrink-0 text-label font-label text-text">
                        {line().name}
                      </span>
                      <span class="flex flex-1 flex-wrap items-center gap-snug">
                        <Badge text={word(standing().key)} weight={standing().weight} dot />
                        <Show when={line().seated_in.length > 0}>
                          <span class="text-note text-text-quiet">
                            {fill(word("acp_seated_in"), { rooms: line().seated_in.join(", ") })}
                          </span>
                        </Show>
                      </span>
                    </li>
                    <Show when={signsInElsewhere(line())}>
                      <li class="px-snug py-tight text-note text-text-quiet">
                        {word("acp_subscription_elsewhere")}
                      </li>
                    </Show>
                  </>
                );
              }}
            </For>
          </ul>
        </section>
      </Show>
      <div class="flex min-w-0 flex-col gap-tight">
        <label for={`${uid}-box`} class="text-note text-text-quiet">
          {word("acp_search")}
        </label>
        <div class="box-content flex h-touch min-w-0 items-center gap-snug rounded-control border border-edge-input bg-raised pl-base">
          <Glyph name="search" size="sm" class="shrink-0 text-text-faint" />
          <input
            id={`${uid}-box`}
            value={props.text}
            spellcheck="false"
            autocomplete="off"
            class={[
              "h-full min-w-0 flex-1 bg-transparent text-body text-text",
              props.reading.kind === "paste" && "font-mono",
            ]}
            onInput={(event) => props.onText(event.currentTarget.value)}
          />
          <button
            type="button"
            data-control="surface"
            class="h-full shrink-0 rounded-r-control border-l border-edge-input px-pane text-label text-text"
            onClick={() => (props.text === "" ? props.onPaste() : props.onText(""))}
          >
            {word(props.text === "" ? "acp_paste" : "acp_clear")}
          </button>
        </div>
      </div>

      <Show
        when={props.reading.kind === "paste"}
        fallback={
          <>
            <section class="flex flex-col gap-snug">
              <h3 class="text-note text-text-quiet">{word("acp_detected")}</h3>
              <Show
                when={shown().detected.length > 0}
                fallback={<p class="text-note text-text-faint">{word("acp_detected_none")}</p>}
              >
                <div class="grid grid-fit gap-base">
                  <For each={shown().detected} keyed={(offer) => offer.id}>
                    {(offer) => <Card offer={offer()} named />}
                  </For>
                </div>
              </Show>
            </section>
            <section class="flex flex-col gap-tight">
              <h3 class="text-note text-text-quiet">
                {word("acp_catalog")} ·{" "}
                {fill(word("acp_catalog_snapshot"), {
                  date: snapshotDay(lang(), props.answer.snapshot.date),
                })}
              </h3>
              <ul class="flex flex-col border-t border-edge">
                <For
                  each={shown().catalog}
                  keyed={(offer) => offer.id}
                  fallback={
                    <li class="py-base text-note text-text-faint">{word("acp_catalog_none")}</li>
                  }
                >
                  {(offer) => {
                    const open = () => opened() === offer().id;
                    const id = createUniqueId();
                    return (
                      <li class={["flex flex-col border-b border-edge", open() && "pb-snug"]}>
                        <button
                          type="button"
                          data-control="surface"
                          aria-expanded={open() ? "true" : "false"}
                          aria-controls={id}
                          class={[
                            "flex min-h-touch items-center gap-base px-snug text-left",
                            open() && "wash-strong",
                          ]}
                          onClick={() => setOpened(open() ? undefined : offer().id)}
                        >
                          <span class="flex-1 text-label font-label text-text">{offer().name}</span>
                          <Show when={!open()}>
                            <span class="text-note text-text-faint">
                              {[offer().version, offer().licence]
                                .filter((each) => each != null)
                                .join(" · ")}
                            </span>
                          </Show>
                          <Glyph
                            name="chevron"
                            size="sm"
                            class={["shrink-0 text-text-faint", open() && "rotate-90"]}
                          />
                        </button>
                        <div id={id} hidden={!open()} inert={!open()}>
                          <Show when={open()}>
                            <div class="pt-snug">
                              <Card offer={offer()} named={false} />
                            </div>
                          </Show>
                        </div>
                      </li>
                    );
                  }}
                </For>
              </ul>
            </section>
          </>
        }
      >
        <section class="flex flex-col gap-snug">
          <h3 class="text-note text-text-quiet">{word("acp_pasted")}</h3>
          <Show when={props.parsed}>{(offer) => <Card offer={offer()} named />}</Show>
        </section>
      </Show>
    </div>
  );
}
