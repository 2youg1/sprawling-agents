// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { Show } from "solid-js";

import { fill, say, type Key } from "../../core/lang";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import type { AgentOffer, AxError } from "../../wire";
import { DiagnosticView } from "../diagnostic_view";
import Button from "../parts/button.solid";
import { PINNED, SOURCE, loginKey, programOf } from "./agents";

interface ConsentProps {
  readonly offer: AgentOffer;
  readonly named: boolean;
  readonly seats: boolean;
  readonly adding: boolean;
  readonly unknown?: boolean;
  readonly error?: AxError | undefined;
  readonly onAdd: (offer: AgentOffer, here: boolean) => void;
  readonly onCheck?: (() => void) | undefined;
}

export default function Consent(props: ConsentProps) {
  const lang = storeValue(ui().lang);
  const word = (key: Key) => say(lang(), key);
  const sources = () => word(SOURCE[props.offer.source]);
  const found = () =>
    props.offer.source === "detected"
      ? ` · ${fill(word("acp_on_path"), { program: programOf(props.offer) })}`
      : "";
  const version = () => {
    const pinned = PINNED[props.offer.pinned];
    const offered = props.offer.version ?? word("acp_version_unknown");
    return pinned === undefined ? offered : `${offered} (${word(pinned)})`;
  };
  function Fact(entry: { readonly label: Key; readonly value: string; readonly mono?: boolean }) {
    return (
      <div class="flex gap-tight">
        <dt class="text-text-faint">{word(entry.label)}</dt>
        <dd class={["text-text-quiet", entry.mono && "font-mono"]}>{entry.value}</dd>
      </div>
    );
  }

  return (
    <div
      class="flex min-w-0 flex-col gap-snug rounded-card bg-raised px-base py-base"
      role="group"
      aria-label={props.offer.name}
    >
      <Show when={props.named}>
        <div class="flex flex-wrap items-baseline justify-between gap-x-base gap-y-tight">
          <span class="text-label font-label text-text">{props.offer.name}</span>
          <span class="text-note text-text-faint">
            {sources()}
            {found()}
          </span>
        </div>
      </Show>
      <code class="block min-w-0 break-all rounded-control bg-page px-snug py-tight font-mono text-note text-text">
        {props.offer.launch_preview}
      </code>
      <dl class="flex flex-wrap gap-x-wide gap-y-tight text-note">
        <Show when={!props.named}>
          <Fact label="acp_source" value={sources()} />
        </Show>
        <Fact label="acp_version" value={version()} />
        <Fact label="acp_licence" value={props.offer.licence ?? word("acp_licence_unstated")} />
        <Fact
          label="acp_env"
          mono
          value={
            props.offer.env_names.length === 0
              ? word("acp_env_none")
              : props.offer.env_names.join(" ")
          }
        />
        <Fact label="acp_login" value={word(loginKey(props.offer.login))} />
      </dl>
      <p class="text-note text-text-quiet">{word("acp_consent")}</p>
      <Show
        when={!props.unknown}
        fallback={
          <div class="flex flex-wrap items-center gap-snug pt-tight">
            <span role="status" class="text-note text-text-quiet">
              {word("saving_unverified")}
            </span>
            <Button
              label={word("saving_check")}
              result="unknown"
              onPress={() => props.onCheck?.()}
            />
          </div>
        }
      >
        <div class="flex flex-wrap gap-snug pt-tight">
          <Show when={props.seats}>
            <Button
              label={word("acp_add_and_use")}
              glyph="door"
              tone="primary"
              loading={props.adding}
              onPress={() => props.onAdd(props.offer, true)}
            />
          </Show>
          <Button
            label={word(props.adding ? "acp_adding" : "acp_add_only")}
            glyph="add"
            tone={props.seats ? "quiet" : "primary"}
            loading={props.adding}
            onPress={() => props.onAdd(props.offer, false)}
          />
        </div>
      </Show>
      <Show when={props.error}>{(error) => <DiagnosticView error={error()} lang={lang()} />}</Show>
    </div>
  );
}
