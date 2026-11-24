// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { For } from "solid-js";
import { fill, say, type Key } from "../../core/lang";
import { count } from "../../core/time";
import { ui } from "../../ui";
import type { CeilingNote, CeilingWhy } from "./trace";
interface Props {
  readonly notes: readonly CeilingNote[];
}
import { storeValue } from "../../core/solid_store";

export type PortProps = Props;

export default function MonitorCeilings(props: Props) {
  const { lang } = ui();
  const $lang = storeValue(lang);
  const WHY: Record<CeilingWhy, Key> = {
    platform: "mon_ceiling_why_platform",
    not_delegated: "mon_ceiling_why_not_delegated",
    refused: "mon_ceiling_why_refused",
    unjoined: "mon_ceiling_why_unjoined",
  };
  function said(note: CeilingNote): string {
    const limit = count(note.ceiling.limit);
    const line = (() => {
      switch (note.ceiling.state) {
        case "hit":
          return fill(say($lang(), "mon_ceiling_hit"), { limit });
        case "unapplied":
          return fill(say($lang(), "mon_ceiling_unapplied"), {
            limit,
            why: say($lang(), WHY[note.ceiling.why]),
          });
        case "unread":
          return fill(say($lang(), "mon_ceiling_unread"), { limit });
      }
    })();
    return note.handle === null
      ? line
      : fill(say($lang(), "mon_ceiling_of"), { handle: note.handle, line });
  }
  return (
    <>
      <For
        each={Array.from(props.notes ?? []).map((__rrItem0, __rrIndex1) => ({
          value: __rrItem0,
          key: __rrIndex1,
        }))}
        keyed={(entry) => entry.key}
      >
        {(__rrItem0, __rrIndex1) => (
          <>
            <p
              data-ceiling={__rrItem0().value.ceiling.state}
              class={"ps-[2ch] -indent-[2ch] whitespace-pre-wrap wrap-break-word text-alert"}
            >
              {said(__rrItem0().value)}
            </p>
          </>
        )}
      </For>
    </>
  );
}
