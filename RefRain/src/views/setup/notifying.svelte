<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  import type { Key } from "../../core/lang";
  import { say } from "../../core/lang";
  import type { Notifying } from "../../core/notify";
  import { NOTIFYINGS } from "../../core/prefs";
  import { ui } from "../../ui";
  import Segmented from "../parts/segmented.svelte";
  import Card from "./card.look.svelte";

  const u = ui();
  const held = u.prefs.held;
  const lang = u.lang;

  const WORDS: Readonly<Record<Notifying, Key>> = { off: "notify_off", on: "notify_on" };

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

<Card title={say($lang, "setup_notify")} note={say($lang, "setup_notify_note")}>
  <Segmented
    label={say($lang, "setup_notify")}
    options={NOTIFYINGS.map((each) => ({ value: each, label: say($lang, WORDS[each]) }))}
    held={$held.notifying}
    onPick={pick}
  />
</Card>
