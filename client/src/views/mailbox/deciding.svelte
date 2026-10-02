<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // The first section of the mailbox: everything that stops until the
  // person acts (client-SPEC 4-49). A door waiting on the person's own
  // hand and a design question a resident filed are decide cards
  // (7C); a failure that stops the work is its notice, with the
  // recovery the city named. Newest first within each kind, the doors
  // and failures before the questions, because a question waits on a
  // judgement while a door waits on a hand already at the keyboard.
  import { urgencyOf } from "../../core/deferral";
  import { say } from "../../core/lang";
  import { recoveryFor } from "../../core/recovering";
  import { ago } from "../../core/time";
  import { ui } from "../../ui";
  import { recover, recoveryLabel, recoveryWhy } from "../notice_recovery";
  import Button from "../parts/button.svelte";
  import Decide from "../parts/decide.svelte";
  import Empty from "../parts/empty.svelte";
  import Notice from "../parts/notice.svelte";
  import { recoveryWords } from "../parts/notice_title";
  import { WaitingCards } from "../talk/waiting.svelte";
  import Section from "./section.svelte";
  import { shown, sweep } from "./swept.svelte";

  const u = ui();
  const { lang } = u;
  const belief = u.conn.belief;
  const approvals = u.approvals;

  const stopped = $derived(
    [...$belief.notices].reverse().filter((notice) => urgencyOf(notice.error) === "needs_you" && shown(notice)),
  );
  const asks = $derived(stopped.filter((notice) => notice.error.code === "E_APPROVAL_PENDING"));
  const failures = $derived(stopped.filter((notice) => notice.error.code !== "E_APPROVAL_PENDING"));
  const questions = $derived($approvals ?? []);
  const count = $derived(stopped.length + questions.length);
</script>

<Section title="mailbox_deciding" {count}>
  {#if count === 0}
    <Empty missing="mailbox_deciding_none" seat="inset" />
  {/if}
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
      </div>
      <kbd class="entry-n mt-snug" aria-hidden="true"></kbd>
    </div>
  {/each}
  <WaitingCards items={questions} />
</Section>
