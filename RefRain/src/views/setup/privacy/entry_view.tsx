// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { JSX } from "@solidjs/web";
import { createMemo, createUniqueId, For, Show } from "solid-js";

import { storeValue } from "../../../core/solid_store";
import { ui } from "../../../ui";
import Button from "../../parts/button.solid";
import type { Action, EntryLook, Fact } from "./entry";
import { lookOf, type EntryModel } from "./page";
import type { Held } from "./state";

export function Facts(props: { readonly values: readonly Fact[] }) {
  return (
    <dl class="grid grid-cols-[auto_minmax(0,1fr)] gap-x-base gap-y-tight text-note">
      <For each={props.values} keyed={(fact) => fact.label}>
        {(fact) => (
          <>
            <dt class="text-text-quiet">{fact().label}</dt>
            <dd
              class={["min-w-0 text-text [overflow-wrap:anywhere]", fact().figure && "font-mono"]}
            >
              {fact().value}
            </dd>
          </>
        )}
      </For>
    </dl>
  );
}

export function EntryCard(look: EntryLook) {
  const title = () => `${look.id}-title`;
  return (
    <article
      id={look.id}
      tabindex="-1"
      aria-labelledby={title()}
      class="flex min-w-0 scroll-mt-wide flex-col gap-snug rounded-card bg-raised px-base py-snug"
    >
      <h4 id={title()} class="text-label font-label text-text">
        {look.title}
      </h4>
      <p class="text-note text-text-faint [overflow-wrap:anywhere]">
        <span class="text-text-quiet">{look.lines.label}</span> {look.lines.text}
      </p>
      <dl class="flex flex-col gap-tight text-note">
        <For each={look.notes} keyed={(note) => note.label}>
          {(note) => (
            <div class="flex flex-col">
              <dt class="text-text-quiet">{note().label}</dt>
              <dd class="text-text">{note().text}</dd>
            </div>
          )}
        </For>
        <div class="flex flex-col rounded-control border border-edge px-snug py-tight">
          <dt class="text-text-quiet">{look.overlooked.label}</dt>
          <dd class="text-text">{look.overlooked.text}</dd>
        </div>
        <div class="flex flex-col">
          <dt class="text-text-quiet">{look.editions.label}</dt>
          <dd class="text-text">{look.editions.text}</dd>
          <dd class="text-text-faint">{look.editionsNote}</dd>
          <dd class={look.fitWeight === "alert" ? "text-alert" : "text-text-quiet"}>{look.fit}</dd>
          <Show when={look.effect !== null}>
            <dd class="text-alert">{look.effect}</dd>
          </Show>
        </div>
      </dl>
      <Facts values={look.facts} />
      <details class="text-note">
        <summary data-control="surface" class="cursor-pointer text-text-quiet">
          {look.target.label}
        </summary>
        <code class="block font-mono text-text [overflow-wrap:anywhere]">{look.target.text}</code>
      </details>
      <Show when={look.presses.length > 0}>
        <div class="flex flex-wrap items-center gap-snug">
          <For each={look.presses} keyed={(press) => press.label}>
            {(press) => <Button label={press().label} tone="secondary" onPress={press().onPress} />}
          </For>
        </div>
      </Show>
      <p
        role="status"
        class={["text-note", look.status?.weight === "alert" ? "text-alert" : "text-text-quiet"]}
      >
        {look.status?.text ?? ""}
      </p>
    </article>
  );
}

export interface EntryProps {
  readonly model: EntryModel;
  readonly held: Held;
  readonly onPress: (action: Action) => void;
  readonly look?: (look: EntryLook) => JSX.Element;
  readonly root?: string;
}

export default function Entry(props: EntryProps) {
  const uid = createUniqueId();
  const lang = storeValue(ui().lang);
  const drawn = createMemo(() =>
    lookOf(lang(), props.model, {
      root: props.root ?? uid,
      stage: props.held.stage,
      shown: props.held.shown,
      press: props.onPress,
    }),
  );
  const Look = props.look ?? EntryCard;
  return <Look {...drawn()} />;
}
