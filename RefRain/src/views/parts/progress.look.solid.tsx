// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { ProgressLook } from "./progress";

export default function ProgressLookView(look: ProgressLook) {
  return (
    <>
      <div class={"flex w-full min-w-0 items-center gap-base"}>
        <div
          {...look.bar}
          class={[
            "h-snug min-w-0 flex-1 overflow-hidden rounded-pill bg-raised",
            look.reached === undefined && "pulse",
          ]}
        >
          {look.reached !== undefined ? (
            <>
              <div
                class={"h-full rounded-pill bg-progress-done"}
                style={{ width: String(Math.round(look.reached.share * 100)) + "%" }}
              />
            </>
          ) : undefined}
        </div>
        {look.reached !== undefined ? (
          <>
            <span class={"shrink-0 font-mono text-note text-text-quiet"}>
              {look.reached.done}
              {" / "}
              {look.reached.total}
            </span>
          </>
        ) : undefined}
      </div>
    </>
  );
}
