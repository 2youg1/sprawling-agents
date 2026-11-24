// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { Show, createMemo, createSignal, untrack } from "solid-js";
import { EMPTY } from "./draft";
import type { Draft, Intake, Transport } from "./draft";
export interface ByUrlProps {
  readonly intake: Intake;
  readonly onCommandDoor: () => void;
}
const TRANSPORTS: readonly Transport[] = ["http", "sse", "stdio"];
const OVER_A_URL: Draft = { ...EMPTY, transport: "http" };
import { SPELLED, WHY, encode } from "./draft";
import { say } from "../../core/lang";
import type { Choice } from "../parts/segmented";
import { ui } from "../../ui";
import Button from "../parts/button.solid";
import Field from "../parts/field.solid";
import PairTable from "./pairs.solid";
import Segmented from "../parts/segmented.solid";
import { storeValue } from "../../core/solid_store";

export default function View(props: ByUrlProps) {
  const { lang } = ui();
  const $lang = storeValue(lang);
  const __rrInitial4 = untrack(() => OVER_A_URL);
  const [draft, __rrSet0] = createSignal<Draft>(() => __rrInitial4);
  const encoded = createMemo(() => encode(draft(), props.intake.taken()));
  const reason = createMemo((): string | null => {
    const scope = props.intake.why();
    if (scope !== null) return scope;
    const result = encoded();
    return result.kind === "blocked" ? say($lang(), WHY[result.blocker]) : null;
  });
  const transports = createMemo(() =>
    TRANSPORTS.map((each): Choice<Transport> => ({
      value: each,
      label: say($lang(), SPELLED[each]),
    })),
  );
  function send(): void {
    const out = encoded();
    if (out.kind === "ready" && props.intake.offer(out.server)) {
      __rrSet0(() => OVER_A_URL);
    }
  }
  function pick(transport: Transport): void {
    if (transport === "stdio") {
      props.onCommandDoor();
      return;
    }
    __rrSet0(() => ({ ...draft(), transport }));
  }
  return (
    <>
      <div class={"flex flex-col gap-base"}>
        <div class={"flex flex-wrap items-center gap-snug"}>
          <span class={"text-note text-text-quiet"}>{say($lang(), "mcp_transport")}</span>
          <Segmented
            label={say($lang(), "mcp_transport")}
            options={transports()}
            held={draft().transport}
            onPick={pick}
          />
        </div>
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
            label={say($lang(), "mcp_url")}
            mono
            value={draft().url}
            onInput={(url) => {
              __rrSet0(() => ({ ...draft(), url }));
            }}
          />
        </div>
        <PairTable
          caption={say($lang(), "mcp_header")}
          note={say($lang(), "mcp_header_help")}
          rows={draft().headers}
          onChange={(headers) => {
            __rrSet0(() => ({ ...draft(), headers }));
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
