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
// Nothing here updates anything. The answer is drawn by
// `release/answer.svelte`: both registries the city asked, and the
// city's `update.command` for the channel that installed this binary,
// printed for a User to run, because `sprawling install` owns the
// archive path and npm and cargo own their own, and a third party
// writing over either would be a second authority for where this binary
// lives.

// wording-ok: the project's own release page; a proper noun identical
// in both languages
const RELEASES = "https://github.com/2youg1/sprawling-agents/releases";
</script>

<script lang="ts">
  import { readable } from "svelte/store";
  import type { Readable } from "svelte/store";

  import { QUERIES } from "../core/asking";
  import { say } from "../core/lang";
  import { ui } from "../ui";
  import type { Answer, ReleaseAnswer } from "../wire";
  import Button from "./parts/button.svelte";
  import ReleaseAnswerView from "./release/answer.svelte";

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
    <Button label={say($lang, "release_check_registries")} tone="secondary" loading={asking} onPress={check} />
    <a class="text-note text-accent underline" href={RELEASES} target="_blank" rel="noreferrer">
      {RELEASES}
    </a>
  </div>
  {#if answer !== undefined}
    <ReleaseAnswerView {answer} />
  {/if}
</section>
