// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { AskMayorLook } from "./ask_mayor";

export type PortProps = AskMayorLook;

export default function CityAskMayorLook(look: AskMayorLook) {
  return (
    <>
      <a
        {...look.wire}
        class={
          "inline-flex h-control items-center rounded-control bg-accent px-base text-label text-on-accent transition-colors ease-leave hover:bg-accent-hover hover:ease-arrive"
        }
      >
        {look.label}
      </a>
    </>
  );
}
