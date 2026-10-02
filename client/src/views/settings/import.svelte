<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // Taking a user ID from the GitHub CLI (refrain roadmap §3-13). The
  // read happens on the machine the city runs on, not the one this
  // browser runs on, so the control says so before it is pressed. It is
  // asked only when pressed (`githubLoginQuery`), it shows the host and
  // the login it found as a candidate, and taking the candidate only
  // fills the field: the card's own save makes it the setting. Every
  // reading but a login leaves the field as the person left it and says
  // why, so typing the ID by hand is always the way on.

  import type { Readable } from "svelte/store";

  import { githubLoginQuery } from "../../core/asking";
  import { fill, say } from "../../core/lang";
  import { ui } from "../../ui";
  import type { Answer, GithubReading } from "../../wire";
  import Button from "../parts/button.svelte";

  interface Props {
    readonly onTake: (login: string, host: string) => void;
  }

  const { onTake }: Props = $props();
  const u = ui();
  const { lang } = u;

  let host = $state("");
  let asked = $state.raw<Readable<Answer | undefined> | null>(null);

  const answer = $derived($asked);
  const read = $derived(answer !== undefined && answer !== null && "github_login" in answer ? answer.github_login : null);

  function ask(): void {
    const query = githubLoginQuery(host.trim() === "" ? null : host.trim());
    if (asked !== null) u.conn.asking.refresh(query);
    asked = u.conn.asking.ask(query);
  }

  function why(reading: GithubReading, at: string): string {
    if (reading === "no_cli") return say($lang, "you_import_no_cli");
    if (reading === "not_logged_in") return fill(say($lang, "you_import_signed_out"), { host: at });
    if (reading === "not_a_host") return fill(say($lang, "you_import_not_a_host"), { host: at });
    if ("failed" in reading) return fill(say($lang, "you_import_failed"), { exit: String(reading.failed.exit ?? "—") });
    if ("stuck" in reading) return fill(say($lang, "you_import_stuck"), { why: reading.stuck.why });
    return "";
  }
</script>

<div class="flex flex-col gap-tight border-t border-edge pt-snug">
  <p class="text-note text-text-faint">{say($lang, "you_import_note")}</p>
  <div class="flex flex-wrap items-center gap-snug">
    <input
      class="h-control min-w-0 flex-1 rounded-control border border-edge-input bg-page px-base font-mono text-note text-text"
      aria-label={say($lang, "you_import_host")}
      placeholder={say($lang, "you_import_host_default")}
      bind:value={host}
    />
    <Button label={say($lang, "you_import")} tone="secondary" loading={asked !== null && answer === undefined} onPress={ask} />
  </div>
  {#if read !== null}
    <div class="flex items-center gap-snug text-note" role="status">
      {#if typeof read.reading === "object" && "found" in read.reading}
        {@const login = read.reading.found.login}
        <span class="min-w-0 flex-1 truncate font-mono text-text">{`${read.host} · ${login}`}</span>
        <Button label={say($lang, "you_import_take")} tone="secondary" onPress={() => {
            onTake(login, read.host);
          }} />
      {:else}
        <span class="text-text-quiet">{why(read.reading, read.host)}</span>
      {/if}
    </div>
  {/if}
</div>
