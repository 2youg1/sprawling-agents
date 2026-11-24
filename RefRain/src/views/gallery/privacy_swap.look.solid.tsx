// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { For, Show } from "solid-js";
import "./privacy_swap.look.solid.css";
import Button from "../parts/button.solid";
import type { EntryLook } from "../setup/privacy/entry";

export default function Swap(look: EntryLook) {
  const titleId = () => `${look.id}-swap-title`;
  return (
    <article
      id={`${look.id}-swap`}
      tabindex="-1"
      aria-labelledby={titleId()}
      class="privacy-swap swap"
    >
      <header class="swap-head">
        <h4 id={titleId()} class="swap-title">
          {look.title}
        </h4>
        <div class="swap-presses">
          <For each={look.presses} keyed={(entry) => entry.label}>
            {(press) => <Button label={press().label} tone="primary" onPress={press().onPress} />}
          </For>
        </div>
      </header>
      <p class="swap-quiet swap-break">
        {look.lines.label}: {look.lines.text}
      </p>
      <p class="swap-strong">
        {look.overlooked.label}: {look.overlooked.text}
      </p>
      <ul class="swap-facts">
        <For each={look.facts} keyed={(entry) => entry.label}>
          {(fact) => (
            <li>
              <span class="swap-quiet">{fact().label}</span>{" "}
              <span class="swap-break">{fact().value}</span>
            </li>
          )}
        </For>
      </ul>
      <p class={look.fitWeight === "alert" ? "swap-alert" : "swap-quiet"}>{look.fit}</p>
      <Show when={look.effect !== null}>
        <p class="swap-alert">{look.effect}</p>
      </Show>
      <details class="swap-more">
        <summary>{look.editions.label}</summary>
        <For each={look.notes} keyed={(entry) => entry.label}>
          {(note) => (
            <p>
              <span class="swap-quiet">{note().label}</span> {note().text}
            </p>
          )}
        </For>
        <p>{look.editions.text}</p>
        <p class="swap-quiet">{look.editionsNote}</p>
        <p class="swap-break">
          {look.target.label}: {look.target.text}
        </p>
      </details>
      <p role="status" class={look.status?.weight === "alert" ? "swap-alert" : "swap-quiet"}>
        {look.status?.text ?? ""}
      </p>
    </article>
  );
}
