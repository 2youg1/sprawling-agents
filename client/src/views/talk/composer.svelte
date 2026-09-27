<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->
<script lang="ts" module>
  // The box a person writes in: one textarea where Enter sends, the
  // four pills that say which model answers, which room hears it, how
  // hard the model thinks and which mode the run works in, and the way
  // to stop a run while it is going. Nothing here decides where a
  // message goes; the page does. `composer.ts` owns what the pills offer
  // and what a pick means, `dropping.ts` what a dropped file becomes.
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
  import { heldIn } from "../../core/belief/rooms";
  import { newestWorking } from "../../core/belief/live";
  import type { Sending } from "../../core/doing";
  import { fill, say } from "../../core/lang";
  import { current } from "../../core/route";
  import { completed } from "../../core/completion";
  import { find, parse } from "../../core/slash";
  import type { Slash } from "../../core/slash_hands";
  import { canRecord } from "../../core/speaking";
  import { ui } from "../../ui";
  import type { Address } from "../../wire";
  import PillView from "./pill.svelte";
  import Actions from "./actions.svelte";
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
  }

  const { placeholder, sending, draft, onSend, onStop, hearing }: ComposerProps = $props();

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
  // Which list is over the box: the `/` menu is the only one left here.
  let open = $state(false);
  // The row the menu's cursor is on, as `aria-activedescendant` on the box.
  let activeId = $state<string | null>(null);
  // The verb that row spells, which a Tab that cannot lengthen the typed
  // prefix takes into the box.
  let pointed = $state<string | undefined>(undefined);
  let menuKeys: ((event: KeyboardEvent) => boolean) | null = null;
  let receipt: ReturnType<typeof setTimeout> | undefined = undefined;

  // Whether this engine grows a textarea to fit what is typed in it.
  // `field-sizing: content` does in one declaration what this script
  // does in three, and before the frame is painted; Safari and Firefox
  // have not shipped it (client-SPEC 9.0), so both paths stay and this
  // check decides which runs. Both cap at `max-h-output`.
  function grow(): void {
    if (CSS.supports("field-sizing", "content")) return;
    if (box === undefined) return;
    box.style.height = "auto";
    box.style.height = `${String(box.scrollHeight)}px`;
  }

  // Whatever put words in the box - a transcription, a completion, a
  // command that empties it - goes through here.
  function write(words: string): void {
    text = words;
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

  // The receipt: the pill reads where the words went, with a check, for
  // a moment, so a press is seen to land (ux A3). A box that speaks to
  // no room shows no receipt: there is nowhere to name.
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

  // The room this box speaks to, read off the address bar.
  const here = $derived.by((): Address | null => {
    const view = Option.getOrNull(current(u.bar));
    return view !== null && view.kind === "talk" ? view.address : null;
  });
  // Every room a person could move this conversation to.
  const cityAnswer = u.conn.asking.ask(QUERIES.city);
  const rooms = $derived(
    roomsKnown(
      $cityAnswer !== undefined && "city" in $cityAnswer ? $cityAnswer.city.buildings : [],
      $belief.rooms.keys(),
    ),
  );
  // The run this box would steer: what a typed `/stop` reaches too.
  const live = $derived(here === null ? undefined : newestWorking($belief, here));

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
    if (rest === null) {
      open = false;
      return;
    }
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

  // ---------------------------------------------------------- speaking

  function heardWords(words: string): void {
    const before = text;
    write(before === "" ? words : `${before} ${words}`);
  }

  onMount(() => {
    // A draft restored on mount is taller than one row.
    requestAnimationFrame(grow);
  });

  onDestroy(() => {
    keptDraft.flush();
  });
</script>

<!-- Focus is said by the edge going from quiet to firm, and by nothing
     else: a full-strength accent here made this box the brightest
     rectangle on any page - brighter than the stop button. The accent
     is a budget with two lines in it (client-SPEC 7B). A drag over the
     box is said the same way, with the raised fill, so a person sees
     where to let go. -->
<form
  class={[
    "relative rounded-panel border bg-raised px-base pt-base pb-snug shadow-float focus-within:border-edge-input",
    over ? "border-edge-input bg-raised-hover" : "border-edge-panel",
  ]}
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
  <!-- svelte-ignore a11y_autofocus (the box is what the page exists for, and the shell's own focus chord reaches it the same way) -->
  <textarea
    bind:this={box}
    bind:value={text}
    class="block max-h-output min-h-control w-full resize-none overflow-y-auto bg-transparent px-tight text-body leading-relaxed text-text outline-none field-sizing-content placeholder:text-text-faint focus-visible:outline-none"
    rows={1}
    {placeholder}
    aria-label={placeholder}
    aria-activedescendant={activeId}
    autofocus
    onkeydown={onKeydown}
    oninput={onInput}
  ></textarea>
  {#each unkept as each (each.kind === "refused" ? each.name : "")}
    {#if each.kind === "refused"}
      <p class="px-tight text-note text-alert" role="alert">
        {fill(say($lang, "talk_drop_refused"), { name: each.name, why: each.said === "" ? say($lang, "talk_not_live") : each.said })}
      </p>
    {/if}
  {/each}
  <div class="mt-snug flex items-center gap-tight text-note text-text-faint">
    <div class="flex min-w-0 grow flex-wrap items-center gap-tight">
      <PillView spec={specs[0]} />
      <PillView spec={specs[1]} />
      <PillView spec={specs[2]} />
      <PillView spec={specs[3]} />
      {#if kept}
        <span class="text-alert">{say($lang, "talk_not_live")}</span>
      {/if}
    </div>
    <Actions
      {sending}
      {handed}
      {here}
      empty={text.trim() === ""}
      hearing={hearing === true && canRecord()}
      onWords={heardWords}
      {onStop}
    />
  </div>
</form>
