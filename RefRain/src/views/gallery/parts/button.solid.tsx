// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { say } from "../../../core/lang";
import { ui } from "../../../ui";
import Button from "../../parts/button.solid";
import IconButton from "../../parts/icon_button.solid";
import { GalleryCase } from "../case";
import { storeValue } from "../../../core/solid_store";

export default function ButtonCases() {
  const lang = storeValue(ui().lang);
  return (
    <>
      <GalleryCase label="button · four tones">
        <div class="flex flex-wrap items-center gap-snug">
          <Button label={say(lang(), "part_save")} tone="primary" />
          <Button label={say(lang(), "part_cancel")} tone="secondary" />
          <Button label={say(lang(), "dismiss")} tone="quiet" />
          <Button label={say(lang(), "part_delete")} tone="destructive" />
        </div>
      </GalleryCase>

      <GalleryCase label="button · loading and refused">
        <div class="flex flex-wrap items-center gap-snug">
          <Button label={say(lang(), "part_save")} tone="primary" loading />
          <Button
            label={say(lang(), "part_save")}
            tone="primary"
            why={say(lang(), "part_why_halted")}
          />
        </div>
      </GalleryCase>

      <GalleryCase label="button · icon keys, live and refused">
        <div class="flex flex-wrap items-center gap-snug">
          <IconButton glyph="copy" label={say(lang(), "setup_copy")} />
          <IconButton glyph="cross" label={say(lang(), "panel_close")} />
          <IconButton
            glyph="reveal"
            label={say(lang(), "path_reveal")}
            why={say(lang(), "part_why_halted")}
          />
        </div>
      </GalleryCase>
    </>
  );
}
