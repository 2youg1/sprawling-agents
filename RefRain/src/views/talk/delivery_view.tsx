// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createEffect, createMemo, createSignal, Show, untrack } from "solid-js";
import { fill, say, type Key } from "../../core/lang";
import type { Landing } from "../../core/landing";
import { toFragment } from "../../core/route";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import type { Address } from "../../wire";
import type { Delivery } from "./delivery";

type Echo = Exclude<Delivery, { readonly kind: "none" }>;
const WORDS: Record<Echo["kind"], Key> = {
  held: "talk_delivery_held",
  pending: "talk_delivery_pending",
  accepted: "talk_delivery_accepted",
  unknown: "talk_delivery_unknown",
};

export function Delivered(props: { readonly delivery: Echo }) {
  const lang = storeValue(ui().lang);
  return (
    <div class="my-pane" data-local-feedback={props.delivery.kind}>
      <div class="flex flex-col items-end">
        <div class="max-w-[83%] rounded-panel bg-speech px-pane py-base text-body leading-read whitespace-pre-wrap text-text-quiet wrap-anywhere">
          {props.delivery.words}
        </div>
        <div
          class={[
            "mt-tight text-note",
            props.delivery.kind === "unknown" ? "text-alert" : "text-text-faint",
          ]}
          role="status"
        >
          {say(lang(), WORDS[props.delivery.kind])}
        </div>
      </div>
    </div>
  );
}

export function Landed(props: { readonly landing: Landing }) {
  const lang = storeValue(ui().lang);
  const address = createMemo(() =>
    props.landing.kind === "elsewhere" ? props.landing.addr : undefined,
  );
  return (
    <Show when={address()}>
      {(at) => (
        <div class="mb-snug">
          <div
            class="flex flex-wrap items-baseline gap-snug text-note text-text-faint"
            role="status"
          >
            <span>{say(lang(), "talk_landed")}</span>
            <a
              href={toFragment({ kind: "talk", address: at() })}
              class="font-mono text-text-quiet underline transition-colors ease-leave hover:text-text hover:ease-arrive"
            >
              {at()}
            </a>
          </div>
        </div>
      )}
    </Show>
  );
}

const RECEIPT_MS = 400;

export function Handed(props: { readonly room: Address | null; readonly sent: number }) {
  const lang = storeValue(ui().lang);
  const [handed, setHanded] = createSignal(false);
  let answered = untrack(() => props.sent);
  createEffect(
    () => props.sent,
    (sent) => {
      if (sent === answered) return;
      answered = sent;
      setHanded(true);
      const receipt = setTimeout(() => setHanded(false), RECEIPT_MS);
      return () => clearTimeout(receipt);
    },
  );
  return (
    <span role="status" class="sr-only">
      {handed() && props.room !== null
        ? fill(say(lang(), "talk_handed"), { room: props.room })
        : ""}
    </span>
  );
}
