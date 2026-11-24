// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { JSX } from "@solidjs/web";
import { createEffect, onSettled } from "solid-js";

import type { Stands } from "./sheet";
import { FACE, STANDS } from "./sheet_paint";

interface SheetProps {
  readonly open: boolean;
  readonly stands: Stands;
  readonly title?: string;
  readonly label?: string;
  readonly description?: string | undefined;
  readonly onCancel: () => void;
  readonly children?: JSX.Element;
}

export default function ModalSheet(props: SheetProps) {
  let dialog: HTMLDialogElement | undefined;
  function sync() {
    if (!dialog?.isConnected || props.stands === "in-flow") return;
    if (props.open && !dialog.open) dialog.showModal();
    else if (!props.open && dialog.open) dialog.close();
  }
  createEffect(() => [props.open, props.stands] as const, sync);
  onSettled(() => {
    sync();
    return () => {
      if (dialog?.open && props.stands !== "in-flow") dialog.close();
    };
  });
  return (
    <dialog
      ref={(node) => {
        dialog = node;
      }}
      class={[FACE, STANDS[props.stands]]}
      open={props.stands === "in-flow" ? true : undefined}
      aria-label={props.label}
      aria-labelledby={props.title}
      aria-describedby={props.description}
      onCancel={(event) => {
        event.preventDefault();
        props.onCancel();
      }}
    >
      {props.children}
    </dialog>
  );
}
