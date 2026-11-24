// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { Show } from "solid-js";
import Button from "./parts/button.solid";
import type { DesktopLook } from "./desktop";

export default function DesktopLookView(look: DesktopLook) {
  return (
    <div class="flex flex-col gap-snug">
      <textarea
        class="min-h-output w-full rounded-control border border-edge-input bg-raised px-base py-snug font-mono text-note text-text placeholder:text-text-faint"
        aria-label={look.box["aria-label"]}
        placeholder={look.box.placeholder}
        spellcheck={look.box.spellcheck}
        value={look.box.value}
        onInput={(event) => look.box.oninput(event)}
      />
      <Show when={look.missing}>{(text) => <p class="text-note text-text-faint">{text()}</p>}</Show>
      <div class="flex items-center gap-base">
        <Button
          label={look.save.label}
          tone="primary"
          {...(look.save.why === undefined ? {} : { why: look.save.why })}
          onPress={() => look.save.press()}
        />
        <Show when={look.none}>
          {(text) => <span class="text-note text-text-faint">{text()}</span>}
        </Show>
        <span class="flex-1" />
        <code class="truncate font-mono text-note text-text-faint">{look.scope}</code>
      </div>
    </div>
  );
}
