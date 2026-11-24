// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo, For, Show } from "solid-js";
import type { Notice as NoticeRecord } from "../../core/belief";
import { fill, say } from "../../core/lang";
import { recoveryFor } from "../../core/recovering";
import { storeValue } from "../../core/solid_store";
import { ago } from "../../core/time";
import { ui } from "../../ui";
import { recover, recoveryLabel, recoveryWhy } from "../notice_recovery";
import Button from "../parts/button.solid";
import Notice from "../parts/notice_view";
import { WHY } from "./pending";
import { sweep } from "./swept";

export default function NoticeEntry(props: {
  readonly notice: NoticeRecord;
  readonly blocking: boolean;
}) {
  const u = ui();
  const lang = storeValue(u.lang);
  const recovery = createMemo(() => recoveryFor(props.notice.error));
  function actions() {
    return (
      <>
        <For each={recovery()} keyed={(lever) => recoveryLabel(lever, lang())}>
          {(lever) => {
            const unavailable = createMemo(() => {
              const reason = recoveryWhy(u, lever(), props.notice);
              return reason === undefined ? {} : { why: say(lang(), reason) };
            });
            return (
              <Button
                tone="quiet"
                label={recoveryLabel(lever(), lang())}
                {...unavailable()}
                onPress={() => {
                  recover(u, lever(), props.notice);
                }}
              />
            );
          }}
        </For>
        <Show when={props.blocking}>
          <Button
            tone="quiet"
            label={say(lang(), "dismiss")}
            onPress={() => {
              sweep([props.notice]);
            }}
          />
        </Show>
      </>
    );
  }
  return (
    <div
      class="flex min-w-0 items-start gap-snug rounded-card focus-visible:wash"
      data-entry=""
      tabindex={-1}
    >
      <div class="min-w-0 flex-1">
        <Notice
          seat="drawer"
          weight="alert"
          action={props.notice.error.action}
          code={props.notice.error.code}
          subject={props.notice.error.subject}
          recovery={props.notice.error.recovery}
          at={ago(lang(), props.notice.at, u.now())}
          count={props.notice.count}
          actions={actions}
        />
        <Show when={props.blocking}>
          <p class="mt-tight text-note text-text-quiet">
            {fill(say(lang(), WHY.failure), { code: props.notice.error.code })}
          </p>
        </Show>
      </div>
      <kbd class="entry-n mt-snug" aria-hidden="true" />
    </div>
  );
}
