// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { TalkLinkLook } from "./talk_link";

export type PortProps = TalkLinkLook;

export default function BuildingTalkLinkLook(look: TalkLinkLook) {
  return (
    <>
      <a
        {...look.wire}
        class={
          "inline-flex h-control items-center rounded-control bg-raised px-base text-label text-text transition-colors ease-leave hover:bg-raised-hover hover:ease-arrive still:transition-none"
        }
      >
        {look.label}
      </a>
    </>
  );
}
