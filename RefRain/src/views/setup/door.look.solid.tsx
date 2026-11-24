// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { DoorLook } from "./door";

export default function DoorView(props: DoorLook) {
  return (
    <>
      <a
        {...props.wire}
        class={
          "inline-flex h-control w-fit items-center rounded-control border border-edge-input bg-raised px-base text-label text-text transition-colors duration-short ease-leave hover:bg-raised-hover hover:ease-arrive"
        }
      >
        {props.label}
      </a>
    </>
  );
}
