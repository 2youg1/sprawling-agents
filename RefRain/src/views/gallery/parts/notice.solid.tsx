// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { For } from "solid-js";
interface Refusal {
  readonly action: string;
  readonly code: string;
  readonly subject: string;
  readonly recovery: string;
}
interface Entry {
  readonly at: string;
  readonly count: number;
  readonly one: Refusal;
}
function refusal(code: string, subject: string, recovery: string): Refusal {
  return { action: "attach an endpoint", code, subject, recovery };
}
function entry(at: string, count: number, one: Refusal): Entry {
  return { at, count, one };
}
const MISSING_KEY = refusal(
  "E_CREDENTIAL_MISSING",
  "example",
  "file a key for this provider, then attach it again",
);
const REFUSED_LINK = refusal("E_LINK_REFUSED", "local", "start the endpoint, then try this again");
const STALE_DRAFT = refusal(
  "E_DRAFT_STALE",
  "config.toml",
  "reload the file, then make the change again",
);
const DRAWER: readonly Entry[] = [
  entry("03:35", 3, MISSING_KEY),
  entry("03:34", 1, REFUSED_LINK),
  entry("03:33", 1, STALE_DRAFT),
  entry("03:31", 2, MISSING_KEY),
  entry("03:30", 1, REFUSED_LINK),
  entry("03:28", 1, STALE_DRAFT),
  entry("03:27", 1, MISSING_KEY),
  entry("03:26", 1, REFUSED_LINK),
  entry("03:25", 1, STALE_DRAFT),
  entry("03:24", 1, MISSING_KEY),
  entry("03:23", 1, REFUSED_LINK),
  entry("03:22", 1, STALE_DRAFT),
];
import { say } from "../../../core/lang";
import { ui } from "../../../ui";
import Button from "../../parts/button.solid";
import Notice from "../../parts/notice_view";
import { GalleryCase } from "../case";
import { storeValue } from "../../../core/solid_store";

export default function NoticeCases() {
  const lang = storeValue(ui().lang);
  return (
    <>
      <GalleryCase label="notice · inline beneath what it is about">
        <Notice
          seat="inline"
          weight="alert"
          action={MISSING_KEY.action}
          code={MISSING_KEY.code}
          subject={MISSING_KEY.subject}
          recovery={MISSING_KEY.recovery}
          at="03:35"
          count={3}
        />
      </GalleryCase>
      <GalleryCase label="notices · toasts">
        <div class="flex min-h-output w-full items-end justify-end rounded-card border border-dashed border-edge-input p-snug">
          <div class="flex min-w-0 max-w-measure flex-col gap-tight">
            <Notice
              seat="toast"
              action={MISSING_KEY.action}
              code={MISSING_KEY.code}
              subject={MISSING_KEY.subject}
              recovery={MISSING_KEY.recovery}
              at="03:35"
              count={3}
              actions={() => (
                <>
                  <Button label={say(lang(), "part_undo")} tone="quiet" />
                  <Button label={say(lang(), "dismiss")} tone="quiet" />
                </>
              )}
            />
            <Notice
              seat="toast"
              weight="alert"
              action={REFUSED_LINK.action}
              code={REFUSED_LINK.code}
              subject={REFUSED_LINK.subject}
              recovery={REFUSED_LINK.recovery}
              at="03:34"
            />
          </div>
        </div>
      </GalleryCase>
      <GalleryCase label="notices · drawer with 12 entries" width={520}>
        <div class="max-h-output min-w-0 overflow-y-auto rounded-card border border-edge-panel">
          <For each={DRAWER}>
            {(entry) => (
              <Notice
                seat="drawer"
                weight="alert"
                action={entry.one.action}
                code={entry.one.code}
                subject={entry.one.subject}
                recovery={entry.one.recovery}
                at={entry.at}
                count={entry.count}
              />
            )}
          </For>
        </div>
      </GalleryCase>
    </>
  );
}
