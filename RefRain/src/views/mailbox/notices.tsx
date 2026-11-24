// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo, For, Show } from "solid-js";
import type { Notice } from "../../core/belief";
import { urgencyOf } from "../../core/deferral";
import { say, type Lang } from "../../core/lang";
import { storeValue } from "../../core/solid_store";
import { clock } from "../../core/time";
import { ui } from "../../ui";
import Button from "../parts/button.solid";
import NoticeEntry from "./notice_entry";
import Section from "./section";
import { noticeSweeps, shown, sweep } from "./swept";

function dayOf(lang: Lang, at: number, now: number): string {
  const day = new Date(at).toDateString();
  if (day === new Date(now).toDateString()) return say(lang, "notices_today");
  if (day === new Date(now - 86_400_000).toDateString()) return say(lang, "notices_yesterday");
  return clock(lang, at);
}

export default function Notices() {
  const u = ui();
  const lang = storeValue(u.lang);
  const belief = storeValue(u.conn.belief);
  const sweeps = storeValue(noticeSweeps);
  const visible = createMemo(() =>
    belief().notices.filter(
      (notice) => urgencyOf(notice.error) === "ordinary" && shown(notice, sweeps()),
    ),
  );
  const days = createMemo(() => {
    const groups = new Map<string, Notice[]>();
    for (const notice of [...visible()].reverse()) {
      const id = new Date(notice.at).toDateString();
      const held = groups.get(id) ?? [];
      held.push(notice);
      groups.set(id, held);
    }
    return [...groups].map(([id, items]) => ({
      id,
      label: dayOf(lang(), items[0]?.at ?? 0, u.now()),
      items,
    }));
  });
  return (
    <Section
      title={say(lang(), "notices")}
      empty={say(lang(), "notices_none")}
      count={visible().length}
      tools={
        <Show when={visible().length > 0}>
          <Button
            tone="quiet"
            label={say(lang(), "notices_clear")}
            onPress={() => {
              sweep(visible());
            }}
          />
        </Show>
      }
    >
      <For each={days()} keyed={(day) => day.id}>
        {(day) => (
          <>
            <h4 class="pt-snug text-note text-text-faint">{day().label}</h4>
            <ul>
              <For each={day().items} keyed={(notice) => notice.key}>
                {(notice) => (
                  <li>
                    <NoticeEntry notice={notice()} blocking={false} />
                  </li>
                )}
              </For>
            </ul>
          </>
        )}
      </For>
    </Section>
  );
}
