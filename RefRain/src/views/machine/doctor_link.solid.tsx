// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo, createUniqueId, Show } from "solid-js";
import type { DoctorItem } from "../../wire";
import Tip from "../parts/control_tip";
import { lookOf } from "./doctor_link";

export default function DoctorLink(props: { readonly item: DoctorItem }) {
  const look = createMemo(() => lookOf(props.item));
  const hint = createUniqueId();
  return (
    <Show
      when={look().homepage}
      fallback={
        <span class="shrink-0 whitespace-nowrap font-mono text-label text-text">{look().name}</span>
      }
    >
      {(homepage) => (
        <span class="shrink-0 whitespace-nowrap">
          <Tip text={homepage()} id={hint}>
            <a
              class="font-mono text-label text-text underline decoration-edge underline-offset-2 transition-colors ease-leave hover:decoration-accent hover:ease-arrive"
              href={homepage()}
              target={look().target}
              rel="noreferrer"
              aria-describedby={hint}
            >
              {look().name}
            </a>
          </Tip>
        </span>
      )}
    </Show>
  );
}
