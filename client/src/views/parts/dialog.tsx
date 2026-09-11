// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The question asked before something that cannot be taken back. An
// action that can be undone is done straight away and offered back as a
// Notice; only the irreversible one stops and explains first.
//
// The focus goes to the way out rather than to the way through: on a
// question about deleting something, the safe answer is the one already
// under the hand.

import { Show, createUniqueId } from "solid-js";

import { Button } from "./button";

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
  return (
    <Show when={props.open}>
      <div
        class="fixed inset-0 z-20 flex items-center justify-center bg-g0/80 p-pane"
        onKeyDown={(event) => {
          if (event.key === "Escape") {
            event.preventDefault();
            props.onCancel();
          }
        }}
      >
        <div
          role="dialog"
          aria-modal="true"
          aria-labelledby={heading}
          aria-describedby={props.detail === undefined ? undefined : body}
          class="flex w-full max-w-measure flex-col gap-base rounded-panel border border-g3 bg-g1 p-pane shadow-composer transition-[opacity,transform] duration-200 ease-[cubic-bezier(0.2,0,0,1)] motion-reduce:transition-none"
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
            <span
              ref={(span) => {
                requestAnimationFrame(() => {
                  span.querySelector("button")?.focus();
                });
              }}
            >
              <Button
                label={props.cancelLabel}
                tone="secondary"
                onPress={() => {
                  props.onCancel();
                }}
              />
            </span>
            <Button
              label={props.confirmLabel}
              tone={props.destructive === true ? "destructive" : "primary"}
              onPress={() => {
                props.onConfirm();
              }}
            />
          </div>
        </div>
      </div>
    </Show>
  );
}
