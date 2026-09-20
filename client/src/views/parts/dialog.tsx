// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The question asked before something that cannot be taken back. An
// action that can be undone is done straight away and offered back as a
// Notice; only the irreversible one stops and explains first.
//
// **The platform owns the modality.** `showModal()` puts the element in
// the top layer, traps the focus, marks the rest of the page `inert`,
// and answers Escape - four behaviours this file used to carry as a
// wrapper, a keydown handler, a stacking order and a focus call. The
// top layer also means no `z-index`: nothing on the page can be given a
// number that puts it over a modal dialog.
//
// The focus goes to the way out rather than to the way through: on a
// question about deleting something, the safe answer is the one already
// under the hand. Document order is what says so - the platform focuses
// the first control inside the dialog, and the cancel button is written
// before the confirming one.
//
// **The scrim is the dialog's own backdrop, dimmed rather than
// washed.** `::backdrop` reaches the page's colour tokens only where an
// engine inherits custom properties into it, and a token that fails to
// resolve leaves `background-color` at its initial value - a modal with
// no scrim at all, on an engine nobody tested. A brightness filter asks
// the platform for no colour, so it darkens the page under both
// lightings and cannot fail quietly.

import { Show, createEffect, createSignal, createUniqueId } from "solid-js";

import { Button } from "./button";

// Open and closed are one property with two displays, so the opening
// and the closing are the same transition read in two directions.
// `display` and `overlay` are discrete properties: without
// `transition-discrete` the box would vanish on the first frame of the
// closing and take its fade with it.
const SHEET =
  "m-auto hidden w-full max-w-measure flex-col gap-base rounded-panel border border-g3 " +
  "bg-g1 p-pane opacity-0 shadow-modal transition-[opacity,display,overlay] " +
  "transition-discrete duration-200 ease-standard open:flex open:opacity-100 " +
  "starting:open:opacity-0 motion-reduce:transition-none " +
  "backdrop:bg-transparent backdrop:backdrop-brightness-50";

export interface DialogProps {
  readonly open: boolean;
  // Already in the person's language: what is about to happen.
  readonly title: string;
  // What it will cost, in the words of the page.
  readonly detail?: string;
  readonly confirmLabel: string;
  readonly cancelLabel: string;
  readonly onConfirm: () => void;
  readonly onCancel: () => void;
  // Whether the confirming answer destroys something.
  readonly destructive?: boolean;
}

export function Dialog(props: DialogProps) {
  const heading = createUniqueId();
  const body = createUniqueId();
  // The element is held as a signal rather than a variable so that the
  // effect below runs again when the ref arrives, whichever order the
  // first render and the first `open` happen in.
  const [sheet, setSheet] = createSignal<HTMLDialogElement>();

  // The caller owns whether the question stands; this only carries that
  // answer to the element. Asking an already-open dialog to open throws,
  // so each call is made only on the edge it belongs to.
  createEffect(() => {
    const node = sheet();
    if (node === undefined) return;
    if (props.open) {
      if (!node.open) node.showModal();
      return;
    }
    if (node.open) node.close();
  });

  return (
    <dialog
      ref={setSheet}
      class={SHEET}
      aria-labelledby={heading}
      aria-describedby={props.detail === undefined ? undefined : body}
      // Escape reaches here as a cancel request. The default would close
      // the element behind the caller's back and leave `open` saying it
      // is still up, so the request is answered by the caller instead.
      onCancel={(event) => {
        event.preventDefault();
        props.onCancel();
      }}
    >
      <h2 id={heading} class="text-heading text-text">
        {props.title}
      </h2>
      <Show when={props.detail}>
        {(detail) => (
          <p id={body} class="text-note text-text-quiet">
            {detail()}
          </p>
        )}
      </Show>
      <div class="flex items-center justify-end gap-snug">
        <Button
          label={props.cancelLabel}
          tone="secondary"
          onPress={() => {
            props.onCancel();
          }}
        />
        <Button
          label={props.confirmLabel}
          tone={props.destructive === true ? "destructive" : "primary"}
          onPress={() => {
            props.onConfirm();
          }}
        />
      </div>
    </dialog>
  );
}
