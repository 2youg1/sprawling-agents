<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->
<script lang="ts" module>
  // The box a person writes in: one textarea where Enter sends, the
  // three pills that say which model answers, which room hears it and
  // how hard the model thinks, and the way to stop a run while it is
  // going. Nothing here decides where a message goes; the page does.
  // `composer.ts` owns what the pills offer and what a pick means.
  //
  // A line that begins with `/` is a command rather than a message, and
  // the menu over the box is the same list the Ctrl-K palette reads.
  //
  // **The send control is drawn here rather than in `parts/button.svelte`
  // for one reason: its second face.** After Enter it briefly reads
  // "handed to <room>" beside a check, so a person sees the send land
  // (ux A3), and the button part has no slot for a glyph. The paint is
  // the primary tier's, restated in tokens.

  // How long the send receipt holds its words.
  const RECEIPT_MS = 400;
</script>

<script lang="ts">
  import { Option } from "effect";
  import { onDestroy, onMount } from "svelte";
  import { get } from "svelte/store";

  import { QUERIES } from "../../core/asking";
  import { selectModel } from "../../core/commands";
  import type { Sending } from "../../core/doing";
  import { fill, say } from "../../core/lang";
  import { current } from "../../core/route";
  import { completed } from "../../core/completion";
  import { find } from "../../core/slash";
  import type { Slash } from "../../core/slash";
  import { canRecord, dictation } from "../../core/speaking";
  import { ui } from "../../ui";
  import type { Address } from "../../wire";
  import Button from "../parts/button.svelte";
  import PillView from "./pill.svelte";
  import Glyph from "../parts/glyph.svelte";
  import Popover from "../parts/popover.svelte";
  import {
    SPELLING,
    decodeRoom,
    draftAt,
    effortLevel,
    menuColumns,
    newestRun,
    pickSlash,
    pills,
    roomsKnown,
    slashHands,
    splitModel,
  } from "./composer";
  import type { Picks } from "./composer";

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
    if (onSend(words)) {
      write("");
      kept = false;
      landed();
    } else {
      kept = true;
    }
  }

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
      Object.values($belief.runs),
    ),
  );
  // The run this box would steer: what a typed `/stop` reaches too.
  const live = $derived(newestRun(Object.values($belief.runs), here, "moving"));

  const picks: Picks = { model: pickModel, workspace: pickRoom, effort: pickEffort };
  const specs = $derived(pills($lang, { served: models, chosen: main, rooms, here, effort: $effort }, picks));

  function pickModel(value: string): void {
    const model = splitModel(value);
    if (model === null) return;
    u.send(selectModel(model.endpoint, model.model, "main"));
  }

  function pickRoom(value: string): void {
    const address = decodeRoom(value);
    if (address === null) return;
    u.go({ kind: "talk", address });
  }

  function pickEffort(value: string): void {
    u.chooseEffort(effortLevel(value));
  }

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
        runs: Object.values($belief.runs),
        models,
        effort: get(effort),
        setEffort: (level) => {
          u.chooseEffort(level);
        },
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
    if (open && showing.length > 0) {
      if (event.key === "Tab") {
        event.preventDefault();
        write(completed(text));
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

  // ---------------------------------------------------------- speaking

  const heard = dictation(u.origin, u.pairing, (words) => {
    const before = text;
    write(before === "" ? words : `${before} ${words}`);
  });
  const taking = heard.taking;
  const transcribing = heard.hearing;
  const refused = heard.refused;

  onMount(() => {
    // A draft restored on mount is taller than one row.
    requestAnimationFrame(grow);
  });

  onDestroy(() => {
    keptDraft.flush();
  });
</script>

<!-- Focus is said by the edge going from dashed to solid, and by
     nothing else: a full-strength accent here made this box the
     brightest rectangle on any page - brighter than the stop button.
     The accent is a budget with two lines in it (client-SPEC 7B). -->
<form
  class="relative rounded-panel border border-dashed border-edge-input bg-raised p-base shadow-float focus-within:border-solid"
  aria-label={say($lang, "region_composer")}
  onsubmit={(event) => {
    event.preventDefault();
    submit();
  }}
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
    />
  {/if}
  <!-- svelte-ignore a11y_autofocus (the box is what the page exists for, and the shell's own focus chord reaches it the same way) -->
  <textarea
    bind:this={box}
    bind:value={text}
    class="block max-h-output w-full resize-none overflow-y-auto bg-transparent text-body leading-relaxed text-text field-sizing-content placeholder:text-text-disabled"
    rows={1}
    {placeholder}
    aria-label={placeholder}
    aria-activedescendant={activeId}
    autofocus
    onkeydown={onKeydown}
    oninput={onInput}
  ></textarea>
  <div class="mt-snug flex flex-wrap items-center gap-base text-note text-text-faint">
    <div class="flex min-w-[min(100%,20rem)] grow flex-wrap items-center gap-tight">
      <PillView spec={specs[0]} />
      <PillView spec={specs[1]} />
      <PillView spec={specs[2]} />
      {#if hearing === true && canRecord()}
        <button
          type="button"
          class={[
            "relative flex h-control-sm shrink-0 items-center gap-tight rounded-pill px-base text-note before:absolute before:-inset-snug before:content-['']",
            $taking ? "bg-alert text-on-accent" : $transcribing ? "bg-raised text-text-disabled" : "bg-raised text-text-quiet hover:bg-raised-hover",
          ]}
          aria-disabled={$transcribing}
          onclick={() => {
            if ($transcribing) return;
            heard.speak();
          }}
        >
          {$transcribing ? say($lang, "talk_hearing") : $taking ? say($lang, "talk_recording") : say($lang, "talk_record")}
        </button>
      {/if}
      {#if kept}
        <span class="text-alert">{say($lang, "talk_not_live")}</span>
      {/if}
      {#if $refused}
        <span class="text-alert">{say($lang, "link_refused")}</span>
      {/if}
    </div>
    <div class="ml-auto flex shrink-0 items-center gap-base">
      {#if sending !== "dispatch"}
        <Button
          label={say($lang, "talk_stop")}
          tone="destructive"
          onPress={() => {
            onStop();
          }}
        />
      {/if}
      <button
        type="submit"
        class={[
          "flex h-control-lg items-center gap-snug rounded-control px-base text-label transition-[background-color,color,opacity,transform]",
          "duration-100 ease-standard active:scale-[0.98] motion-reduce:transition-none motion-reduce:active:scale-100",
          text.trim() === "" && !handed ? "bg-raised text-text-disabled" : "bg-accent text-on-accent hover:bg-accent-hover",
        ]}
        aria-disabled={text.trim() === ""}
        onclick={(event) => {
          if (text.trim() === "") event.preventDefault();
        }}
      >
        <span role="status" class="flex items-center gap-tight">
          {#if handed}
            <Glyph name="check" size="sm" />
            {fill(say($lang, "talk_handed"), { room: here ?? "" })}
          {:else}
            {say($lang, SPELLING[sending])}
          {/if}
        </span>
      </button>
    </div>
  </div>
  {#if text !== ""}
    <!-- What two keys do, drawn only while there is something to send (ux A3). -->
    <p class="mt-tight text-note text-text-faint">{say($lang, "talk_enter_hint")}</p>
  {/if}
</form>
