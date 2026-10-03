<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // The first section of the mailbox: everything that stops until the
  // person acts (client/Spec.lean §4-49). A door waiting on the person's own
  // hand, a design question a resident filed and a change a run
  // proposes to a document are decide cards (7C); a failure that stops
  // the work is its notice, with the recovery the city named. Newest
  // first within each kind, the doors and failures before the questions
  // and the proposals, because those wait on a judgement while a door
  // waits on a hand already at the keyboard (4-55). Every card says
  // under its head which event or refusal put it here and which rule
  // keeps it here (`pending.ts`).
  import { fill, say } from "../../core/lang";
  import { recoveryFor } from "../../core/recovering";
  import { ago } from "../../core/time";
  import { ui } from "../../ui";
  import { recover, recoveryLabel, recoveryWhy } from "../notice_recovery";
  import Button from "../parts/button.svelte";
  import Decide from "../parts/decide.svelte";
  import Notice from "../parts/notice.svelte";
  import { recoveryWords } from "../parts/notice_title";
  import { WaitingCards } from "../talk/waiting.svelte";
  import DecidingDocument from "./deciding_document.svelte";
  import { documentsOf, openCards } from "./deciding_proposals";
  import { stoppedOf, WHY } from "./pending";
  import Section from "./section.svelte";
  import { shown, sweep } from "./swept.svelte";

  interface Props {
    // Puts the mailbox away once the person follows a card elsewhere.
    readonly onLeave: () => void;
  }

  const { onLeave }: Props = $props();

  const u = ui();
  const { lang } = u;
  const belief = u.conn.belief;
  const approvals = u.approvals;

  const stopped = $derived(stoppedOf($belief.notices.filter(shown)));
  const asks = $derived(stopped.asks);
  const failures = $derived(stopped.failures);
  const questions = $derived($approvals ?? []);
  const cards = openCards(u.conn.asking);
  const documents = $derived(documentsOf($cards));
  const count = $derived(asks.length + failures.length + questions.length + $cards.length);
</script>

<Section title="mailbox_deciding" empty="mailbox_deciding_none" {count}>
  {#each asks as notice (notice.key)}
    <div class="my-base">
      <Decide
        kind="ask"
        asker={say($lang, "mailbox_ask_title")}
        at={ago($lang, notice.at, u.now())}
        choices={[
          {
            answer: "no",
            label: say($lang, "dismiss"),
            onPress: () => {
              sweep([notice]);
            },
          },
        ]}
      >
        {#snippet body()}
          <p class="text-body">{notice.error.action}</p>
          <p class="font-mono text-note wrap-anywhere text-text-quiet">{notice.error.subject}</p>
          <p class="mt-snug text-note text-text">{recoveryWords($lang, notice.error.code, notice.error.recovery)}</p>
          <p class="mt-snug text-note text-text-quiet">{say($lang, WHY.ask)}</p>
        {/snippet}
      </Decide>
    </div>
  {/each}
  {#each failures as notice (notice.key)}
    <div class="flex min-w-0 items-start gap-snug rounded-card focus-visible:wash" data-entry tabindex="-1">
      <div class="min-w-0 flex-1">
        <Notice
          seat="drawer"
          weight="alert"
          action={notice.error.action}
          code={notice.error.code}
          subject={notice.error.subject}
          recovery={notice.error.recovery}
          at={ago($lang, notice.at, u.now())}
          count={notice.count}
        >
          {#snippet actions()}
            {#each recoveryFor(notice.error) as recovery (recoveryLabel(recovery, $lang))}
              {@const why = recoveryWhy(u, recovery, notice)}
              <Button
                tone="quiet"
                label={recoveryLabel(recovery, $lang)}
                {...why === undefined ? {} : { why: say($lang, why) }}
                onPress={() => {
                  recover(u, recovery, notice);
                }}
              />
            {/each}
            <Button
              tone="quiet"
              label={say($lang, "dismiss")}
              onPress={() => {
                sweep([notice]);
              }}
            />
          {/snippet}
        </Notice>
        <p class="mt-tight text-note text-text-quiet">{fill(say($lang, WHY.failure), { code: notice.error.code })}</p>
      </div>
      <kbd class="entry-n mt-snug" aria-hidden="true"></kbd>
    </div>
  {/each}
  {#if questions.length > 0}
    <p class="mt-base text-note text-text-quiet">{say($lang, WHY.question)}</p>
  {/if}
  <WaitingCards items={questions} />
  {#each documents as offered (offered.doc)}
    <DecidingDocument doc={offered.doc} at={offered.at} {onLeave} />
  {/each}
</Section>
