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
  import { completed } from "../../core/completion";
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
  import { boxKey, menuColumns, pickSlash, picksFor, pills, roomsKnown, sessionModel, slashHands } from "./composer";
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
    // Where the unsent words are kept, when they are kept at all: the
    // room or the run this box speaks to.
    readonly draft?: string | undefined;
    // Both answer whether the frame went out, so the box can keep the
    // words when it did not.
    readonly onSend: (text: string) => boolean;
    readonly onStop: () => boolean;
    // Whether this city has an endpoint that transcribes. A microphone
    // on a city with none is a button whose only answer is a refusal.
    readonly hearing?: boolean | undefined;
    // In the band under the world layer's sheet on one column, what stands above the words (the last thing said);
    // a box in the band draws no settings row beyond the notice that a change was kept, the session sheet says it (docs/frontend-method.md §7H, §7I).
    readonly band?: Snippet | undefined;
    // The room this box speaks to, when the page holding it says so; the
    // address bar answers otherwise.
    readonly room?: Address | undefined;
  }

  const { placeholder, sending, draft, onSend, onStop, hearing, band, room, started = false }: ComposerProps = $props();

  const u = ui();
  const { lang } = u;
  const effort = u.effort;
  const belief = u.conn.belief;

  // The words in the box, and where they are kept while unsent: the
  // place the page names, followed when it names another, so a room
  // switched to in this tab gets back its own words and selection (4-63).
  // svelte-ignore state_referenced_locally (the first place; the effect below follows the rest)
  let place = draft;
  let keptDraft = draftAt(u.prefs, place);
  let text = $state(keptDraft.read);
  const field = new TextBox(() => text);
  // A message the connection would not take: the words stay in the box.
  let kept = $state(false);
  // How many times words were handed, which the send receipt counts.
  let sent = $state(0);
  // Whether the box has the focus, which draws its line at full strength.
  let focused = $state(false);
  // Which list is over the box: the `/` menu is the only one left here.
  let open = $state(false);
  // The row the menu's cursor is on, as `aria-activedescendant` on the box.
  let activeId = $state<string | null>(null);
  // The verb that row spells, which a Tab that cannot lengthen the typed
  // prefix takes into the box.
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

  // A box that arrived at its place: grown to the words, the caret where
  // the person left it there.
  function arrived(): void {
    field.grow();
    restoreSelection(place, field.element);
  }

  // A line quoted from the inspector while this box is open joins the
  // words where they end, and the caret stays where the person left it.
  $effect(() => (draft === undefined ? undefined : hearQuotes(draft, quoted)));
  function quoted(quote: string): void {
    keepSelection(place, field.element);
    write(joined(text, quote));
    requestAnimationFrame(arrived);
  }

  // Whether the browser refused to keep this place's words (4-63).
  const unkept = $derived(draft === undefined ? readable(false) : u.prefs.draftUnkept(draft));

  // Whatever put words in the box - a transcription, a completion, a
  // command that empties it - goes through here, and the menu follows
  // the words as it does for typing: `/help` writes `/` to open it.
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
    // A line that spells a verb is that verb, whether the menu is open
    // or the person closed it: sent as a message, `/new` reached the
    // model as words to answer.
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

  // The city's answer to the words last handed: a refusal puts them
  // back in the box (`handing.ts`).
  let handing: Handing = IDLE;
  $effect(() => {
    const next = settle(handing, $belief, untrack(() => text));
    handing = next.handing;
    if (next.box !== null) write(next.box);
  });

  // ------------------------------------------------- what the city offers

  const endpoints = u.conn.asking.ask(QUERIES.endpoints);
  const answer = $derived(
    $endpoints !== undefined && "endpoints" in $endpoints ? $endpoints.endpoints : undefined,
  );
  // The id a command carries next to the name a person reads.
  const models = $derived(
    (answer?.endpoints ?? []).flatMap((endpoint) =>
      endpoint.models.map((row) => ({ endpoint: endpoint.name, label: endpoint.label, model: row.id })),
    ),
  );
  const main = $derived(answer?.chosen.find((each) => each.tag === "main"));

  // The room this box speaks to: the page's word for it, or the address bar's.
  const shown = $derived(room === undefined ? Option.getOrNull(current(u.bar)) : { kind: "talk" as const, address: room });
  const here = $derived(shown !== null && shown.kind === "talk" ? shown.address : null);
  // Every room a person could move this conversation to.
  const cityAnswer = u.conn.asking.ask(QUERIES.city);
  const rooms = $derived(
    roomsKnown(
      $cityAnswer !== undefined && "city" in $cityAnswer ? $cityAnswer.city.buildings : [],
      [...$belief.rooms.keys(), ...(here === null ? [] : [here])],
    ),
  );
  // The run in front of the person: what a typed `/stop` and `/steer` reach.
  const live = $derived(shown === null ? undefined : runInFront($belief, shown));

  const startedHere = $derived(here !== null && heldIn($belief, here).some((run) => run.lastSeq > ($belief.sessions[here] ?? 0)));
  const session = $derived(here === null ? null : sessionModel(heldIn($belief, here), $belief.sessions[here] ?? null));
  const offered = $derived({ served: models, chosen: main, session, rooms, here, effort: $effort });
  const specs = $derived(pills($lang, offered, picksFor(u, session)));

  // ------------------------------------------------------- the `/` menu

  const showing = $derived(open ? menuColumns($lang, text) : []);

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
        models,
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
        write(completed(text, pointed));
        return;
      case "recall": {
        // ↑ in an empty box brings back what was last sent here.
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

  // ----------------------------------------------------------- dropping

  const zone = new DropZone(u, (paths) => {
    write(insertAt(text, field.element?.selectionStart ?? text.length, paths));
    field.element?.focus();
  });

  // ------------------------------------------------------- the look

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
      placeholder,
      "aria-label": placeholder,
      "aria-activedescendant": showing.length > 0 ? activeId : null,
      "aria-controls": showing.length > 0 ? menuBinding?.controls.join(" ") : undefined,
      // The box is what the page exists for, and the shell's own focus
      // chord reaches it the same way.
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
      const chosen = find(row.id);
      if (chosen !== undefined) pick(chosen);
    }}
    onClose={closeMenu}
    bind={(binding: PopoverBinding) => { menuBinding = binding; }}
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
    <!-- What the microphone heard joins the words; it is never sent by itself (4-16). -->
    <Record onWords={(words: string) => { write(text === "" ? words : `${text} ${words}`); }} />
  {/if}
  <Gauge room={here}>
    <Coin
      face={faceOf(text, live !== undefined)}
      {sending}
      onStop={() => {
        onStop();
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
  <SettingsRow {specs} room={here} draws={band !== undefined || started || startedHere ? "notice" : "everything"} {kept} />
{/snippet}

<Look {...look} />
