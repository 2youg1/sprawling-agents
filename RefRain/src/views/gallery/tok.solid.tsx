// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { For } from "solid-js";
import { storeValue } from "../../core/solid_store";
import type { Key } from "../../core/lang";
import type { GlyphName } from "../parts/glyph";
import { say } from "../../core/lang";
import { ui } from "../../ui";

import Glyph from "../parts/glyph.solid";
import Setup from "../setup_view";
import { GalleryCase as Case } from "./case";
import { ENDPOINTS } from "./served";

export default function Tok() {
  const KEYS: readonly { readonly glyph: GlyphName; readonly name: Key }[] = [
    { glyph: "app-grid", name: "edge_layers" },
    { glyph: "inbox", name: "edge_mailbox" },
    { glyph: "settings", name: "nav_settings" },
  ];

  const BEHIND = Array.from(
    { length: 9 },
    (_, n) =>
      `Sprawling/city/src/document.rs:${String(40 + n * 7)} reads the version the person saw`,
  );

  const CORNERS: readonly { readonly box: string; readonly size: string }[] = [
    { box: "rounded-panel", size: "h-[96px] w-[160px]" },
    { box: "rounded-card", size: "h-[64px] w-[120px]" },
    { box: "rounded-control", size: "h-control w-[96px]" },
    { box: "rounded-key", size: "size-key" },
    { box: "rounded-pill", size: "h-control-sm w-[96px]" },
  ];
  const lang = storeValue(ui().lang);
  function Over(props: { readonly glass: "on" | "off" }) {
    return (
      <div class="relative h-[200px] overflow-hidden bg-page" data-glass={props.glass}>
        <div
          class="flex flex-col gap-tight p-snug font-mono text-note text-text-quiet"
          aria-hidden="true"
        >
          <For each={BEHIND} keyed={true}>
            {(line) => <span>{line}</span>}
          </For>
        </div>
        <div class="absolute bottom-snug left-wide flex flex-col gap-snug">
          <For each={KEYS} keyed={(key) => key.glyph}>
            {(key) => (
              <button
                type="button"
                class="glass grid h-touch w-touch place-items-center rounded-control"
                data-control="surface"
                aria-label={say(lang(), key().name)}
              >
                <Glyph name={key().glyph} size="key" />
              </button>
            )}
          </For>
        </div>
      </div>
    );
  }

  return (
    <>
      <Case label="surfaces · the edge keys as glass over a page of words" width={360}>
        <Over glass="on" />
      </Case>

      <Case label="surfaces · the edge keys with glass turned solid" width={360}>
        <Over glass="off" />
      </Case>

      <Case label="surfaces · every rounded box at the one corner exponent" width={760}>
        <div class="flex flex-wrap items-end gap-wide">
          <For each={CORNERS} keyed={(corner) => corner.box}>
            {(corner) => (
              <span
                class={[corner().box, corner().size, "shrink-0 border border-edge-panel bg-raised"]}
              ></span>
            )}
          </For>
        </div>
      </Case>

      <Case
        label="surfaces · the appearance group, where glass and the blend opacity are chosen"
        width={1280}
      >
        <Setup
          group="appearance"
          endpoints={ENDPOINTS}
          autonomy={undefined}
          onAutonomy={() => undefined}
        />
      </Case>
    </>
  );
}
