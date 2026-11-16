<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  import { QUERIES } from "../../core/asking";
  import { putIdentity } from "../../core/commands";
  import { say } from "../../core/lang";
  import { toFragment } from "../../core/route";
  import { ui } from "../../ui";
  import type { IdentityCard, StatedIdentity } from "../../wire";
  import Field from "../parts/field.svelte";
  import Unanswered from "../parts/unanswered.svelte";
  import Card from "./card.svelte";
  import Import from "./import.svelte";
  import { HELD, answered, awaitReceipt, edited, refused, sent } from "./saving";
  import type { Saving, Slot } from "./saving";

  const u = ui();
  const { lang } = u;
  const belief = u.conn.belief;
  const asked = u.conn.asking.ask(QUERIES.identity);

  const stated = $derived.by((): StatedIdentity | null => {
    const answer = $asked;
    return answer !== undefined && "identity" in answer && "stated" in answer.identity
      ? answer.identity.stated
      : null;
  });
  const unreadable = $derived.by(() => {
    const answer = $asked;
    return answer !== undefined && "identity" in answer && "unreadable" in answer.identity
      ? answer.identity.unreadable
      : null;
  });

  let userId = $state("");
  let importedFrom = $state<string | null>(null);
  let about = $state("");
  let mayor = $state("");
  let person = $state.raw<Saving>(HELD);
  let named = $state.raw<Saving>(HELD);

  $effect(() => {
    if (stated === null) return;
    const version = stated.version;
    person = answered(person, version);
    named = answered(named, version);
    if (person.kind === "held" || person.kind === "saved") {
      userId = stated.user_id ?? "";
      importedFrom = stated.imported_from ?? null;
      about = stated.about;
    }
    if (named.kind === "held" || named.kind === "saved") mayor = stated.mayor ?? "";
  });

  let seen = $belief.refusal;
  $effect(() => {
    const error = $belief.refusal;
    if (error === null || error === seen) return;
    seen = error;
    person = refused(person, error);
    named = refused(named, error);
  });

  const personSlot: Slot = { now: () => person, mark: (next) => (person = next) };
  const namedSlot: Slot = { now: () => named, mark: (next) => (named = next) };

  function settle(card: IdentityCard, base: string, slot: Slot): void {
    if (stated === null || !u.send(putIdentity(card, base))) return;
    const mine = sent(stated.version);
    slot.mark(mine);
    awaitReceipt(mine, slot);
  }

  function savePerson(): void {
    if (stated === null) return;
    const id = userId.trim();
    const card: IdentityCard = {
      person: {
        user_id: id === "" ? null : id,
        imported_from: id === "" ? null : importedFrom,
        about,
      },
    };
    settle(card, stated.preferences_text, personSlot);
  }

  function saveMayor(): void {
    if (stated === null) return;
    const name = mayor.trim();
    settle({ mayor: { name: name === "" ? null : name } }, stated.mayor_text, namedSlot);
  }

  function personMoved(): void {
    person = edited(userId.trim() !== (stated?.user_id ?? "") || about !== stated?.about);
  }
</script>

{#if unreadable !== null}
  <div class="asks flex max-w-talk flex-col gap-tight rounded-card bg-raised px-base py-snug">
    <span class="text-label font-label text-text">{say($lang, "you_unreadable")}</span>
    <p class="font-mono text-note text-text-quiet">
      {`${unreadable.document === "mayor" ? "MAYOR.md" : "PREFERENCES.md"}:${String(unreadable.line)} · ${unreadable.why}`}
    </p>
    <a
      href={toFragment({ kind: "setup", group: "run" })}
      class="w-fit text-label text-text-quiet underline hover:text-text"
    >
      {say($lang, "you_unreadable_open")}
    </a>
  </div>
{:else if stated === null}
  {#if $asked !== undefined && "unavailable" in $asked}
    <Unanswered query={$asked.unavailable.query} asked={QUERIES.identity} />
  {/if}
{:else}
  <div class="flex max-w-talk flex-col gap-base">
    <Card
      title="you_person"
      note="you_person_note"
      saving={person}
      settled="you_next_session"
      onSave={savePerson}
    >
      <Field
        label={say($lang, "you_user_id")}
        placeholder={say($lang, "you_user_id_default")}
        value={userId}
        mono
        onInput={(next: string) => {
          userId = next;
          importedFrom = null;
          personMoved();
        }}
      />
      <Import
        onTake={(login: string, host: string) => {
          userId = login;
          importedFrom = host;
          person = edited(true);
        }}
      />
      <label class="flex flex-col gap-tight">
        <span class="text-note text-text-quiet">{say($lang, "you_about")}</span>
        <textarea
          class="min-h-figure w-full rounded-control border border-edge-input bg-raised px-base py-snug text-body text-text"
          bind:value={about}
          oninput={personMoved}></textarea>
      </label>
    </Card>
    <Card
      title="you_mayor"
      note="you_mayor_note"
      saving={named}
      settled="you_next_session"
      onSave={saveMayor}
    >
      <Field
        label={say($lang, "you_mayor_name")}
        placeholder={say($lang, "nav_mayor")}
        value={mayor}
        onInput={(next: string) => {
          mayor = next;
          named = edited(mayor.trim() !== (stated?.mayor ?? ""));
        }}
      />
      <p class="text-note text-text-faint">{say($lang, "you_mayor_address")}</p>
    </Card>
  </div>
{/if}
