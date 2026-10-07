<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // The privacy group (`#/setup/privacy`, client/Spec.lean §7L): the
  // Windows settings of the machine running this city that decide what
  // data Windows and apps may collect, upload or read.
  //
  // The page informs and never nudges: nothing is preselected, there is
  // no "apply all", every entry states its cost, and an entry changes only
  // after its own press and a confirmation that states the change, what
  // a person could overlook and how it is undone
  // (client/spec/Views/Privacy.lean). The only bulk action restores what
  // this app itself changed.
  //
  // The page asks `Query::Privacy` when it opens, after each of its own
  // operations, and when the person asks it to read again; it does not
  // poll, because one answer reads every control on the host.

  import { QUERIES } from "../../../core/asking";
  import { privacyOperation } from "../../../core/commands";
  import { fill, say } from "../../../core/lang";
  import { ui } from "../../../ui";
  import type { PrivacyAnswer, PrivacyControl } from "../../../wire";
  import Button from "../../parts/button.svelte";
  import Dialog from "../../parts/dialog.svelte";
  import Skeleton from "../../parts/skeleton.svelte";
  import type { Action } from "./entry";
  import Entry from "./entry.svelte";
  import { confirmOf, entryId, hostFacts, sectionWord, sectionsOf, standingOf, type EntryModel } from "./page";
  import { privacyPage, type Source } from "./state.svelte";
  import { controlWord, originalReason, PAGE_FACTS, reasonWord } from "./words";

  interface Props {
    // A fixed answer in place of the city's, for the gallery; operations
    // are then minted and never sent.
    readonly fixture?: PrivacyAnswer | undefined;
  }

  const { fixture }: Props = $props();
  const u = ui();
  const lang = u.lang;
  const uid = $props.id();

  let asked = $state.raw<PrivacyAnswer | undefined>(undefined);
  $effect(() => {
    if (fixture !== undefined) return;
    return u.conn.asking.ask(QUERIES.privacy).subscribe((answer) => {
      asked = answer !== undefined && "privacy" in answer ? answer.privacy : undefined;
    });
  });

  const source: Source = {
    answer: () => fixture ?? asked,
    send: (action) => {
      const { command, idem } = privacyOperation(action);
      return { idem, sent: fixture === undefined ? u.send(command) : true };
    },
    refresh: () => {
      if (fixture === undefined) u.conn.asking.refresh(QUERIES.privacy);
    },
  };
  const page = privacyPage(source);

  const answer = $derived(source.answer());
  const sections = $derived(answer === undefined ? [] : sectionsOf(answer));
  const models = $derived(sections.flatMap((section) => section.entries));
  const standing = $derived(answer === undefined ? "open" : standingOf(answer.history));
  const restorable = $derived(models.filter((model) => model.offers.some((offer) => offer.action === "restore")));
  const confirming = $derived(models.find((model) => page.heldOf(model.row.control).stage.kind === "confirming"));
  const confirmLook = $derived.by(() => {
    if (confirming === undefined) return undefined;
    const stage = page.heldOf(confirming.row.control).stage;
    return stage.kind === "confirming" ? confirmOf($lang, confirming, stage.action) : undefined;
  });
  const [first, ...rest] = PAGE_FACTS;

  let restoringAll = $state(false);

  // A line not written points at the entries that come closest.
  function reach(control: PrivacyControl): void {
    const target = document.getElementById(entryId(uid, control));
    target?.scrollIntoView({ block: "start" });
    target?.focus();
  }

  function settle(model: EntryModel): void {
    page.confirm(model);
  }
</script>

<div class="flex min-w-0 flex-col gap-wide">
  <section aria-label={say($lang, "privacy_host_title")} class="flex max-w-talk flex-col gap-snug">
    {#if first !== undefined}
      <p class="text-body text-text">{say($lang, first)}</p>
    {/if}
    {#if answer !== undefined}
      {@const facts = hostFacts($lang, answer.host)}
      <div class="flex flex-col gap-tight rounded-card bg-raised px-base py-snug">
        <span class="text-label font-label text-text">{say($lang, "privacy_host_title")}</span>
        {#if typeof facts === "string"}
          <p class="text-note text-text-quiet">{facts}</p>
        {:else}
          <dl class="grid grid-cols-[auto_minmax(0,1fr)] gap-x-base gap-y-tight text-note">
            {#each facts as fact (fact.label)}
              <dt class="text-text-quiet">{fact.label}</dt>
              <dd class={["min-w-0 text-text [overflow-wrap:anywhere]", fact.figure && "font-mono"]}>{fact.value}</dd>
            {/each}
          </dl>
        {/if}
      </div>
    {/if}
    <ul class="flex list-disc flex-col gap-tight pl-base text-note text-text-quiet">
      {#each rest as fact (fact)}
        <li>{say($lang, fact)}</li>
      {/each}
    </ul>
    {#if standing !== "open"}
      <p class="text-note text-alert">
        {say($lang, standing === "withheld" ? "privacy_history_withheld" : standing === "unreadable" ? "privacy_history_unreadable" : "privacy_unresolved")}
      </p>
    {/if}
    <div class="flex flex-wrap items-center gap-snug">
      <Button label={say($lang, "privacy_refresh")} tone="quiet" onPress={source.refresh} />
      {#if restorable.length > 0}
        <Button label={say($lang, "privacy_restore_all")} tone="secondary" onPress={() => (restoringAll = true)} />
      {/if}
    </div>
  </section>

  {#if answer === undefined}
    <Skeleton label={say($lang, "privacy_reading")} rows={4} />
  {:else}
    {#each sections as section (section.category)}
      <section aria-labelledby="{uid}-section-{section.category}" class="flex min-w-0 flex-col gap-base">
        <h3 id="{uid}-section-{section.category}" class="text-heading font-heading text-text">
          {say($lang, sectionWord(section.category))}
        </h3>
        <div class="grid grid-fit items-start gap-base">
          {#each section.entries as model (model.row.control)}
            <Entry {model} root={uid} held={page.heldOf(model.row.control)} onPress={(action: Action) => { page.press(model, action); }} />
          {/each}
        </div>
      </section>
    {/each}

    {#if answer.not_written.length > 0}
      <section aria-labelledby="{uid}-not-written" class="flex min-w-0 flex-col gap-base">
        <h3 id="{uid}-not-written" class="text-heading font-heading text-text">{say($lang, "privacy_not_written_title")}</h3>
        <p class="text-note text-text-quiet">{say($lang, "privacy_not_written_hint")}</p>
        <div class="grid grid-fit items-start gap-base">
          {#each answer.not_written as entry (entry.line.item)}
            {@const why = originalReason(entry.line.item)}
            <div class="flex min-w-0 flex-col gap-tight rounded-card bg-raised px-base py-snug text-note">
              <span class="font-mono text-text [overflow-wrap:anywhere]">{entry.line.text}</span>
              <span class="text-label font-label text-text-quiet">{say($lang, reasonWord(entry.reason))}</span>
              {#if why !== null}
                <p class="text-text">{say($lang, why)}</p>
              {/if}
              <div class="flex flex-wrap items-center gap-snug">
                {#each entry.alternatives as control (control)}
                  <Button
                    label={fill(say($lang, "privacy_see"), { title: say($lang, controlWord(control, "title")) })}
                    tone="quiet"
                    onPress={() => {
                      reach(control);
                    }}
                  />
                {:else}
                  <span class="text-text-faint">{say($lang, "privacy_none_close")}</span>
                {/each}
              </div>
            </div>
          {/each}
        </div>
      </section>
    {/if}
  {/if}
</div>

<Dialog
  open={confirmLook !== undefined}
  title={confirmLook?.title ?? ""}
  confirmLabel={confirmLook?.confirmLabel ?? ""}
  cancelLabel={say($lang, "privacy_cancel")}
  onConfirm={() => {
    if (confirming !== undefined) settle(confirming);
  }}
  onCancel={() => {
    if (confirming !== undefined) page.cancel(confirming);
  }}
>
  {#if confirmLook !== undefined}
    <dl class="grid grid-cols-[auto_minmax(0,1fr)] gap-x-base gap-y-tight text-note">
      {#each confirmLook.facts as fact (fact.label)}
        <dt class="text-text-quiet">{fact.label}</dt>
        <dd class={["min-w-0 text-text [overflow-wrap:anywhere]", fact.figure && "font-mono"]}>{fact.value}</dd>
      {/each}
    </dl>
    {#each confirmLook.notes as note (note.label)}
      <div class="flex flex-col text-note">
        <span class="text-text-quiet">{note.label}</span>
        <p class="text-text">{note.text}</p>
      </div>
    {/each}
  {/if}
</Dialog>

<Dialog
  open={restoringAll}
  title={fill(say($lang, "privacy_restore_all_title"), { count: String(restorable.length) })}
  detail={say($lang, "privacy_restore_all_detail")}
  confirmLabel={say($lang, "privacy_restore")}
  cancelLabel={say($lang, "privacy_cancel")}
  onConfirm={() => {
    restoringAll = false;
    page.restoreAll(restorable);
  }}
  onCancel={() => (restoringAll = false)}
/>
