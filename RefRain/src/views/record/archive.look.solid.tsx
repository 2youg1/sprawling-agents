// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { For } from "solid-js";
import type { ArchiveLook } from "./archive";

export type PortProps = ArchiveLook;

export default function RecordArchiveLook(look: ArchiveLook) {
  return (
    <>
      <ul class={"text-note"}>
        <For
          each={Array.from(look.hits ?? []).map((__rrItem0, __rrIndex1) => ({
            value: __rrItem0,
            key: __rrItem0.key,
          }))}
          keyed={(entry) => entry.key}
        >
          {(__rrItem0) => (
            <>
              <li class={"settled-row flex items-baseline gap-base border-b border-edge py-snug"}>
                <span class={"figure w-figure shrink-0 text-text-faint"}>
                  {__rrItem0().value.day}
                </span>
                <span class={"w-figure shrink-0 truncate text-text-faint"}>
                  {__rrItem0().value.kind}
                </span>
                <a
                  {...__rrItem0().value.wire}
                  class={
                    "block max-w-[24ch] shrink-0 truncate font-mono text-text-quiet transition-colors ease-leave still:transition-none hover:text-text hover:ease-arrive"
                  }
                >
                  {__rrItem0().value.building}
                </a>
                <span class={"flex-1 truncate text-text"}>{__rrItem0().value.subject}</span>
              </li>
            </>
          )}
        </For>
      </ul>
    </>
  );
}
