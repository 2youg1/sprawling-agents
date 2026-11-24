// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import Badge from "../parts/badge.solid";
import type { RoomRowLook } from "./room_row";

export type PortProps = RoomRowLook;

export default function BuildingRoomRowLook(look: RoomRowLook) {
  return (
    <>
      <button
        data-control=""
        {...(() => {
          const { onclick: __rrArg0, ...__rrWire } = look.wire;
          return { ...__rrWire, onClick: __rrArg0 };
        })()}
        class={
          "flex h-step w-full items-center gap-snug rounded-control px-snug text-left leading-none text-text-quiet hover:wash"
        }
      >
        <span class={"min-w-0 flex-1 truncate"}>{look.name}</span>
        {look.live !== undefined ? (
          <>
            <Badge text={look.live} weight={"live"} dot />
          </>
        ) : undefined}
      </button>
    </>
  );
}
