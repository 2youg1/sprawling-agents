// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { JSX } from "@solidjs/web";
import { onSettled } from "solid-js";
import type { Backdrop, First, Named, Seat } from "../modal";

interface Props {
  readonly seat: Seat;
  readonly named: Named;
  readonly backdrop: Backdrop;
  readonly first?: First | undefined;
  readonly onClose: () => void;
  readonly children: JSX.Element;
}

export default function CommandDialog(props: Props) {
  let dialog: HTMLDialogElement | undefined;
  function cancel(event: Pick<Event, "preventDefault">): void {
    event.preventDefault();
    if (props.seat === "modal") props.onClose();
  }
  onSettled(() => {
    const node = dialog;
    if (node === undefined || props.seat === "specimen") return;
    const before = document.activeElement;
    if (props.first === "sheet") {
      node.autofocus = true;
      node.tabIndex = -1;
    }
    node.showModal();
    if (props.first === "sheet") node.focus({ preventScroll: true });
    return () => {
      node.close();
      const active = document.activeElement;
      if (
        (active === document.body || active === null) &&
        before instanceof HTMLElement &&
        before.isConnected
      )
        before.focus({ preventScroll: true });
    };
  });
  return (
    <dialog
      ref={(node) => {
        dialog = node;
      }}
      aria-label={"aria-label" in props.named ? props.named["aria-label"] : undefined}
      aria-labelledby={
        "aria-labelledby" in props.named ? props.named["aria-labelledby"] : undefined
      }
      open={props.seat === "specimen" ? true : undefined}
      onCancel={cancel}
      onKeyDown={(event) => {
        if (event.key === "Escape" && !event.defaultPrevented && !event.isComposing) cancel(event);
      }}
      onClick={(event) => {
        if (
          props.seat === "modal" &&
          props.backdrop === "closes" &&
          event.target === event.currentTarget
        )
          props.onClose();
      }}
      class={[
        "m-auto mt-section hidden w-full max-w-measure rounded-panel bg-raised p-0 opacity-0 shadow-sheet",
        "transition-[opacity,display,overlay] transition-discrete duration-panel ease-leave",
        "open:block open:opacity-100 open:ease-arrive starting:open:opacity-0 still:transition-none",
        "backdrop:bg-transparent backdrop:backdrop-brightness-50",
        { static: props.seat === "specimen" },
      ]}
    >
      <div class="flex flex-col gap-snug p-snug">{props.children}</div>
    </dialog>
  );
}
