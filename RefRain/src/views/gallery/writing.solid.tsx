// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createSignal, Show } from "solid-js";

import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import WorkspaceDocument from "../editor/workspace_document";
import { GalleryCase } from "./case";

const TEXT = "# Meeting notes\n\n周五交付初稿，下周一集中讨论。\n\n- 整理样例\n- 补齐设计说明\n";
const VERSION = "d9a3e19dae2568ccf814aab789fdd808e36c86733a7e6184530c71a5925618d6";
const BYTES = new TextEncoder().encode(TEXT);

export default function WritingCases() {
  const lang = storeValue(ui().lang);
  const [open, setOpen] = createSignal(true);
  return (
    <GalleryCase label="Markdown · writing surface, save mark and left readout" width={1440}>
      <div class="@container/shell">
        <div class="frame h-96 overflow-hidden bg-page">
          <div class="refrain-workspace col-span-full row-span-full grid min-h-0 grid-cols-subgrid grid-rows-subgrid" data-subject="files">
            <div class="refrain-canvas">
              <Show when={open()}>
                <WorkspaceDocument name="Meeting.md" bytes={BYTES} version={VERSION} lang={lang()}
                  save={() => Promise.resolve({ version: VERSION })} close={() => setOpen(false)} />
              </Show>
            </div>
          </div>
        </div>
      </div>
    </GalleryCase>
  );
}
