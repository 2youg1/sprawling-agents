// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createEffect } from "solid-js";
import { get } from "../core/store";
import { storeValue } from "../core/solid_store";
import { say } from "../core/lang";
import { notices, UNHEARD, type Heard } from "../core/notify";
import type { View } from "../core/route";
import { ui } from "../ui";

export function watchNotifications(view: () => View): void {
  const u = ui();
  const held = storeValue(u.prefs.held);
  const items = storeValue(u.approvals);
  const opened = Date.now();
  let heard: Heard = UNHEARD;
  createEffect(
    () => [view(), items(), held().notifying] as const,
    ([page, approvals, notifying]) => {
      const [next, raised] = notices(heard, approvals, {
        notifying,
        focus: document.hasFocus() ? "focused" : "blurred",
        elapsed: Date.now() - opened,
        watching: page.kind === "talk" ? page.address : null,
      });
      heard = next;
      if (typeof Notification === "undefined" || Notification.permission !== "granted") return;
      for (const item of raised) {
        new Notification(say(get(u.lang), "notify_approval"), {
          body: item.action_desc,
          tag: item.id,
        });
      }
    },
  );
}
