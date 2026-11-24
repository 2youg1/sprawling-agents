// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { JSX } from "@solidjs/web";
import { createEffect, createMemo, createUniqueId } from "solid-js";

import { lookOf, type DialogProps } from "./dialog";
import DialogLookView from "./dialog.look.solid";

export default function Dialog(props: DialogProps<JSX.Element>) {
  const uid = createUniqueId();
  const look = createMemo(() => lookOf(props, uid));
  let sheet: HTMLDialogElement | undefined;

  createEffect(
    () => props.open,
    (open) => {
      if (sheet === undefined || sheet.open === open) return;
      if (open) sheet.showModal();
      else sheet.close();
    },
  );

  return (
    <DialogLookView
      look={look()}
      handle={(node) => {
        sheet = node;
      }}
    />
  );
}
