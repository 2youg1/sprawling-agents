// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { RunRowLook } from "./run_row";

export type PortProps = RunRowLook;

export default function BuildingRunRowLook(look: RunRowLook) {
  return (
    <>
      <a
        {...look.wire}
        class={"flex items-center gap-base rounded-control px-snug py-snug hover:wash"}
      >
        <span
          class={[
            "inline-block size-dot shrink-0 rounded-pill",
            look.live ? "bg-accent" : "bg-mark",
          ]}
        />
        <span class={"min-w-0 flex-1 truncate text-text-quiet"}>{look.task}</span>
        <span class={"shrink-0 text-text-faint"}>{look.posture}</span>
        {look.spent !== undefined ? (
          <>
            <span class={"shrink-0 text-text-faint"}>{look.spent}</span>
          </>
        ) : undefined}
        {look.started !== undefined ? (
          <>
            <span class={"shrink-0 text-text-faint"}>{look.started}</span>
          </>
        ) : undefined}
      </a>
    </>
  );
}
