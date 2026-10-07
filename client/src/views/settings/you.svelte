<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // You and the main agent (refrain roadmap §3-13): two cards, each
  // saved on its own against the document it lives in. The person's ID
  // and the "about you" text live in `PREFERENCES.md`, the Mayor's name
  // in `MAYOR.md`; the city reads both in one place (`city::Naming`)
  // and answers them through `Query::Identity`, so this page holds no
  // second copy: each card starts from the answer, saves through
  // `PutIdentity` against the text the answer named as its base, and
  // calls itself saved only when the city answers with a new version
  // (`views/settings/saving.ts`).
  //
  // An identity area the city cannot read is its own answer rather than
  // a default name: drawing the default would let the next save write
  // over the line the person got wrong, so the cards step aside and the
  // page says which line, and the raw documents stay one entry away.
  //
  // A name takes effect at the next session: a session freezes the
  // names its first run read (glossary, identity area).

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
    return answer !== undefined && "identity" in answer && "stated" in answer.identity ? answer.identity.stated : null;
  });
  const unreadable = $derived.by(() => {
    const answer = $asked;
    return answer !== undefined && "identity" in answer && "unreadable" in answer.identity ? answer.identity.unreadable : null;
  });

  // The two cards' fields, which follow the answer until somebody types.
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

  // A refusal that arrives while a card waits is that card's answer.
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

  // Sends one card against the text the answer named as its base, and
  // marks the card sent; the receipt or the patience settles it later.
  function settle(card: IdentityCard, base: string, slot: Slot): void {
    if (stated === null || !u.send(putIdentity(card, base))) return;
    const mine = sent(stated.version);
    slot.mark(mine);
    awaitReceipt(mine, slot);
  }

  function savePerson(): void {
    if (stated === null) return;
    const id = userId.trim();
    const card: IdentityCard = { person: { user_id: id === "" ? null : id, imported_from: id === "" ? null : importedFrom, about } };
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
    <a href={toFragment({ kind: "setup", group: "run" })} class="w-fit text-label text-text-quiet underline hover:text-text">
      {say($lang, "you_unreadable_open")}
    </a>
  </div>
{:else if stated === null}
  {#if $asked !== undefined && "unavailable" in $asked}
    <Unanswered query={$asked.unavailable.query} asked={QUERIES.identity} />
  {/if}
{:else}
  <div class="flex max-w-talk flex-col gap-base">
    <Card title="you_person" note="you_person_note" saving={person} settled="you_next_session" onSave={savePerson}>
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
          class="min-h-figure w-full rounded-control border border-edge-input bg-page px-base py-snug text-body text-text"
          bind:value={about}
          oninput={personMoved}
        ></textarea>
      </label>
    </Card>
    <Card title="you_mayor" note="you_mayor_note" saving={named} settled="you_next_session" onSave={saveMayor}>
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
