<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->
<script lang="ts">
  import { Option } from "effect";
  import { onDestroy, onMount, untrack } from "svelte";
  import { get, readable } from "svelte/store";

  import { QUERIES } from "../../core/asking";
  import type { Snippet } from "svelte";
  import { heldIn } from "../../core/belief/rooms";
  import type { Sending } from "../../core/doing";
  import { runInFront } from "../../core/in_front";
  import { say } from "../../core/lang";
  import { markEndAtFrame, markStart } from "../../core/timing";
  import { current } from "../../core/route";
  import { find, parse } from "../../core/slash";
  import type { Slash } from "../../core/slash_hands";
  import type { Address } from "../../wire";
  import { canRecord } from "../../core/speaking";
  import { ui } from "../../ui";
  import SettingsRow from "./settings_row.svelte";
  import Coin from "./coin.svelte";
  import { faceOf } from "./coin_face";
  import Gauge from "./gauge.svelte";
  import Handed from "./handed.svelte";
  import Record from "./record.svelte";
  import TypedLine from "./typed_line.svelte";
  import Popover from "../parts/popover.svelte";
  import type { PopoverBinding } from "../parts/popover";
  import Unkept from "../parts/unkept.svelte";
  import {
    boxKey,
    completedAt,
    configLevel,
    menuColumns,
    offeredBy,
    pickSlash,
    picksFor,
    sessionModel,
    slashHands,
    startedIn,
  } from "./composer";
  import type { BoxKey, ComposerLook } from "./composer";
  import Look from "./composer.look.svelte";
  import { draftAt } from "./draft";
  import { TextBox } from "./box.svelte";
  import { IDLE, hand, settle } from "./handing";
  import { sessionHands } from "./session_hands";
  import type { Handing } from "./handing";
  import { insertAt } from "./dropping";
  import { DropZone } from "./drop_zone.svelte";
  import DropRefused from "./drop_refused.svelte";
  import { hearQuotes, joined } from "./quoting";
  import { keepSelection, recalled, restoreSelection } from "./standing";

  interface ComposerProps {
    readonly placeholder: string;
    readonly started?: boolean;
    readonly sending: Sending;

    readonly draft?: string | undefined;

    readonly onSend: (text: string) => boolean;
    readonly onStop: () => boolean;

    readonly hearing?: boolean | undefined;

    readonly band?: Snippet | undefined;

    readonly room?: Address | undefined;

    readonly door?: Snippet | undefined;
  }

  const {
    placeholder,
    sending,
    draft,
    onSend,
    onStop,
    hearing,
    band,
    room,
    door,
    started = false,
  }: ComposerProps = $props();

  const u = ui();
  const { lang } = u;
  const effort = u.effort;
  const belief = u.conn.belief;

  // svelte-ignore state_referenced_locally (the first place; the effect below follows the rest)
  let place = draft;
  let keptDraft = draftAt(u.prefs, place);
  let text = $state(keptDraft.read);
  const field = new TextBox(() => text);

  let kept = $state(false);

  let sent = $state(0);

  let focused = $state(false);

  let open = $state(false);

  let activeId = $state<string | null>(null);

  let pointed = $state<string | undefined>(undefined);
  let menuBinding = $state<PopoverBinding | null>(null);

  $effect(() => {
    const next = draft;
    untrack(() => {
      if (next !== place) moveTo(next);
    });
  });

  function moveTo(next: string | undefined): void {
    keptDraft.flush();
    keepSelection(place, field.element);
    place = next;
    keptDraft = draftAt(u.prefs, next);
    text = keptDraft.read;
    open = text.startsWith("/");
    requestAnimationFrame(arrived);
  }

  function arrived(): void {
    field.grow();
    restoreSelection(place, field.element);
  }

  $effect(() => (draft === undefined ? undefined : hearQuotes(draft, quoted)));
  function quoted(quote: string): void {
    keepSelection(place, field.element);
    write(joined(text, quote));
    requestAnimationFrame(arrived);
  }

  const unkept = $derived(draft === undefined ? readable(false) : u.prefs.draftUnkept(draft));

  function write(words: string): void {
    text = words;
    open = words.startsWith("/");
    keptDraft.replace(words);
    requestAnimationFrame(() => {
      field.grow();
    });
  }

  function submit(): void {
    const words = text.trim();
    if (words === "") return;

    const call = parse(words);
    const chosen = call === null ? undefined : find(call.verb);
    if (chosen !== undefined) {
      pick(chosen);
      return;
    }
    markStart("send_shown");
    if (onSend(words)) {
      markEndAtFrame("send_shown");
      handing = hand(words, get(belief));
      write("");
      kept = false;
      sent += 1;
    } else {
      kept = true;
    }
  }

  let handing: Handing = IDLE;
  $effect(() => {
    const next = settle(
      handing,
      $belief,
      untrack(() => text),
    );
    handing = next.handing;
    if (next.box !== null) write(next.box);
  });

  const endpoints = u.conn.asking.ask(QUERIES.endpoints);
  const city = u.conn.asking.ask(QUERIES.city);
  const buildings = $derived(
    $city !== undefined && "city" in $city ? $city.city.buildings : undefined,
  );
  const answer = $derived(
    $endpoints !== undefined && "endpoints" in $endpoints ? $endpoints.endpoints : undefined,
  );

  const shown = $derived(
    room === undefined
      ? Option.getOrNull(current(u.bar))
      : { kind: "talk" as const, address: room },
  );
  const here = $derived(shown !== null && shown.kind === "talk" ? shown.address : null);

  const live = $derived(shown === null ? undefined : runInFront($belief, shown));

  const startedHere = $derived(
    here !== null && startedIn(heldIn($belief, here), $belief.sessions[here]),
  );
  const session = $derived(
    here === null ? null : sessionModel(heldIn($belief, here), $belief.sessions[here] ?? null),
  );
  const picks = $derived(picksFor(u, session));

  const roomConfig = $derived(
    here === null ? readable(undefined) : u.conn.asking.ask({ config: { addr: here } }),
  );
  const offered = $derived(offeredBy(answer, $effort ?? configLevel($roomConfig), picks));

  const showing = $derived(open ? menuColumns($lang, text, offered.argued) : []);

  function closeMenu(): void {
    open = false;
    field.element?.focus();
  }

  function pick(chosen: Slash): void {
    const rest = pickSlash(
      chosen,
      text,
      slashHands({
        command: u.send,
        go: u.go,
        here,
        live,
        belief: get(belief),
        buildings,
        models: offered.models,
        effort: get(effort),
        setEffort: u.chooseEffort,
        policy: get(u.policy),
        setPolicy: u.choosePolicy,
        goal: say(get(lang), "talk_goal"),
        write,
        ...sessionHands(u, here),
      }),
    );
    if (rest === null) return;
    write(rest);
    field.element?.focus();
  }

  function onKeydown(event: KeyboardEvent): void {
    const state = { menu: open && showing.length > 0, empty: text === "" };
    const first = boxKey(event, state);
    if (first === "menu" && menuBinding?.keys(event) === true) {
      event.preventDefault();
      return;
    }
    act(first === "menu" ? boxKey(event, { ...state, menu: false }) : first, event);
  }

  function act(key: BoxKey, event: KeyboardEvent): void {
    switch (key) {
      case "complete":
        event.preventDefault();
        write(completedAt(text, pointed));
        return;
      case "recall": {
        const last = here === null ? null : recalled(get(belief), here);
        if (last === null) return;
        event.preventDefault();
        write(last);
        return;
      }
      case "send":
        event.preventDefault();
        submit();
        return;
      case "menu":
      case "type":
        return;
    }
  }

  function onInput(event: Event & { readonly currentTarget: HTMLTextAreaElement }): void {
    text = event.currentTarget.value;
    keptDraft.keep(text);
    field.grow();
    open = text.startsWith("/");
  }

  const zone = new DropZone(u, (paths) => {
    write(insertAt(text, field.element?.selectionStart ?? text.length, paths));
    field.element?.focus();
  });

  const opened = $derived(text === "" ? door : undefined);

  const look: ComposerLook = $derived({
    form: {
      "aria-label": say($lang, "region_composer"),
      onsubmit: (event: SubmitEvent) => {
        event.preventDefault();
        submit();
      },
      ondragover: zone.enter,
      ondragleave: zone.leave,
      ondrop: zone.drop,
    },
    over: zone.over,
    box: {
      rows: 1,
      placeholder: opened === undefined ? placeholder : undefined,
      "aria-label": placeholder,
      "aria-activedescendant": showing.length > 0 ? activeId : null,
      "aria-controls": showing.length > 0 ? menuBinding?.controls.join(" ") : undefined,

      autofocus: true,
      onkeydown: onKeydown,
      oninput: onInput,
      onfocus: () => {
        focused = true;
      },
      onblur: () => {
        focused = false;
      },
      [field.key]: field.hold,
    },
    menu: open && showing.length > 0 ? drawMenu : undefined,
    door: opened,
    band,
    keys: drawKeys,
    line: drawLine,
    under: drawUnder,
  });

  onMount(() => {
    requestAnimationFrame(arrived);
  });

  onDestroy(() => {
    keepSelection(place, field.element);
    keptDraft.flush();
  });
</script>

{#snippet drawMenu()}
  <Popover
    label="talk_commands"
    columns={showing}
    onApply={(_column, row: { readonly id: string }) => {
      // A verb row's id is its spelling; an argument row's is the whole
      // line it runs, which goes into the box before the verb reads it.
      const call = parse(row.id);
      const chosen = call === null ? undefined : find(call.verb);
      if (chosen === undefined) return;
      if (row.id !== chosen.spelling) write(row.id);
      pick(chosen);
    }}
    onClose={closeMenu}
    bind={(binding: PopoverBinding) => {
      menuBinding = binding;
    }}
    onCursorChange={(rowId) => {
      activeId = rowId;
    }}
    onCursorRow={(row) => {
      pointed = row?.id;
    }}
  />
{/snippet}

{#snippet drawKeys()}
  {#if hearing === true && canRecord()}
    <Record
      onWords={(words: string) => {
        write(text === "" ? words : `${text} ${words}`);
      }}
    />
  {/if}
  <Gauge room={here}>
    <Coin
      face={faceOf(text, live !== undefined)}
      {sending}
      onStop={() => {
        onStop();
        // Stopping with an empty box turns the coin idle and unmounts it;
        // return focus to the field so the next keystrokes still land here.
        field.element?.focus();
      }}
    />
  </Gauge>
{/snippet}

{#snippet drawLine()}
  <TypedLine {text} box={field.element} lit={focused || text !== ""} />
{/snippet}

{#snippet drawUnder()}
  <Handed room={here} {sent} />
  {#if $unkept && text !== ""}
    <Unkept words={() => text} />
  {/if}
  <DropRefused refused={zone.refused} />
  <SettingsRow
    picker={offered.picker}
    room={here}
    draws={band !== undefined || started || startedHere ? "notice" : "everything"}
    {kept}
  />
{/snippet}

<Look {...look} />
