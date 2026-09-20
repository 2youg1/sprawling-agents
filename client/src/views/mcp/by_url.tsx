// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// `claude mcp add --transport http|sse <name> <url>`, as a form: the
// three transports side by side, the url, and the headers the request
// carries.
//
// Picking `stdio` moves to the command door rather than emptying this
// one, because a stdio server has no url and a person who picked it
// wants the other form; picking `sse` stays here and says what the wire
// cannot spell.

import { Show, createMemo, createSignal } from "solid-js";

import { EMPTY, SPELLED, WHY, encode } from "./draft";
import type { Draft, Intake, Transport } from "./draft";
import { Button } from "../parts/button";
import { Field } from "../parts/field";
import { PairTable } from "./pairs";
import { Segmented, type Choice } from "../parts/segmented";
import { useSay } from "../../ui";

const OVER_A_URL: Draft = { ...EMPTY, transport: "http" };

// The three transports in the order a person meets them, `stdio` last
// because picking it leaves this door rather than filling it in.
const TRANSPORTS: readonly Transport[] = ["http", "sse", "stdio"];

export function ByUrl(props: { readonly intake: Intake; readonly onCommandDoor: () => void }) {
  const say = useSay();
  const [draft, setDraft] = createSignal<Draft>(OVER_A_URL);

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
      setDraft(OVER_A_URL);
    }
  };
  const pick = (transport: Transport) => {
    if (transport === "stdio") {
      props.onCommandDoor();
      return;
    }
    setDraft({ ...draft(), transport });
  };
  const transports = createMemo<readonly Choice<Transport>[]>(() =>
    TRANSPORTS.map((transport) => ({ value: transport, label: say(SPELLED[transport]) })),
  );

  return (
    <div class="flex flex-col gap-base">
      <div class="flex flex-wrap items-center gap-snug">
        <span class="text-note text-text-quiet">{say("mcp_transport")}</span>
        <Segmented
          label={say("mcp_transport")}
          options={transports()}
          held={draft().transport}
          onPick={pick}
        />
      </div>
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
          label={say("mcp_url")}
          mono
          value={draft().url}
          onInput={(url) => {
            setDraft({ ...draft(), url });
          }}
        />
      </div>
      <PairTable
        caption={say("mcp_header")}
        note={say("mcp_header_help")}
        rows={draft().headers}
        onChange={(headers) => {
          setDraft({ ...draft(), headers });
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
