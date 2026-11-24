// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { For, Show, createMemo } from "solid-js";
import { say } from "../../core/lang";
import { ui } from "../../ui";
import type { DoctorScanning } from "../../wire";
import { scanningOf } from "./scanning";
interface Props {
  readonly scanning: DoctorScanning;
}
import { storeValue } from "../../core/solid_store";

export type PortProps = Props;

export default function MachineScanning(props: Props) {
  const { lang } = ui();
  const $lang = storeValue(lang);
  return (
    <>
      <section
        class={"flex min-w-0 flex-col gap-tight"}
        aria-label={say($lang(), "machine_scan_title")}
      >
        <h2 class={"text-label font-label text-text"}>{say($lang(), "machine_scan_title")}</h2>
        <dl
          class={"grid grid-cols-[minmax(0,10rem)_minmax(0,1fr)] gap-x-base gap-y-tight text-note"}
        >
          <For
            each={Array.from(scanningOf(props.scanning) ?? []).map((__rrItem0, __rrIndex1) => ({
              value: __rrItem0,
              key: __rrItem0.subject,
            }))}
            keyed={(entry) => entry.key}
          >
            {(__rrItem0) => (
              <>
                <dt class={"text-text-faint"}>{say($lang(), __rrItem0().value.subject)}</dt>
                <dd class={"flex min-w-0 flex-wrap items-baseline gap-tight"}>
                  <span class={"text-text"}>{say($lang(), __rrItem0().value.found.key)}</span>
                  {(() => {
                    const __rrBranch3 = createMemo(() => {
                      const __rrCaptured2 = __rrItem0().value;
                      return __rrCaptured2.found.said !== null
                        ? { yes: { __rrCaptured2 }, no: undefined }
                        : { yes: undefined, no: { __rrCaptured2 } };
                    });
                    return (
                      <Show when={__rrBranch3().yes}>
                        {(__rrActive4) => (
                          <>
                            <span class={"min-w-0 break-all font-mono text-text-faint"}>
                              {__rrActive4().__rrCaptured2.found.said}
                            </span>
                          </>
                        )}
                      </Show>
                    );
                  })()}
                </dd>
              </>
            )}
          </For>
        </dl>
      </section>
    </>
  );
}
