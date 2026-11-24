// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo, Match, Show, Switch } from "solid-js";
import { readAnswer } from "../../core/answered";
import { QUERIES } from "../../core/asking";
import { say } from "../../core/lang";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import Unanswered from "../parts/unanswered.solid";
import EndpointList from "../setup/providers/endpoint_list.solid";
import AttachForm from "../setup/providers/form_view";

export interface ProviderDoorProps {
  readonly onAttached?: (() => void) | undefined;
  readonly openModels?: boolean | undefined;
  readonly stepped?: boolean | undefined;
}

export default function ProviderDoor(props: ProviderDoorProps) {
  const u = ui();
  const lang = storeValue(u.lang);
  const endpoints = storeValue(u.conn.asking.ask(QUERIES.endpoints));
  const read = createMemo(() =>
    readAnswer(endpoints(), (held) => ("endpoints" in held ? held.endpoints : undefined)),
  );
  const answer = () => {
    const held = read();
    return held.kind === "held" ? held.value : undefined;
  };
  const lost = () => {
    const held = read();
    return held.kind === "unavailable" ? held : undefined;
  };
  function Listing() {
    return (
      <Switch>
        <Match when={answer()}>
          {(held) => <EndpointList answer={held()} openModels={props.openModels} />}
        </Match>
        <Match when={lost()}>
          {(failure) => <Unanswered query={failure().query} asked={QUERIES.endpoints} />}
        </Match>
      </Switch>
    );
  }
  const attached = () => {
    const action = props.onAttached;
    return action === undefined ? {} : { onAttached: action };
  };
  return (
    <Show
      when={props.stepped}
      fallback={
        <div class="flex flex-col gap-base">
          <Listing />
          <AttachForm {...attached()} />
        </div>
      }
    >
      <div class="flex flex-col gap-wide">
        <section class="flex min-w-0 flex-col gap-snug">
          <header class="flex min-w-0 flex-col gap-tight">
            <h3 class="text-label font-label text-text">{say(lang(), "setup_step_connected")}</h3>
            <p class="text-note text-text-faint">{say(lang(), "setup_step_connected_hint")}</p>
          </header>
          <Listing />
        </section>
        <section class="flex min-w-0 max-w-talk flex-col gap-snug border-t border-edge pt-wide">
          <header class="flex min-w-0 flex-col gap-tight">
            <h3 class="text-label font-label text-text">{say(lang(), "setup_step_attach")}</h3>
            <p class="text-note text-text-faint">{say(lang(), "setup_step_attach_hint")}</p>
          </header>
          <AttachForm {...attached()} />
        </section>
      </div>
    </Show>
  );
}
