<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // The one card for everything that stops and asks the person
  // (client/Spec.lean §7C): a 2 px bar down the leading edge and a glyph, a
  // heading that says who asks and when, a body that is the kind's own,
  // and at most three answers, each on a chord the key table names
  // (`core/keys`). A person learns the card once; every later one is
  // recognised rather than read.
  //
  // **The glyph is the encoding, the colour only repeats it**: a
  // forced-colour mode repaints the bar and leaves the drawing, so each
  // kind is its own drawing in the table below. A kind is added by a row
  // there and a body at the call site; the props below do not change.
  //
  // **An answer is a choice the caller hands in, never an action this
  // file knows.** The card decides how answers look and which key
  // reaches each; what an answer sends is the caller's, because only
  // the caller knows whether it is an approval, a dismissal or a
  // proposal's verdict.
  import type { Snippet } from "svelte";

  import type { Action } from "../../core/keys";
  import type { GlyphName } from "./glyph";

  // What is being asked. `question` is a design question a resident
  // filed (`ApprovalItem`); `ask` is a door waiting on the person's own
  // hand (`E_APPROVAL_PENDING`); `proposal` is a change a run offers to
  // a document (`ProposalCard`, client/Spec.lean §4-55).
  export type DecideKind = "question" | "ask" | "proposal";

  // The three answers, each on its key.
  export type Answer = "yes" | "edit" | "no";

  export interface Choice {
    readonly answer: Answer;
    readonly label: string;
    // Why the answer cannot be given now; the control stays reachable
    // and says so (client/Spec.lean §7-2).
    readonly why?: string | undefined;
    readonly onPress: () => void;
  }

  export interface DecideProps {
    readonly kind: DecideKind;
    // Who asks, in words; the heading of the card and its name.
    readonly asker: string;
    // When it was asked, already in the reader's clock. A proposal card
    // carries no moment on the wire, so its card states the version it
    // was made on here instead: the one thing that places it.
    readonly at: string;
    // The kind's own body: what is asked, and what it is about.
    readonly body: Snippet;
    // At most one choice per answer, in the order yes, edit, no.
    readonly choices: readonly Choice[];
  }

  const GLYPH: Readonly<Record<DecideKind, GlyphName>> = {
    question: "hand",
    ask: "gate",
    proposal: "propose",
  };

  const KEY: Readonly<Record<Answer, Action>> = {
    yes: "decide.yes",
    edit: "decide.edit",
    no: "decide.no",
  };

  const ORDER: readonly Answer[] = ["yes", "edit", "no"];
</script>

<script lang="ts">
  import { keymap } from "../../core/keys";
  import Button from "./button.svelte";
  import Glyph from "./glyph.svelte";
  import { Kbd } from "./kbd.svelte";

  const { kind, asker, at, body, choices }: DecideProps = $props();

  const keys = keymap();
  const id = $props.id();

  const ordered = $derived(
    ORDER.flatMap((answer) => choices.filter((choice) => choice.answer === answer).slice(0, 1)),
  );

  function writing(target: EventTarget | null): boolean {
    return (
      target instanceof HTMLInputElement ||
      target instanceof HTMLTextAreaElement ||
      target instanceof HTMLSelectElement ||
      (target instanceof HTMLElement && target.isContentEditable)
    );
  }

  // The keys are heard on the card itself, below every control in it.
  let card = $state<HTMLElement | undefined>(undefined);
  $effect(() => {
    const held = card;
    if (held === undefined) return;
    held.addEventListener("keydown", answer);
    return () => {
      held.removeEventListener("keydown", answer);
    };
  });

  // The three chords answer this card while the focus is anywhere inside
  // it, and nowhere else: they are the card's, read through the one key
  // table so a rebinding reaches them. Each holds the accelerator, so a
  // letter typed while the focus sat on the card answers nothing.
  function answer(event: KeyboardEvent): void {
    if (event.isComposing) return;
    const action = keys.acting({
      key: event.key,
      ctrlKey: event.ctrlKey,
      metaKey: event.metaKey,
      shiftKey: event.shiftKey,
      altKey: event.altKey,
      target: writing(event.target) ? "field" : "page",
    });
    const chosen = ordered.find((choice) => KEY[choice.answer] === action);
    if (chosen === undefined || chosen.why !== undefined) return;
    event.preventDefault();
    event.stopPropagation();
    chosen.onPress();
  }
</script>

<!-- The card is one region for a reader, named by its heading, and one
entry of whatever list it stands in: its own focus is where `j`/`k` and
the digits of the mailbox land (client/Spec.lean §7-11), and where its three
chords are heard. -->
<div
  class="asks flex min-w-0 flex-col gap-snug rounded-card py-base pr-pane focus-visible:wash"
  role="group"
  tabindex="-1"
  aria-labelledby="{id}-asker"
  data-decide={kind}
  data-entry
  bind:this={card}
>
  <div class="flex min-w-0 items-center gap-snug text-note">
    <Glyph name={GLYPH[kind]} size="sm" class="shrink-0 text-alert" />
    <span id="{id}-asker" class="min-w-0 flex-1 truncate text-text-quiet">{asker}</span>
    <span class="figure shrink-0 text-text-faint">{at}</span>
    <!-- The digit a list of entries draws beside each one; empty
    wherever the card is not in such a list (theme.css, the mailbox). -->
    <kbd class="entry-n" aria-hidden="true"></kbd>
  </div>
  <div class="min-w-0">{@render body()}</div>
  {#if ordered.length > 0}
    <div class="flex flex-wrap items-center justify-end gap-snug">
      {#each ordered as choice (choice.answer)}
        <span class="inline-flex items-center gap-tight">
          <Button
            label={choice.label}
            tone={choice.answer === "yes" ? "primary" : "quiet"}
            {...choice.why === undefined ? {} : { why: choice.why }}
            onPress={choice.onPress}
          />
          <span aria-hidden="true">
            <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression, @typescript-eslint/no-unsafe-call (a snippet call is the render itself; svelte-check types this imported snippet fine, and typescript-eslint does not resolve exports of another .svelte module) -->
            {@render Kbd({ action: KEY[choice.answer] })}
          </span>
        </span>
      {/each}
    </div>
  {/if}
</div>
