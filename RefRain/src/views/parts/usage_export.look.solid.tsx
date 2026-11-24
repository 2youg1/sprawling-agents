// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { For } from "solid-js";
import Button from "./button.solid";
import type { UsageExportLook } from "./usage_export";

export type PortProps = UsageExportLook;

export default function PartsUsageExportLook(look: UsageExportLook) {
  return (
    <>
      <div class={"flex flex-wrap items-center gap-snug"}>
        <For
          each={Array.from(look.buttons ?? []).map((__rrItem0, __rrIndex1) => ({
            value: __rrItem0,
            key: __rrItem0.format,
          }))}
          keyed={(entry) => entry.key}
        >
          {(__rrItem0) => (
            <>
              <Button
                label={__rrItem0().value.label}
                tone={"quiet"}
                loading={__rrItem0().value.loading}
                onPress={__rrItem0().value.press}
              />
            </>
          )}
        </For>
        {look.unavailable !== undefined ? (
          <>
            <p class={"flex basis-full flex-col gap-tight text-note text-text-faint wrap-anywhere"}>
              <span>{look.unavailable.said}</span>
              {look.unavailable.reason !== undefined ? (
                <>
                  <span class={"font-mono"}>{look.unavailable.reason}</span>
                </>
              ) : undefined}
            </p>
          </>
        ) : undefined}
      </div>
    </>
  );
}
