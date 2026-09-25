<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // The one screen on which a provider is attached: what this city
  // already reaches, the two doors in, and the form behind the door
  // that is open.
  //
  // The welcome walk and the settings page both show it. They used to
  // hold a copy each - two signals named `door`, two rows of pills,
  // two orders of the same three parts - and the copies had already
  // drifted: only one of them drew the model table underneath. A door
  // added to this screen now arrives on both pages, or on neither.
  //
  // Which endpoints exist is asked here rather than passed in, because
  // `core/asking.ts` matches an answer to a question by content: a
  // page that already asked the same question is answered once and
  // both readers see it.
  //
  // **A subscription login is the other door**: begin, approve in a
  // browser, bring back the code. Its two controls live here rather
  // than in a file of their own because this is the only screen that
  // opens that door.

  export interface ProviderDoorProps {
    // A registration went through; the settings page refreshes its
    // lists on it. Absent where there is nothing beside the door to
    // bring up to date.
    readonly onAttached?: () => void;
  }

  // The two ways a provider is reached: a key somebody pastes, or a
  // subscription somebody logs in to.
  type Door = "key" | "login";

  // The subscription door signs in to one provider, and the login is
  // that provider's own.
  // wording-ok: a provider id, which no language translates
  const LOGIN_PROVIDER = "anthropic";
</script>

<script lang="ts">
  import { QUERIES } from "../../core/asking";
  import { loginBegin, loginCode } from "../../core/commands";
  import { say } from "../../core/lang";
  import type { EndpointsAnswer } from "../../wire";
  import { ui } from "../../ui";
  import Button from "../parts/button.svelte";
  import Field from "../parts/field.svelte";
  import Segmented from "../parts/segmented.svelte";
  import { AttachForm, EndpointList } from "../setup/providers";

  const { onAttached }: ProviderDoorProps = $props();
  const u = ui();
  const lang = u.lang;

  let door = $state<Door>("key");
  let code = $state("");

  const endpoints = u.conn.asking.ask(QUERIES.endpoints);
  const answer = $derived.by((): EndpointsAnswer | undefined => {
    const held = $endpoints;
    return held !== undefined && "endpoints" in held ? held.endpoints : undefined;
  });

  // The approval page the login opened, read out of the ledger: the
  // city writes `login_started` with the URL the browser must be sent
  // to, and the last one for this provider is the one still open.
  const history = u.conn.asking.ask({ history: { before: null, limit: 40 } });
  const url = $derived.by((): string | null => {
    const held = $history;
    if (held === undefined || !("history" in held)) return null;
    for (let i = held.history.records.length - 1; i >= 0; i -= 1) {
      const record = held.history.records[i];
      if (record?.kind !== "login_started" || record.data.provider !== LOGIN_PROVIDER) continue;
      const href = record.data.auth_url;
      return typeof href === "string" ? href : null;
    }
    return null;
  });
</script>

<div class="flex flex-col gap-base">
  {#if answer !== undefined}
    <EndpointList {answer} />
  {/if}
  <Segmented
    label={say($lang, "setup_providers")}
    options={[
      { value: "key", label: say($lang, "setup_attach") },
      { value: "login", label: say($lang, "setup_login") },
    ]}
    held={door}
    onPick={(picked: Door) => {
      door = picked;
    }}
  />
  {#if door === "key"}
    <AttachForm {...(onAttached === undefined ? {} : { onAttached })} />
  {:else}
    <div class="flex flex-col gap-base">
      <div class="flex items-center gap-snug">
        <Button
          label={`${say($lang, "setup_login_begin")} · ${LOGIN_PROVIDER}`}
          tone="secondary"
          onPress={() => {
            u.send(loginBegin(LOGIN_PROVIDER));
          }}
        />
        {#if url !== null}
          <a
            href={url}
            target="_blank"
            rel="noopener noreferrer"
            class="text-note text-accent underline"
          >
            {say($lang, "setup_login_open")}
          </a>
        {/if}
      </div>
      {#if url !== null}
        <div class="flex items-end gap-snug">
          <Field
            label={say($lang, "setup_login_code")}
            mono
            value={code}
            onInput={(value: string) => {
              code = value;
            }}
          />
          <Button
            label={say($lang, "setup_login_finish")}
            tone="primary"
            {...(code.trim() === "" ? { why: say($lang, "setup_form_incomplete") } : {})}
            onPress={() => {
              const typed = code.trim();
              if (typed === "") return;
              if (u.send(loginCode(LOGIN_PROVIDER, typed))) {
                code = "";
                onAttached?.();
              }
            }}
          />
        </div>
      {/if}
    </div>
  {/if}
</div>
