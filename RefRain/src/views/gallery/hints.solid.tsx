// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { say } from "../../core/lang";
import { ui } from "../../ui";
import Button from "../parts/button.solid";
import { GalleryCase } from "./case";
import { storeValue } from "../../core/solid_store";

export default function HintCases() {
  const lang = storeValue(ui().lang);
  return (
    <>
      <GalleryCase label="tip · near the top of the window, where a hint has to flip below">
        <Button
          label={say(lang(), "part_save")}
          tone="primary"
          why={say(lang(), "gallery_tip_tall")}
        />
      </GalleryCase>
      <GalleryCase label="tip · inside a box that scrolls, which clips or is escaped">
        <div class="h-output overflow-auto rounded-card border border-edge-panel p-base">
          <Button
            label={say(lang(), "part_save")}
            tone="primary"
            why={say(lang(), "part_why_halted")}
          />
          <div class="h-output" aria-hidden="true" />
        </div>
      </GalleryCase>
    </>
  );
}
