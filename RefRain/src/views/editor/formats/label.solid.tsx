// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { For, createMemo } from "solid-js";
import { fill, say } from "../../../core/lang";
import { ui } from "../../../ui";
import { short } from "../reading";
import { NAME } from "./format";
import type { Drawn } from "./format";
interface Props {
  readonly format: Drawn;
  readonly tool: string;
  readonly settings: readonly string[];

  readonly version: string | null;
  readonly notes?: readonly string[];
}
import { storeValue } from "../../../core/solid_store";

export type PortProps = Props;

export default function EditorFormatsLabel(props: Props) {
  const lang = ui().lang;
  const $lang = storeValue(lang);
  const line = createMemo(() =>
    [
      props.tool,
      ...props.settings,
      ...(props.version === null
        ? []
        : [fill(say($lang(), "format_version"), { version: short(props.version) })]),
    ].join(" · "),
  );
  return (
    <>
      <div class={"border-b border-edge px-wide py-tight text-note text-text-faint"}>
        <p>
          <span class={"text-text-quiet"}>{NAME[props.format]}</span>
          {" · "}
          {line()}
        </p>
        <For
          each={Array.from((props.notes === undefined ? [] : props.notes) ?? []).map(
            (__rrItem0, __rrIndex1) => ({
              value: __rrItem0,
              key: __rrItem0,
            }),
          )}
          keyed={(entry) => entry.key}
        >
          {(__rrItem0) => (
            <>
              <p>{__rrItem0().value}</p>
            </>
          )}
        </For>
      </div>
    </>
  );
}
