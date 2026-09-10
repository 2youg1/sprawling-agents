// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The one thing that may float over a page uninvited: a refusal, in
// the three parts the city wrote it in, shown once and dismissed by
// hand. A link that cannot come up shows here too, with the way back.

import { Show } from "solid-js";

import { useSay, useUi } from "../ui";

export function Refusal() {
  const ui = useUi();
  const say = useSay();
  const refused = () => ui.conn.belief.refusal;
  const state = () => ui.conn.state();

  return (
    <>
      <Show when={refused()}>
        {(error) => (
          <div
            role="alert"
            class="fixed bottom-wide left-wide z-10 w-full max-w-measure rounded-panel border border-alert/50 bg-g1 px-pane py-base text-note shadow-composer"
          >
            <div class="flex items-start justify-between gap-base">
              <div class="min-w-0">
                <div class="font-label text-alert">
                  {error().action} <span class="font-mono text-note text-text-faint">{error().code}</span>
                </div>
                <div class="mt-tight break-all font-mono text-text">{error().subject}</div>
                <Show when={error().recovery !== ""}>
                  <div class="mt-tight text-text-quiet">{error().recovery}</div>
                </Show>
                <Show when={error().gate}>
                  {(gate) => (
                    <div class="mt-snug text-text-faint">
                      {gate().rule} — {gate().violation} — {gate().alternative}
                    </div>
                  )}
                </Show>
              </div>
              <button
                type="button"
                class="shrink-0 whitespace-nowrap rounded-control px-snug py-tight text-text-quiet hover:bg-g2"
                onClick={() => {
                  ui.conn.dismissRefusal();
                }}
              >
                {say("dismiss")}
              </button>
            </div>
            <Show when={state().kind === "refused"}>
              <button
                type="button"
                class="mt-snug rounded-control bg-g2 px-base py-tight text-label hover:bg-g3"
                onClick={() => {
                  ui.conn.retry();
                }}
              >
                {say("link_retry")}
              </button>
            </Show>
          </div>
        )}
      </Show>
    </>
  );
}
