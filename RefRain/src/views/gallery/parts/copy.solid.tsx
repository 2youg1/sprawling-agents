// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { For } from "solid-js";
import { fill, say } from "../../../core/lang";
import { ui } from "../../../ui";
import { lookOf, REST } from "../../parts/copy";
import type { Copied } from "../../parts/copy";
import Copy from "../../parts/copy.solid";
import CopyLook from "../../parts/copy.look.solid";
import Button from "../../parts/button.solid";
import UsageExportLook from "../../parts/usage_export.look.solid";
import { GalleryCase } from "../case";
import { storeValue } from "../../../core/solid_store";

export default function CopyCases() {
  const lang = storeValue(ui().lang);
  const COMMAND = "cargo install sprawling --locked";
  const STATES: readonly Copied[] = [
    REST,
    { kind: "copied" },
    { kind: "refused", why: { kind: "refused", said: "Document is not focused." } },
    { kind: "refused", why: { kind: "absent" } },
  ];
  const idle = (): void => undefined;
  const UNREAD = "the usage index is still being built";
  return (
    <>
      <GalleryCase label="copy · beside a command, at rest, copied, refused, and with no clipboard">
        <div class="flex flex-col gap-snug">
          <div class="flex min-w-0 items-center gap-snug">
            <code class="min-w-0 truncate font-mono text-note text-text">{COMMAND}</code>
            <Button label={say(lang(), "link_retry")} tone="quiet" />
            <Copy text={COMMAND} />
          </div>
          <For
            each={Array.from(STATES ?? []).map((__rrItem0, __rrIndex1) => ({
              value: __rrItem0,
              key: __rrIndex1,
            }))}
            keyed={(entry) => entry.key}
          >
            {(__rrItem0, __rrIndex1) => (
              <>
                <div class="flex min-w-0 items-center gap-snug">
                  <CopyLook {...lookOf({ text: COMMAND }, __rrItem0().value, lang(), idle)} />
                  <CopyLook
                    {...lookOf(
                      { text: COMMAND, note: say(lang(), "code_copy_note"), form: "bare" },
                      __rrItem0().value,
                      lang(),
                      idle,
                    )}
                  />
                </div>
              </>
            )}
          </For>
        </div>
      </GalleryCase>
      <GalleryCase label="usage export · the city could not answer the last press">
        <UsageExportLook
          buttons={[
            {
              format: "jsonl",
              label: say(lang(), "usage_export_jsonl"),
              loading: false,
              press: idle,
            },
            { format: "csv", label: say(lang(), "usage_export_csv"), loading: false, press: idle },
          ]}
          unavailable={{
            said: fill(say(lang(), "answer_unavailable"), { query: "usage_export" }),
            reason: UNREAD,
          }}
        />
      </GalleryCase>
    </>
  );
}
