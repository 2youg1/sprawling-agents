// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// `claude mcp add <name> -- <cmd> [args]`, as a form: a name, the shell
// line that starts the program, and the environment that line is given.
//
// The environment table is here because a stdio server that needs a
// token is the ordinary case; today it can be filled in and not sent,
// and the sentence under it says which half of that is the city's fault.

import { Show, createSignal } from "solid-js";

import { EMPTY, WHY, encode } from "./draft";
import type { Draft, Intake } from "./draft";
import { Button } from "../parts/button";
import { Field } from "../parts/field";
import { PairTable } from "./pairs";
import { useSay } from "../../ui";

export function ByCommand(props: { readonly intake: Intake }) {
  const say = useSay();
  const [draft, setDraft] = createSignal<Draft>(EMPTY);

  const encoded = () => encode(draft(), props.intake.taken());
  const reason = () => {
    const scope = props.intake.why();
    if (scope !== null) return scope;
    const out = encoded();
    return out.kind === "blocked" ? say(WHY[out.blocker]) : null;
  };
  const send = () => {
    const out = encoded();
    if (out.kind === "ready" && props.intake.offer(out.server)) {
      setDraft(EMPTY);
    }
  };

  return (
    <div class="flex flex-col gap-base">
      <div class="grid gap-base md:grid-cols-[1fr_2fr]">
        <Field
          label={say("mcp_label")}
          help={say("mcp_label_help")}
          mono
          value={draft().label}
          onInput={(label) => {
            setDraft({ ...draft(), label });
          }}
        />
        <Field
          label={say("mcp_command")}
          help={say("mcp_command_help")}
          mono
          value={draft().command}
          onInput={(command) => {
            setDraft({ ...draft(), command });
          }}
        />
      </div>
      <PairTable
        caption={say("mcp_env")}
        note={say("mcp_env_help")}
        rows={draft().env}
        onChange={(env) => {
          setDraft({ ...draft(), env });
        }}
      />
      <Show
        when={reason()}
        fallback={<Button label={say("mcp_add")} tone="primary" onPress={send} />}
      >
        {(why) => (
          <div class="flex min-w-0 items-center gap-base">
            <Button label={say("mcp_add")} why={why()} />
            <span class="min-w-0 text-note text-text-faint">{why()}</span>
          </div>
        )}
      </Show>
    </div>
  );
}
