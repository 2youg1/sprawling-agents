<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // The room a person works in: the box they write in, the words a
  // model is still saying, the two lists that open over that box, and
  // the questions the city is holding for them - plus the session and
  // fork shapes a thread gains when one room runs more than once: one
  // round, the divider that folds a previous segment away, the fork
  // affordance over a message, and the picker a bare `/fork` opens.
  //
  // Two of these need room the page gives them and a fixture does not.
  // The list that opens over the composer opens *upward*, so the
  // padding above it is the room the foot of a page has; the sheet the
  // questions sit in is drawn where the page draws it.
  //
  // The session and fork shapes are drawn here rather than through
  // `talk/` because those modules do not exist yet: the divider and
  // the affordance are what they will draw, frozen at the state worth
  // looking at, and the picker is the real multi-column `parts`
  // popover they are specified to walk (roadmap S2).

  import type { Doing } from "../../core/doing";
  import { sendingInto } from "../../core/doing";
  import { EFFORTS } from "../../core/commands";
  import { fill, say } from "../../core/lang";
  import { toFragment } from "../../core/route";
  import { UNSTATED, offered } from "../../core/slash";
  import { RunId } from "../../wire";
  import { motherName } from "../talk/forking";
  import type { Utterance } from "./served";
  import { CALLS, EARLIER_SEGMENT, ONE_QUESTION, ROUND, TURNS, WAITING } from "./served";
  import { SAYING } from "./saying";

  // The question the rail's dot counts stays reachable under the name
  // it always had, for the presences fixture that reads it.
  export { ONE_QUESTION } from "./served";

  // The postures a run can be in, in the order a dispatch meets them.
  const POSTURES: readonly Doing[] = [
    { kind: "thinking" },
    { kind: "calling", tool: "exec", subject: "just check" },
    { kind: "waiting" },
    { kind: "frozen", completion: "done" },
    { kind: "unknown" },
  ];

  // The mother run the forked segment branches from: an id in the
  // shape a run id has, so the link under the divider is a link a
  // person can follow.
  const MOTHER = RunId.make("0199c0de-1a2b-4c3d-8e4f-5a6b7c8d9e0f");
  // What the mother run was asked: the divider names it by its first
  // sentence, never by the id above.
  const MOTHER_TASK = EARLIER_SEGMENT.find((line) => line.speaker === "person")?.text ?? "";
</script>

<script lang="ts">
  import { ui } from "../../ui";
  import Popover from "../parts/popover.svelte";
  import Composer from "../talk/composer.svelte";
  import Saying from "../talk/saying.svelte";
  import { WaitingCards } from "../talk/waiting.svelte";
  import Asked from "../talk/asked.svelte";
  import Case from "./case.svelte";
  import { CHOSEN, MODELS } from "./served";

  const { lang } = ui();
  const uid = $props.id();
  // The previous segment is folded away until somebody asks for it.
  let open = $state(false);
</script>

{#snippet utterance(line: Utterance)}
  {#if line.speaker === "person"}
    <div class="my-base flex flex-col items-end">
      <div class="max-w-[83%] rounded-panel bg-speech px-pane py-base text-body leading-relaxed whitespace-pre-wrap">{line.text}</div>
      <div class="mt-tight text-note text-text-disabled">{say($lang, "talk_you")}</div>
    </div>
  {:else if line.speaker === "resident"}
    <div class="my-base text-body">
      <div class="mb-tight text-note text-text-disabled">{say($lang, "talk_resident")}</div>
      <div class="whitespace-pre-wrap leading-relaxed">{line.text}</div>
    </div>
  {/if}
{/snippet}

{#snippet divider(word: string)}
  <div class="my-wide flex items-center gap-base text-note text-text-disabled">
    <span class="h-px flex-1 bg-raised"></span>
    <span>{word}</span>
    <span class="h-px flex-1 bg-raised"></span>
  </div>
{/snippet}

{#snippet forkHere(shown: boolean)}
  <button
    type="button"
    class={[
      "absolute end-0 top-0 rounded-control px-tight text-note text-text-faint hover:bg-chrome hover:text-text-quiet",
      shown ? "" : "opacity-0 group-hover:opacity-100 group-focus-within:opacity-100",
    ]}
    onclick={() => undefined}
  >
    <!-- wording-ok: a branch mark, not a word -->
    <span aria-hidden="true">⑂</span>
    {say($lang, "fork_here")}
  </button>
{/snippet}

{#each POSTURES as doing (doing.kind)}
  <Case label={`${doing.kind} · ${sendingInto(doing)}`}>
    <Composer
      placeholder={say($lang, "talk_placeholder_mayor")}
      sending={sendingInto(doing)}
      hearing={doing.kind === "thinking"}
      onSend={() => false}
      onStop={() => false}
    />
  </Case>
{/each}

{#each SAYING as text (text)}
  <Case label={`saying · ${String(text.length)}`}>
    <Saying {text} who={say($lang, "talk_resident")} />
  </Case>
{/each}

<Case label="waiting">
  <WaitingCards items={WAITING} />
</Case>

<Case label="waiting · what the question is about, on the card">
  <Asked
    locator={ONE_QUESTION.artifact}
    content={{ binary: false, bytes: 96, locator: ONE_QUESTION.artifact, text: "rm -rf target\n# frees 4.2 GiB; the next build starts cold", truncated: false }}
  />
</Case>

<!-- The empty room lifts the box into the upper third rather than
centring it: two thousand pixels of white under a composer in a room
with one round in it reads as a page that broke. -->
<Case label="composer · an empty room lifts the box">
  <div class="flex flex-col items-center gap-base pt-[18vh] text-center">
    <p class="text-heading font-heading text-text-disabled">{say($lang, "talk_empty_mayor")}</p>
    <p class="text-note text-text-faint">{say($lang, "talk_opening_mayor")}</p>
    <div class="w-full">
      <Composer
        placeholder={say($lang, "talk_placeholder_mayor")}
        sending="dispatch"
        onSend={() => false}
        onStop={() => false}
      />
    </div>
  </div>
</Case>

<Case label="composer · docked once the room has a thread">
  <div class="px-pane pb-pane">
    <Composer
      placeholder={say($lang, "talk_placeholder_mayor")}
      sending="dispatch"
      hearing
      onSend={() => false}
      onStop={() => false}
    />
  </div>
</Case>

<!-- The two lists that open over the composer, each in the two layers
of padding that stand in for the page under them. -->

<Case label="menu · a line that begins with a slash">
  <div class="pt-palette">
    <div class="pt-output">
      <div class="relative">
        <Popover
          label="talk_commands"
          columns={[
            {
              id: "commands",
              label: "talk_commands",
              rows: offered("/").map((each) => ({
                id: each.spelling,
                label: each.spelling,
                secondary:
                  each.grammar === ""
                    ? say($lang, each.about)
                    : `${each.grammar} · ${say($lang, each.about)}`,
              })),
            },
          ]}
          onApply={() => undefined}
          onClose={() => undefined}
        />
      </div>
    </div>
  </div>
</Case>

<Case label="selector · model, workspace and effort in one list">
  <div class="pt-palette">
    <div class="pt-output">
      <div class="relative">
        <Popover
          label="talk_choose"
          columns={[
            {
              id: "model",
              label: "talk_column_model",
              rows: MODELS.map((each) => ({
                id: each.id,
                label: each.id,
                secondary: "zenmux",
                chosen: each.id === CHOSEN.id,
              })),
            },
            {
              id: "workspace",
              label: "talk_column_workspace",
              rows: [
                { id: "hall/mayor", label: "hall/mayor", chosen: true },
                { id: "lab/east", label: "lab/east" },
              ],
            },
            {
              id: "effort",
              label: "talk_column_effort",
              // The column the composer draws: nobody having chosen
              // leads it and is the row marked, because that is the
              // state a city nobody has told starts in, and each level
              // carries what it costs.
              rows: [UNSTATED, ...EFFORTS].map((level) => ({
                id: level,
                label: say($lang, `effort_${level}`),
                secondary: say($lang, `effort_note_${level}`),
                chosen: level === UNSTATED,
              })),
            },
          ]}
          onApply={() => undefined}
          onClose={() => undefined}
        />
      </div>
    </div>
  </div>
</Case>

<!-- One round of a room, with the box docked: the drop from the
middle of an empty room to the foot of a full one happens the first
time somebody sends, so both landings are on this page. -->
<Case label="talk · one round, and the box docked">
  <div class="flex min-h-output flex-col">
    <div class="flex-1">
      {#each ROUND as line (line.text)}
        <div class="relative">
          <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression (a snippet call is the render itself; the typechecker types local snippet calls as returning void) -->
          {@render utterance(line)}
          {#if line.speaker === "person"}
            <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression (a snippet call is the render itself; the typechecker types local snippet calls as returning void) -->
            {@render forkHere(false)}
          {/if}
        </div>
      {/each}
    </div>
    <div class="px-pane pb-pane">
      <Composer
        placeholder={say($lang, "talk_placeholder_mayor")}
        sending="dispatch"
        onSend={() => false}
        onStop={() => false}
      />
    </div>
  </div>
</Case>

<!-- A session divider folds the segment before it into one line and
keeps the fold one press away: the rounds are still the room's
history, and five of them is not what the person came back for. -->
<Case label="session divider · the previous segment folded away">
  <button
    type="button"
    class="flex items-center gap-tight rounded-control px-tight text-note text-text-faint hover:bg-chrome hover:text-text-quiet"
    aria-expanded={open}
    aria-controls="{uid}-earlier"
    onclick={() => {
      open = !open;
    }}
  >
    <!-- wording-ok: a fold triangle is a mark, not a word -->
    <span class="inline-block w-pane" aria-hidden="true">{open ? "▾" : "▸"}</span>
    <span>{fill(say($lang, "session_previous"), { n: "5" })}</span>
  </button>
  {#if open}
    <div id="{uid}-earlier" class="border-l border-edge-panel pl-base">
      {#each EARLIER_SEGMENT as line (line.text)}
        <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression (a snippet call is the render itself; the typechecker types local snippet calls as returning void) -->
        {@render utterance(line)}
      {/each}
    </div>
  {/if}
  <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression (a snippet call is the render itself; the typechecker types local snippet calls as returning void) -->
  {@render divider(fill(say($lang, "session_new_divider"), { at: "12:04" }))}
  {#each ROUND as line (line.text)}
    <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression (a snippet call is the render itself; the typechecker types local snippet calls as returning void) -->
    {@render utterance(line)}
  {/each}
</Case>

<!-- The forked reading of the same divider: which turn it branched
from, and the mother run a person can open to see where the words
came from. -->
<Case label="talk · forked from turn 3, with the mother run linked">
  <div class="my-wide flex items-center gap-base text-note text-text-disabled">
    <span class="h-px flex-1 bg-raised"></span>
    <a href={toFragment({ kind: "run", run: MOTHER })} class="hover:text-text-quiet">
      {fill(say($lang, "session_forked_divider"), { turn: "3", at: motherName(MOTHER_TASK) })} · 11:47
    </a>
    <span class="h-px flex-1 bg-raised"></span>
  </div>
  {#each ROUND as line (line.text)}
    <div class="relative">
      <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression (a snippet call is the render itself; the typechecker types local snippet calls as returning void) -->
      {@render utterance(line)}
      {#if line.speaker === "person"}
        <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression (a snippet call is the render itself; the typechecker types local snippet calls as returning void) -->
        {@render forkHere(false)}
      {/if}
    </div>
  {/each}
  <p class="mb-tight text-note text-text-faint" role="status">
    {fill(say($lang, "fork_pending"), { turn: "3" })}
  </p>
</Case>

<!-- The affordance in its revealed state, because a state that only
appears under a pointer is a state nobody judges: over any message it
arrives on hover or when anything inside the message holds focus, and
the keyboard reaches it in the message's own tab stop. -->
<Case label="message · fork from here, revealed over the message">
  <div>
    {#each ROUND as line (line.text)}
      <div class="relative">
        <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression (a snippet call is the render itself; the typechecker types local snippet calls as returning void) -->
        {@render utterance(line)}
        {#if line.speaker === "person"}
          <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression (a snippet call is the render itself; the typechecker types local snippet calls as returning void) -->
          {@render forkHere(true)}
        {/if}
      </div>
    {/each}
  </div>
</Case>

<!-- A bare `/fork` opens the picker: turns of this run down the left,
what the marked turn did down the right. The columns are the real
multi-column listbox, so the two-column reading is the shipped one. -->
<Case label="fork picker · open, turns and calls in two columns">
  <div class="pt-palette">
    <div class="pt-output">
      <div class="relative">
        <Popover
          label="fork_pick_title"
          columns={[
            {
              id: "turns",
              label: "run_turns",
              rows: TURNS.map((each) => ({
                id: each.id,
                label: fill(say($lang, "fork_from_turn"), { turn: each.turn }),
                secondary: each.at,
              })),
            },
            {
              id: "calls",
              label: "talk_calls",
              rows: CALLS.map((each) => ({ id: each.id, label: each.what, secondary: each.at })),
            },
          ]}
          onApply={() => undefined}
          onClose={() => undefined}
        />
      </div>
    </div>
  </div>
</Case>
