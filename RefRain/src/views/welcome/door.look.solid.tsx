// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { DoorLook } from "./door";

export type PortProps = DoorLook;

export default function WelcomeDoorLook(look: DoorLook) {
  return (
    <>
      <a
        {...look.wire}
        class={
          "inline-flex h-control items-center self-start text-label font-label text-accent transition-colors ease-leave hover:text-accent-hover hover:ease-arrive"
        }
      >
        {look.label}
      </a>
    </>
  );
}
