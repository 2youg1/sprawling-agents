<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // The adapter between `core/notify.ts` and the browser's
  // `Notification`: it follows the approval queue, asks the decision
  // which items to raise, and raises them only where the browser has
  // granted the permission. It draws nothing.
  import { say } from "../core/lang";
  import { notices, UNHEARD, type Heard } from "../core/notify";
  import type { View } from "../core/route";
  import { ui } from "../ui";

  const { view }: { view: View } = $props();

  const u = ui();
  const held = u.prefs.held;
  const lang = u.lang;
  const approvals = u.approvals;
  const opened = Date.now();
  let heard: Heard = UNHEARD;

  $effect(() => {
    const [next, raised] = notices(heard, $approvals, {
      notifying: $held.notifying,
      focus: document.hasFocus() ? "focused" : "blurred",
      elapsed: Date.now() - opened,
      watching: view.kind === "talk" ? view.address : null,
    });
    heard = next;
    if (typeof Notification === "undefined" || Notification.permission !== "granted") return;
    for (const item of raised) {
      new Notification(say($lang, "notify_approval"), { body: item.action_desc, tag: item.id });
    }
  });
</script>
