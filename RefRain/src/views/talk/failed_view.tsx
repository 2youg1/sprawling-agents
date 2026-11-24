// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo, For, Show } from "solid-js";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import type { AxError } from "../../wire";
import { DiagnosticView } from "../diagnostic_view";
import Button from "../parts/button.solid";
import { failedLook, type CardLook, type Way } from "./refused";

function draw(way: Way) {
  return way.kind === "link" ? (
    <a
      href={way.href}
      class="underline transition-colors ease-leave hover:text-text hover:ease-arrive"
    >
      {way.text}
    </a>
  ) : (
    <Button label={way.label} onPress={way.onPress} />
  );
}

export function FailureCard(props: CardLook) {
  return (
    <div
      class="rounded-card border border-alert/40 px-base py-snug text-note text-text-quiet"
      role={props.role}
    >
      <p>
        <span class="text-alert">{props.head}</span>
        {props.after === undefined ? "" : ` · ${props.after}`}
      </p>
      <For each={props.lines} keyed={false}>
        {(line) => (
          <p class={["mt-tight", line().ink === "faint" ? "text-text-faint" : "text-text-quiet"]}>
            {line().text}
          </p>
        )}
      </For>
      <Show when={props.fold}>
        {(fold) => (
          <details class="mt-tight min-w-0">
            <summary class="cursor-pointer text-text-faint transition-colors ease-leave hover:text-text hover:ease-arrive">
              {fold().summary}
            </summary>
            <p class="mt-tight font-mono wrap-anywhere">{fold().text}</p>
          </details>
        )}
      </Show>
      <Show when={props.ways.length > 0}>
        <div class="mt-snug flex flex-wrap items-center gap-base">
          <For each={props.ways} keyed={false}>
            {(way) => draw(way())}
          </For>
        </div>
      </Show>
    </div>
  );
}

export default function Failed(props: {
  readonly what: string;
  readonly error?: AxError | undefined;
  readonly settings?: "offered" | "absent" | undefined;
  readonly onRetry?: (() => void) | undefined;
}) {
  const lang = storeValue(ui().lang);
  const card = createMemo(() =>
    failedLook(lang(), props.what, props.error, {
      settings: props.settings ?? "decided",
      onRetry: props.onRetry,
    }),
  );
  return (
    <div class="my-pane">
      <FailureCard {...card()} />
      <Show when={props.error}>{(error) => <DiagnosticView error={error()} lang={lang()} />}</Show>
    </div>
  );
}
