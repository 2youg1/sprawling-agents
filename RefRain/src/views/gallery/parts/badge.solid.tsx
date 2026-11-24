// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { For } from "solid-js";
import { say } from "../../../core/lang";
import { storeValue } from "../../../core/solid_store";
import { ui } from "../../../ui";
import Badge from "../../parts/badge.solid";
import Glyph from "../../parts/glyph.solid";
import { GalleryCase } from "../case";

export default function BadgeCases() {
  const lang = storeValue(ui().lang);
  return (
    <>
      <GalleryCase label="badge · status, tier and dot">
        <div class="flex flex-wrap items-center gap-snug">
          <Badge text={say(lang(), "status_in_progress")} status="live" />
          <Badge text={say(lang(), "mcp_state_failed")} status="refused" />
          <Badge text={say(lang(), "mcp_state_connected")} weight="live" dot />
          <Badge text={say(lang(), "city_quiet")} dot />
          <Badge text={say(lang(), "setup_local")} />
        </div>
      </GalleryCase>
      <GalleryCase label="badge · counts on a key's corner: a few, past the cap, and new without a number">
        <div class="flex flex-wrap items-center gap-wide">
          <For each={[3, 120]}>
            {(count) => (
              <span class="relative inline-flex size-key items-center justify-center rounded-key bg-raised text-text-quiet">
                <Glyph name="inbox" size="key" />
                <span class="absolute -top-tight -right-tight">
                  <Badge count={count} />
                </span>
              </span>
            )}
          </For>
          <span class="relative inline-flex size-key items-center justify-center rounded-key bg-raised text-text-quiet">
            <Glyph name="inbox" size="key" />
            <span class="absolute top-tight right-tight flex">
              <Badge count="fresh" weight="quiet" />
            </span>
          </span>
        </div>
      </GalleryCase>
    </>
  );
}
