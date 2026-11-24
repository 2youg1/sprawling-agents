// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { Show } from "solid-js";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import type { Address } from "../../wire";
import Glyph from "../parts/glyph.solid";
import { restriction, sandboxQuery, sandboxSaid } from "./sandbox";

export const FACT =
  "fact inline-flex h-control-sm min-w-0 max-w-[16rem] items-center gap-tight rounded-control px-snug text-note text-text-quiet";

export default function SandboxFact(props: { readonly room: Address | null }) {
  const u = ui();
  const lang = storeValue(u.lang);
  const building = storeValue(() =>
    props.room === null ? undefined : u.conn.asking.ask(sandboxQuery(props.room)),
  );
  return (
    <Show when={restriction(building())}>
      {(limits) => (
        <span class={FACT}>
          <Glyph name="sandbox" size="sm" />
          <span class="truncate">{sandboxSaid(limits(), lang())}</span>
        </span>
      )}
    </Show>
  );
}
