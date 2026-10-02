<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->
<script lang="ts" module>
  // The box a person writes in, written as a page (client-SPEC 7I): the
  // words, a line under them, the settings row under the line, and the
  // coin key in its context ring beside the words. Nothing here decides
  // where a message goes; the page does. `composer.ts` owns what the
  // pills offer and what a pick means, `dropping.ts` a dropped file.
  //
  // A line that begins with `/` is a command rather than a message, and
  // the menu over the box is the same list the Ctrl-K palette reads.

  // How long the send receipt holds its words.
  const RECEIPT_MS = 400;
</script>

<script lang="ts">
  import { Option } from "effect";
  import { onDestroy, onMount, untrack } from "svelte";
  import { get } from "svelte/store";

  import { QUERIES } from "../../core/asking";
  import type { Snippet } from "svelte";
  import { heldIn } from "../../core/belief/rooms";
  import type { Sending } from "../../core/doing";
  import { runInFront } from "../../core/in_front";
  import { fill, say } from "../../core/lang";
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
  import Record from "./record.svelte";
  import TypedLine from "./typed_line.svelte";
  import Popover from "../parts/popover.svelte";
  import {
    draftAt,
    menuColumns,
    pickSlash,
    picksFor,
    pills,
    roomsKnown,
    sessionModel,
    slashHands,
  } from "./composer";
  import { IDLE, hand, settle } from "./handing";
  import type { Handing } from "./handing";
  import { dropped, insertAt } from "./dropping";
  import type { Kept } from "./dropping";

  interface ComposerProps {
    readonly placeholder: string;
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
    // In the panorama tier's band, what stands above the words (the last thing said);
    // a box in the band draws no settings row, the session sheet says it (7D, 7I).
    readonly band?: Snippet | undefined;
    // The room this box speaks to, when the page holding it says so; the
    // address bar answers otherwise.
    readonly room?: Address | undefined;
  }

  const { placeholder, sending, draft, onSend, onStop, hearing, band, room }: ComposerProps = $props();

  const u = ui();
  const { lang } = u;
  const effort = u.effort;
  const mode = u.mode;
  const belief = u.conn.belief;

  // The words in the box, and where they are kept while unsent. The
  // place is read once, at mount: a box that moved to another room
  // keeps the words in it rather than swapping them under the writer.
  // svelte-ignore state_referenced_locally (the draft's place is read once at mount and never followed)
  const keptDraft = draftAt(u.prefs, draft);
  let text = $state(keptDraft.read);
  let box = $state<HTMLTextAreaElement | undefined>(undefined);
  // A message the connection would not take: the words stay in the box.
  let kept = $state(false);
  let handed = $state(false);
  // Whether the box has the focus, which draws its line at full strength.
  let focused = $state(false);
  // Which list is over the box: the `/` menu is the only one left here.
  let open = $state(false);
  // The row the menu's cursor is on, as `aria-activedescendant` on the box.
  let activeId = $state<string | null>(null);
  // The verb that row spells, which a Tab that cannot lengthen the typed
  // prefix takes into the box.
  let pointed = $state<string | undefined>(undefined);
  let menuKeys: ((event: KeyboardEvent) => boolean) | null = null;
  let receipt: ReturnType<typeof setTimeout> | undefined = undefined;

  // `field-sizing: content` grows the box before the frame is painted
  // where the engine has it; this is the path for the engines without it.
  function grow(): void {
    if (CSS.supports("field-sizing", "content")) return;
    if (box === undefined) return;
    box.style.height = "auto";
    box.style.height = `${String(box.scrollHeight)}px`;
  }

  // Whatever put words in the box - a transcription, a completion, a
  // command that empties it - goes through here, and the menu follows
  // the words as it does for typing: `/help` writes `/` to open it.
  function write(words: string): void {
    text = words;
    open = words.startsWith("/");
    keptDraft.replace(words);
    requestAnimationFrame(grow);
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
    if (onSend(words)) {
      handing = hand(words, get(belief));
      write("");
      kept = false;
      landed();
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

  // The receipt: for a moment a screen reader is told where the words
  // went, so a press is heard to land; nowhere to name, no receipt.
  function landed(): void {
    const room = here;
    if (room === null) return;
    handed = true;
    if (receipt !== undefined) clearTimeout(receipt);
    receipt = setTimeout(() => {
      handed = false;
      receipt = undefined;
    }, RECEIPT_MS);
  }

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
      $belief.rooms.keys(),
    ),
  );
  // The run in front of the person: what a typed `/stop` and `/steer` reach.
  const live = $derived(shown === null ? undefined : runInFront($belief, shown));

  const session = $derived(
    here === null ? null : sessionModel(heldIn($belief, here), $belief.sessions[here] ?? null),
  );
  const specs = $derived(
    pills(
      $lang,
      { served: models, chosen: main, session, rooms, here, effort: $effort, mode: $mode },
      picksFor(u, here, session),
    ),
  );

  // ------------------------------------------------------- the `/` menu

  const showing = $derived(open ? menuColumns($lang, text) : []);

  function holdKeys(keys: (event: KeyboardEvent) => boolean): void {
    menuKeys = keys;
  }

  function closeMenu(): void {
    open = false;
    box?.focus();
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
        setEffort: (level) => {
          u.chooseEffort(level);
        },
        mode: get(mode),
        goal: say(get(lang), "talk_goal"),
        write,
      }),
    );
    if (rest === null) return;
    write(rest);
    box?.focus();
  }

  function onKeydown(event: KeyboardEvent): void {
    // A key that confirms an input method's composition belongs to the
    // input method, not to the menu.
    if (open && showing.length > 0 && !event.isComposing) {
      if (event.key === "Tab") {
        event.preventDefault();
        write(completed(text, pointed));
        return;
      }
      if (menuKeys?.(event) === true) {
        event.preventDefault();
        return;
      }
    }
    if (event.key === "Enter" && !event.shiftKey && !event.isComposing) {
      event.preventDefault();
      submit();
    }
  }

  function onInput(): void {
    keptDraft.keep(text);
    grow();
    open = text.startsWith("/");
  }

  // ----------------------------------------------------------- dropping

  // Whether a drag is over the box, and what the last drop could not keep.
  let over = $state(false);
  let unkept = $state<readonly Kept[]>([]);

  function onDrop(event: DragEvent): void {
    over = false;
    const drop = dropped(event.dataTransfer, u.origin, u.pairing);
    if (drop.kind === "nothing") return;
    event.preventDefault();
    unkept = [];
    const place = (paths: readonly string[]): void => {
      write(insertAt(text, box?.selectionStart ?? text.length, paths));
      box?.focus();
    };
    if (drop.kind === "named") {
      place(drop.paths);
      return;
    }
    void drop.kept.then((kept) => {
      place(kept.flatMap((each) => (each.kind === "path" ? [each.path] : [])));
      unkept = kept.filter((each) => each.kind === "refused");
    });
  }

  onMount(() => {
    // A draft restored on mount is taller than one row.
    requestAnimationFrame(grow);
  });

  onDestroy(() => {
    keptDraft.flush();
  });
</script>

<!-- A page, not a card: focus is said by the line coming to full
strength, and a drag over the box by the wash it takes. -->
<form
  class={["relative rounded-control transition-colors", over ? "wash" : ""]}
  aria-label={say($lang, "region_composer")}
  onsubmit={(event) => {
    event.preventDefault();
    submit();
  }}
  ondragover={(event) => {
    event.preventDefault();
    over = true;
  }}
  ondragleave={(event) => {
    const next = event.relatedTarget;
    if (!(next instanceof Node && event.currentTarget.contains(next))) over = false;
  }}
  ondrop={onDrop}
>
  {#if open && showing.length > 0}
    <Popover
      label="talk_commands"
      columns={showing}
      onApply={(_column, row: { readonly id: string }) => {
        const chosen = find(row.id);
        if (chosen !== undefined) pick(chosen);
      }}
      onClose={closeMenu}
      bind={holdKeys}
      onCursorChange={(rowId) => {
        activeId = rowId;
      }}
      onCursorRow={(row) => {
        pointed = row?.id;
      }}
    />
  {/if}
  {#if band !== undefined}{@render band()}{/if}
  <div class="relative pb-snug">
    <div class="flex min-h-key items-end gap-base">
      <!-- svelte-ignore a11y_autofocus (the box is what the page exists for, and the shell's own focus chord reaches it the same way) -->
      <textarea
        bind:this={box}
        bind:value={text}
        class="block max-h-output min-h-key min-w-0 flex-1 resize-none overflow-y-auto bg-transparent py-snug text-body leading-relaxed text-text caret-accent outline-hidden field-sizing-content placeholder:text-text-faint"
        rows={1}
        {placeholder}
        aria-label={placeholder}
        aria-activedescendant={activeId}
        autofocus
        onkeydown={onKeydown}
        oninput={onInput}
        onfocus={() => {
          focused = true;
        }}
        onblur={() => {
          focused = false;
        }}
      ></textarea>
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
    </div>
    <TypedLine {text} {box} lit={focused || text !== ""} />
  </div>
  <span role="status" class="sr-only">
    {#if handed && here !== null}{fill(say($lang, "talk_handed"), { room: here })}{/if}
  </span>
  {#each unkept as each (each.kind === "refused" ? each.name : "")}
    {#if each.kind === "refused"}
      <p class="text-note text-alert" role="alert">
        {fill(say($lang, "talk_drop_refused"), { name: each.name, why: each.said === "" ? say($lang, "talk_not_live") : each.said })}
      </p>
    {/if}
  {/each}
  <SettingsRow {specs} room={here} draws={band !== undefined ? "notice" : session === null ? "everything" : "facts"} {kept} />
</form>
