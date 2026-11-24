// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { FieldLook } from "./field";

export default function FieldLookView(look: FieldLook) {
  const AFFIX = "shrink-0 font-mono text-note text-text-faint";
  return (
    <>
      <div class={"flex w-full min-w-0 flex-col gap-tight"}>
        <label
          {...look.labelWire}
          class={["text-note text-text-quiet", look.labelling === "hidden" && "sr-only"]}
        >
          {look.label}
        </label>
        <div
          class={[
            "flex h-control min-w-0 items-center gap-tight rounded-control border bg-raised px-base",
            look.refused ? "border-alert" : "border-edge-input",
            !look.disabled && "has-[:user-invalid]:border-alert",
          ]}
        >
          {look.prefix !== undefined ? (
            <>
              <span class={AFFIX}>{look.prefix}</span>
            </>
          ) : undefined}
          <input
            {...look.box}
            class={[
              "min-w-0 flex-1 bg-transparent text-body placeholder:text-text-faint",
              look.disabled ? "aria-disabled:text-text-disabled" : "text-text",
              look.mono && "font-mono",
            ]}
          />
          {look.suffix !== undefined ? (
            <>
              <span class={AFFIX}>{look.suffix}</span>
            </>
          ) : undefined}
        </div>
        {look.note !== undefined ? (
          <>
            <p
              {...look.note.wire}
              class={["text-note", look.note.kind === "error" ? "text-alert" : "text-text-faint"]}
            >
              {look.note.text}
            </p>
          </>
        ) : undefined}
      </div>
    </>
  );
}
