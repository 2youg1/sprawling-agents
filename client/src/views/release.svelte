<script lang="ts" module>
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Which release this city is running, and - only if asked - whether a
// newer one exists.
//
// **One row, and the press is the whole trigger.** This is the settings
// page's last group: a button that asks only when a person presses it
// says everything a paragraph about npm, an archive recipe and the note
// that nothing updates itself used to say.
//
// `ask` is called from the handler rather than from the component body:
// the answer is therefore watched by nobody, so neither a reconnect nor
// an event re-asks it, and opening this page costs no request. A city
// that polled a registry would be spending the promise QUICKSTART.md
// opens with on a question nobody asked.
//
// Nothing here updates anything. The command under a `behind` verdict
// is printed for a person to run, because `sprawling install` owns the
// archive path and npm owns its own, and a third party writing over
// either would be a second authority for where this binary lives.

// wording-ok: the update recipe printed for a person to run; a machine
// spelling, identical in both languages (client/Spec.lean §4-10)
const UPDATE_NPM = "bunx sprawling@latest up";
// wording-ok: the project's own release page; a proper noun identical
// in both languages
const RELEASES = "https://github.com/2youg1/sprawling-agents/releases";
</script>

<script lang="ts">
  import { readable } from "svelte/store";
  import type { Readable } from "svelte/store";

  import { QUERIES } from "../core/asking";
  import { fill, say } from "../core/lang";
  import { ui } from "../ui";
  import type { Answer, ReleaseAnswer } from "../wire";
  import Button from "./parts/button.svelte";

  const u = ui();
  const lang = u.lang;

  // The slot the answer will arrive in. `ask` is not called until the
  // press, so the page opens with the slot holding nothing.
  let slot: Readable<Answer | undefined> = $state(readable<Answer | undefined>(undefined));
  let asking = $state(false);

  const answer = $derived.by((): ReleaseAnswer | undefined => {
    const now = $slot;
    return now !== undefined && "release" in now ? now.release : undefined;
  });
  // One derivation per state, because the check and the read have to
  // happen on one value: asking `"stands" in answer` and then reading
  // `answer.stands` are two calls, and the second is not narrowed by
  // the first.
  const refused = $derived(answer !== undefined && "refused" in answer ? answer.refused : undefined);
  const unreleased = $derived(answer !== undefined && "unreleased" in answer ? answer.unreleased : undefined);
  const stands = $derived(answer !== undefined && "stands" in answer ? answer.stands : undefined);

  // A fresh answer ends the wait, whoever asked for it.
  $effect(() => {
    if (answer !== undefined) {
      asking = false;
    }
  });

  function check(): void {
    asking = true;
    // First press opens the slot and sends; every press after it sends
    // again, because "check again" means now rather than what was
    // already answered.
    slot = u.conn.asking.ask(QUERIES.release);
    u.conn.asking.refresh(QUERIES.release);
  }
</script>

<!-- "About this version" stands on the page surface under its group's
heading and rule, rather than lifted onto a card (docs/frontend-method.md §7A-4,
client/Spec.lean §4-50). -->
<section class="flex min-w-0 flex-col gap-snug">
  <div class="flex flex-wrap items-center gap-snug">
    <Button label={say($lang, "release_check")} tone="secondary" loading={asking} onPress={check} />
    <a class="text-note text-accent underline" href={RELEASES} target="_blank" rel="noreferrer">
      {RELEASES}
    </a>
  </div>
  {#if refused !== undefined}
    <p class="text-note text-alert">
      {say($lang, "release_refused")}
      <code class="font-mono text-note text-text-quiet">{refused.refusal.recovery}</code>
    </p>
  {/if}
  {#if unreleased !== undefined}
    <p class="text-note text-text-quiet">
      {say($lang, "release_source")}
      {fill(say($lang, "release_newest"), {
        version: unreleased.newest.version,
        released: unreleased.newest.released,
      })}
    </p>
  {/if}
  {#if stands !== undefined}
    <div class="flex flex-col gap-tight">
      <p class="text-note text-text">
        {fill(say($lang, "release_mine"), {
          version: stands.mine.version,
          released: stands.mine.released,
        })}
      </p>
      {#if stands.verdict === "current"}
        <p class="text-note text-accent">{say($lang, "release_current")}</p>
      {:else if stands.verdict === "ahead"}
        <p class="text-note text-text-quiet">
          {fill(say($lang, "release_ahead"), { version: stands.newest.version })}
        </p>
      {:else if stands.verdict === "behind"}
        <p class="text-note text-alert">
          {fill(say($lang, "release_behind"), {
            version: stands.newest.version,
            released: stands.newest.released,
          })}
        </p>
        <code class="block w-fit border-l-2 border-edge-input pl-base font-mono text-note text-text">
          {UPDATE_NPM}
        </code>
      {/if}
    </div>
  {/if}
</section>
