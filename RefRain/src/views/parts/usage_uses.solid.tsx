// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo } from "solid-js";
import { fill, say } from "../../core/lang";
import { clock } from "../../core/time";
import { ui } from "../../ui";
import type { DayCount } from "../../wire";
import { dayLine, outcomeWord } from "./usage_uses";
import type { UseRow, UsesLook } from "./usage_uses";
import Look from "./usage_uses.look.solid";
interface Props {
  readonly uses: readonly UseRow[];
  readonly days: readonly DayCount[];
}
import { storeValue } from "../../core/solid_store";

export type PortProps = Props;

export default function PartsUsageUses(props: Props) {
  const { lang } = ui();
  const $lang = storeValue(lang);
  const look = createMemo((): UsesLook => {
    const last = props.uses.at(-1);
    if (last === undefined) return { kind: "never", said: say($lang(), "usage_never") };
    return {
      kind: "used",
      count: fill(say($lang(), "usage_count"), {
        n: String(props.uses.length),
        when: clock($lang(), last.at),
      }),
      days: dayLine(props.days),
      fold: say($lang(), "usage_every_use"),
      uses: props.uses.map((used) => ({
        at: clock($lang(), used.at),
        who: used.resident ?? used.run,
        part: used.part ?? undefined,
        outcome: say($lang(), outcomeWord(used.outcome)),
      })),
    };
  });
  return (
    <>
      <Look {...look()} />
    </>
  );
}
