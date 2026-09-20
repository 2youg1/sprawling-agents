// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The one screen on which a provider is attached: what this city
// already reaches, the two doors in, and the form behind the door that
// is open.
//
// The welcome walk and the settings page both show it. They used to
// hold a copy each - two signals named `door`, two rows of pills, two
// orders of the same three parts - and the copies had already drifted:
// only one of them drew the model table underneath. A door added to
// this screen now arrives on both pages, or on neither.
//
// Which endpoints exist is asked here rather than passed in, because
// `core/asking.ts` matches an answer to a question by content: a page
// that already asked the same question is answered once and both
// readers see it.

import { Show, createMemo, createSignal } from "solid-js";

import { QUERIES } from "../../core/asking";
import type { EndpointsAnswer } from "../../wire";
import { useSay, useUi } from "../../ui";
import { Segmented } from "../parts/segmented";
import { AttachForm, EndpointList, LoginForm } from "../setup/providers";

// The two ways a provider is reached: a key somebody pastes, or a
// subscription somebody logs in to.
type Door = "key" | "login";

export function ProviderDoor() {
  const ui = useUi();
  const say = useSay();
  const [door, setDoor] = createSignal<Door>("key");
  const endpoints = ui.conn.asking.ask(QUERIES.endpoints);
  const answer = createMemo<EndpointsAnswer | undefined>(() => {
    const held = endpoints();
    return held !== undefined && "endpoints" in held ? held.endpoints : undefined;
  });

  return (
    <div class="flex flex-col gap-base">
      <Show when={answer()}>{(held) => <EndpointList answer={held()} />}</Show>
      <Segmented<Door>
        label={say("setup_providers")}
        options={[
          { value: "key", label: say("setup_attach") },
          { value: "login", label: say("setup_login") },
        ]}
        held={door()}
        onPick={setDoor}
      />
      <Show when={door() === "key"} fallback={<LoginForm />}>
        <AttachForm />
      </Show>
    </div>
  );
}
