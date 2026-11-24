// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { say } from "../../core/lang";
import type { Key, Lang } from "../../core/lang";
import type { Notifying } from "../../core/notify";
import Segmented from "../parts/segmented.solid";

export interface NotifyProps {
  readonly lang: Lang;
  readonly value: Notifying;
  readonly options: readonly Notifying[];
  readonly update: (next: Notifying) => void;
}

const WORDS: Readonly<Record<Notifying, Key>> = { off: "notify_off", on: "notify_on" };

export default function NotificationSetting(props: NotifyProps) {
  function pick(next: Notifying): void {
    props.update(next);
    if (next === "off") return;
    if (typeof Notification === "undefined" || Notification.permission === "denied") {
      props.update("off");
      return;
    }
    if (Notification.permission === "default") {
      void Notification.requestPermission().then((answer) => {
        if (answer !== "granted") props.update("off");
      });
    }
  }

  return (
    <div class="flex min-w-0 flex-col gap-snug rounded-card bg-raised px-base py-snug">
      <div class="flex flex-col gap-hair">
        <span class="text-label font-label text-text">{say(props.lang, "setup_notify")}</span>
        <p class="text-note text-text-faint">{say(props.lang, "setup_notify_note")}</p>
      </div>
      <Segmented
        label={say(props.lang, "setup_notify")}
        options={props.options.map((each) => ({
          value: each,
          label: say(props.lang, WORDS[each]),
        }))}
        held={props.value}
        onPick={pick}
      />
    </div>
  );
}
