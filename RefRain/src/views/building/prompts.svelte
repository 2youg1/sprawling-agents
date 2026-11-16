<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  
  
  import { readAnswer } from "../../core/answered";
  import { heldIn, heldWithin } from "../../core/belief/rooms";
  import { putPrompt } from "../../core/commands";
  import { fill, say } from "../../core/lang";
  import { toFragment } from "../../core/route";
  import { ui } from "../../ui";
  import { Address as AddressSchema } from "../../wire";
  import type { Address, PrefixAnswer, PromptStep, Query } from "../../wire";
  import { openDocument } from "../inspect/open.svelte";
  import Dialog from "../parts/dialog.svelte";
  import Unanswered from "../parts/unanswered.svelte";
  import { actionName, actionWord, cardsOf, slotsOf } from "./prompts";
  import type { CardAction, CardLook, PromptsLook } from "./prompts";
  import Look from "./prompts.look.svelte";

  interface Props {
    readonly building: Address;
    
    readonly rooms: readonly string[];
  }

  const { building, rooms }: Props = $props();

  const u = ui();
  const { lang } = u;
  const belief = u.conn.belief;

  
  let picked = $state<Address | null>(null);
  const newestRoom = $derived(heldWithin($belief, building).at(-1)?.addr ?? null);
  const room = $derived(picked ?? newestRoom ?? building);
  
  
  const last = $derived(heldIn($belief, room).at(-1)?.run);

  const layersAsked = $derived<Query>({ layers: { addr: room } });
  const layers = $derived(u.conn.asking.ask(layersAsked));
  const read = $derived(
    readAnswer($layers, (answer) => ("layers" in answer ? answer.layers : undefined)),
  );
  let sent = $state.raw<PrefixAnswer | undefined>(undefined);
  $effect(() => {
    const run = last;
    sent = undefined;
    if (run === undefined) return;
    return u.conn.asking.ask({ prefix: { run } }).subscribe((answer) => {
      sent = answer !== undefined && "prefix" in answer ? answer.prefix : undefined;
    });
  });

  
  let asking = $state.raw<{ readonly card: CardLook; readonly step: PromptStep } | null>(null);

  function act(card: CardLook, action: CardAction): void {
    if (action.kind === "open") {
      openDocument({ building, path: action.path, version: null });
      return;
    }
    if (action.step === "own") {
      u.send(putPrompt(card.at, card.base, action.step));
      return;
    }
    asking = { card, step: action.step };
  }

  const look = $derived.by((): PromptsLook | null => {
    if (read.kind !== "held") return null;
    return {
      rooms: choices,
      roomsLabel: say($lang, "prompts_rooms"),
      heading: fill(say($lang, last === undefined ? "prompts_no_run" : "prompts_last_run"), {
        room,
      }),
      slots: sent === undefined ? [] : slotsOf(sent, $lang),
      run:
        last === undefined
          ? undefined
          : {
              label: say($lang, "prompts_open_run"),
              href: toFragment({ kind: "run", run: last, lens: "prompt" }),
            },
      note: say($lang, "prompts_next_run"),
      documentsLabel: say($lang, "prompts_documents"),
      cards: cardsOf(read.value.documents, building, $lang),
      actionLabel: (action) => actionWord(action, $lang),
      actionName: (card, action) => actionName(card, action, $lang),
      onAction: act,
    };
  });

  const choices = $derived(
    [building, ...rooms.map((name) => AddressSchema.make(`${building}/${name}`))].map((each) => ({
      label: each,
      current: each === room,
      onPress: () => {
        picked = each;
      },
    })),
  );
</script>

{#if look !== null}
  <Look {...look} />
{:else if read.kind === "unavailable"}
  <Unanswered query={read.query} asked={layersAsked} />
{:else}
  <p class="text-text-faint">…</p>
{/if}

<Dialog
  open={asking !== null}
  title={asking === null
    ? ""
    : fill(
        say($lang, asking.step === "inherit" ? "prompts_inherit_title" : "prompts_restore_title"),
        { at: asking.card.at },
      )}
  detail={asking === null
    ? ""
    : say($lang, asking.step === "inherit" ? "prompts_inherit_detail" : "prompts_restore_detail")}
  confirmLabel={asking === null ? "" : actionWord(asking.step, $lang)}
  cancelLabel={say($lang, "part_cancel")}
  destructive
  onConfirm={() => {
    const held = asking;
    asking = null;
    if (held !== null) u.send(putPrompt(held.card.at, held.card.base, held.step));
  }}
  onCancel={() => {
    asking = null;
  }}
/>
