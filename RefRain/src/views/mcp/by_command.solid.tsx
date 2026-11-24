// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { Show, createMemo, createSignal, untrack } from "solid-js";
import type { Draft, Intake } from "./draft";
export interface ByCommandProps {
  readonly intake: Intake;
}
import { EMPTY, WHY, encode } from "./draft";
import { say } from "../../core/lang";
import { ui } from "../../ui";
import Button from "../parts/button.solid";
import Field from "../parts/field.solid";
import PairTable from "./pairs.solid";
import { storeValue } from "../../core/solid_store";

export default function View(props: ByCommandProps) {
  const { lang } = ui();
  const $lang = storeValue(lang);
  const __rrInitial4 = untrack(() => EMPTY);
  const [draft, __rrSet0] = createSignal<Draft>(() => __rrInitial4);
  const encoded = createMemo(() => encode(draft(), props.intake.taken()));
  const reason = createMemo((): string | null => {
    const scope = props.intake.why();
    if (scope !== null) return scope;
    const result = encoded();
    return result.kind === "blocked" ? say($lang(), WHY[result.blocker]) : null;
  });
  function send(): void {
    const out = encoded();
    if (out.kind === "ready" && props.intake.offer(out.server)) {
      __rrSet0(() => EMPTY);
    }
  }
  return (
    <>
      <div class={"flex flex-col gap-base"}>
        <div class={"grid gap-base grid-cols-[repeat(auto-fit,minmax(320px,1fr))]"}>
          <Field
            label={say($lang(), "mcp_label")}
            help={say($lang(), "mcp_label_help")}
            mono
            value={draft().label}
            onInput={(label) => {
              __rrSet0(() => ({ ...draft(), label }));
            }}
          />
          <Field
            label={say($lang(), "mcp_command")}
            help={say($lang(), "mcp_command_help")}
            mono
            value={draft().command}
            onInput={(command) => {
              __rrSet0(() => ({ ...draft(), command }));
            }}
          />
        </div>
        <PairTable
          caption={say($lang(), "mcp_env")}
          note={say($lang(), "mcp_env_help")}
          rows={draft().env}
          onChange={(env) => {
            __rrSet0(() => ({ ...draft(), env }));
          }}
        />
        {(() => {
          const __rrBranch2 = createMemo(() => {
            const __rrCaptured1 = reason();
            return __rrCaptured1 === null
              ? { yes: { __rrCaptured1 }, no: undefined }
              : { yes: undefined, no: { __rrCaptured1 } };
          });
          return (
            <Show
              when={__rrBranch2().yes}
              fallback={
                <Show when={__rrBranch2().no}>
                  {(__rrActive3) => (
                    <>
                      <div class={"flex min-w-0 items-center gap-base"}>
                        <Button label={say($lang(), "mcp_add")} why={__rrActive3().__rrCaptured1} />
                        <span class={"min-w-0 text-note text-text-faint"}>
                          {__rrActive3().__rrCaptured1}
                        </span>
                      </div>
                    </>
                  )}
                </Show>
              }
            >
              {(__rrActive3) => (
                <>
                  <Button label={say($lang(), "mcp_add")} tone={"primary"} onPress={send} />
                </>
              )}
            </Show>
          );
        })()}
      </div>
    </>
  );
}
