// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { Show } from "solid-js";
import { storeValue } from "../../core/solid_store";
import { clock } from "../../core/time";
import { ui } from "../../ui";
import type { RunId } from "../../wire";
import ForkButton from "./fork_button_view";
import type { ForkEntry, ForkPlan } from "./forking";
import { withoutSources } from "./source_context";

export interface PersonProps {
  readonly text: string;
  readonly label: string;
  readonly labelHref?: string | undefined;
  readonly at: number | undefined;
  readonly entry: ForkEntry | null;
  readonly run: RunId;
  readonly onFork?: ((plan: ForkPlan) => void) | undefined;
  readonly onHover: (entry: ForkEntry | null) => void;
}

export default function Person(props: PersonProps) {
  const lang = storeValue(ui().lang);
  return (
    <div class="my-pane">
      <div class="group flex flex-col items-end">
        <div class="max-w-[83%] rounded-panel bg-speech px-pane py-base font-read text-body leading-read whitespace-pre-wrap wrap-anywhere">
          {withoutSources(props.text)}
        </div>
        <div class="mt-tight flex items-center gap-snug text-note text-text-faint">
          <Show when={props.onFork === undefined ? null : props.entry}>
            {(entry) => (
              <ForkButton
                entry={entry()}
                run={props.run}
                onFork={props.onFork}
                onHover={props.onHover}
              />
            )}
          </Show>
          <span>
            <Show when={props.labelHref} fallback={props.label}>
              {(link) => (
                <a href={link()} class="text-text-quiet hover:text-text">
                  {props.label}
                </a>
              )}
            </Show>
            {props.at === undefined ? "" : ` · ${clock(lang(), props.at)}`}
          </span>
        </div>
      </div>
    </div>
  );
}
