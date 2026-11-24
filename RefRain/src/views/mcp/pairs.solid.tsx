// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { For, Show, createMemo } from "solid-js";
import type { Pair } from "./draft";
export interface PairTableProps {
  readonly caption: string;
  readonly rows: readonly Pair[];
  readonly onChange: (rows: readonly Pair[]) => void;
  readonly note?: string;
}
import { say } from "../../core/lang";
import { ui } from "../../ui";
import Button from "../parts/button.solid";
import Field from "../parts/field.solid";
import { storeValue } from "../../core/solid_store";

export default function View(props: PairTableProps) {
  const { lang } = ui();
  const $lang = storeValue(lang);
  function written(at: number, pair: Pair): void {
    props.onChange(props.rows.map((row, index) => (index === at ? pair : row)));
  }
  function dropped(at: number): void {
    props.onChange(props.rows.filter((_, index) => index !== at));
  }
  return (
    <>
      <div class={"flex w-full min-w-0 flex-col gap-tight"}>
        <span class={"text-note text-text-quiet"}>{props.caption}</span>
        <For
          each={Array.from(props.rows ?? []).map((__rrItem0, __rrIndex1) => ({
            value: __rrItem0,
            key: __rrIndex1,
          }))}
          keyed={(entry) => entry.key}
        >
          {(__rrItem0, __rrIndex1) => (
            <>
              <div class={"flex min-w-0 items-center gap-tight"}>
                <Field
                  label={String(props.caption) + " " + String(say($lang(), "mcp_pair_name"))}
                  labelling={"hidden"}
                  mono
                  placeholder={say($lang(), "mcp_pair_name")}
                  value={__rrItem0().value.name}
                  onInput={(value) => {
                    written(__rrIndex1(), { name: value, value: __rrItem0().value.value });
                  }}
                />
                <Field
                  label={String(props.caption) + " " + String(say($lang(), "mcp_pair_value"))}
                  labelling={"hidden"}
                  mono
                  placeholder={say($lang(), "mcp_pair_value")}
                  value={__rrItem0().value.value}
                  onInput={(value) => {
                    written(__rrIndex1(), { name: __rrItem0().value.name, value });
                  }}
                />
                <Button
                  label={say($lang(), "mcp_pair_remove")}
                  tone={"quiet"}
                  onPress={() => {
                    dropped(__rrIndex1());
                  }}
                />
              </div>
            </>
          )}
        </For>
        <div class={"flex items-center gap-base"}>
          <span class={"shrink-0"}>
            <Button
              label={say($lang(), "mcp_pair_add")}
              tone={"quiet"}
              onPress={() => {
                props.onChange([...props.rows, { name: "", value: "" }]);
              }}
            />
          </span>
          {(() => {
            const __rrBranch3 = createMemo(() => {
              const __rrCaptured2 = props.note;
              return __rrCaptured2 !== undefined
                ? { yes: { __rrCaptured2 }, no: undefined }
                : { yes: undefined, no: { __rrCaptured2 } };
            });
            return (
              <Show when={__rrBranch3().yes}>
                {(__rrActive4) => (
                  <>
                    <span class={"min-w-0 text-note text-text-faint"}>
                      {__rrActive4().__rrCaptured2}
                    </span>
                  </>
                )}
              </Show>
            );
          })()}
        </div>
      </div>
    </>
  );
}
