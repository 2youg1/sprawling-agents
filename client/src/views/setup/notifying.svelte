<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // The switch for browser notifications (client D7). Off until
  // the person turns it on, and the browser is asked for its permission
  // only at that moment, never when the page opens.
  import type { Key } from "../../core/lang";
  import { say } from "../../core/lang";
  import type { Notifying } from "../../core/notify";
  import { NOTIFYINGS } from "../../core/prefs";
  import { ui } from "../../ui";
  import Segmented from "../parts/segmented.svelte";

  const u = ui();
  const held = u.prefs.held;
  const lang = u.lang;

  const WORDS: Readonly<Record<Notifying, Key>> = { off: "notify_off", on: "notify_on" };

  // A switch that reads `on` while the browser refuses every
  // notification promises what never happens, so a browser that has no
  // notifications, or that says no, puts the switch back.
  function pick(next: Notifying): void {
    u.prefs.setNotifying(next);
    if (next === "off") return;
    if (typeof Notification === "undefined" || Notification.permission === "denied") {
      u.prefs.setNotifying("off");
      return;
    }
    if (Notification.permission === "default") {
      void Notification.requestPermission().then((answer) => {
        if (answer !== "granted") u.prefs.setNotifying("off");
      });
    }
  }
</script>

<div class="flex flex-col gap-tight rounded-card bg-raised px-base py-snug">
  <span class="text-label font-label text-text">{say($lang, "setup_notify")}</span>
  <p class="text-note text-text-faint">{say($lang, "setup_notify_note")}</p>
  <Segmented
    label={say($lang, "setup_notify")}
    options={NOTIFYINGS.map((each) => ({ value: each, label: say($lang, WORDS[each]) }))}
    held={$held.notifying}
    onPick={pick}
  />
</div>
