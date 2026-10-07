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
  // button on a message, and the picker a bare `/fork` opens.
  //
  // Two of these need room the page gives them and a fixture does not.
  // The list that opens over the composer opens *upward*, so the
  // padding above it is the room the foot of a page has; the sheet the
  // questions sit in is drawn where the page draws it.
  //
  // The divider is drawn here, frozen at the state worth looking at;
  // a person's words are the real `talk/person.svelte` with its real
  // fork button, and the picker is the real multi-column `parts`
  // popover (roadmap S2).

  import type { Doing } from "../../core/doing";
  import { sendingInto } from "../../core/doing";
  import { EFFORTS } from "../../core/commands";
  import { fill, say } from "../../core/lang";
  import { toFragment } from "../../core/route";
  import { UNSTATED, offered } from "../../core/slash";
  import type { Turn } from "../../wire";
  import { RunId, Seq, TimeMs } from "../../wire";
  import { motherName } from "../talk/forking";
  import type { Utterance } from "./served";
  import { CALLS, EARLIER_SEGMENT, ONE_QUESTION, ROUND, TURNS, WAITING } from "./served";
  import { SAYING } from "./saying";

  // The question the mailbox key's badge counts stays reachable under
  // the name it always had, for the mailbox fixture that reads it.
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
  // The turn a person's words in these fixtures arrived in: a fork from
  // a message cuts before the turn it landed in.
  const LANDED: Turn = { calls: [], notes: [], number: 1, opened: Seq.make(1), t: TimeMs.make(1), timing: "measured" };
</script>

<script lang="ts">
  import { ui } from "../../ui";
  import Popover from "../parts/popover.svelte";
  import Composer from "../talk/composer.svelte";
  import Saying from "../talk/saying.svelte";
  import { WaitingCards } from "../talk/waiting.svelte";
  import Asked from "../talk/asked.svelte";
  import Person from "../talk/person.svelte";
  import { foldWire } from "../talk/fold";
  import Rule from "../talk/rule.look.svelte";
  import Case from "./case.svelte";
  import { CHOSEN, MODELS } from "./served";

  const { lang } = ui();
  // The previous segment is folded away until somebody asks for it.
  let open = $state(false);
</script>

{#snippet utterance(line: Utterance)}
  {#if line.speaker === "person"}
    <div class="my-base flex flex-col items-end">
      <div class="max-w-[83%] rounded-panel bg-speech px-pane py-base text-body leading-relaxed whitespace-pre-wrap">{line.text}</div>
      <div class="mt-tight text-note text-text-faint">{say($lang, "talk_you")}</div>
    </div>
  {:else if line.speaker === "resident"}
    <div class="my-base text-body">
      <div class="mb-tight text-note text-text-faint">{say($lang, "talk_resident")}</div>
      <div class="whitespace-pre-wrap leading-relaxed">{line.text}</div>
    </div>
  {/if}
{/snippet}

<!-- A line of a thread that can be branched from: a person's words are
the real message with its real fork button, revealed on hover or focus
like the thread's own. -->
{#snippet branchable(line: Utterance)}
  {#if line.speaker === "person"}
    <Person
      text={line.text}
      label={say($lang, "talk_you")}
      at={undefined}
      entry={{ kind: "message", turn: LANDED, text: line.text }}
      run={MOTHER}
      onFork={() => undefined}
      onHover={() => undefined}
    />
  {:else}
    <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression (a snippet call is the render itself; the typechecker types local snippet calls as returning void) -->
    {@render utterance(line)}
  {/if}
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
    <Composer started
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
        <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression (a snippet call is the render itself; the typechecker types local snippet calls as returning void) -->
        {@render branchable(line)}
      {/each}
    </div>
    <div class="px-pane pb-pane">
      <Composer started
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
  {#if open}
    <div class="fade">
      {#each EARLIER_SEGMENT as line (line.text)}
        <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression (a snippet call is the render itself; the typechecker types local snippet calls as returning void) -->
        {@render utterance(line)}
      {/each}
    </div>
  {/if}
  <Rule
    parts={[
      {
        kind: "fold",
        fold: {
          label: `${fill(say($lang, "session_previous"), { n: "5" })} · ${say($lang, open ? "session_collapse" : "session_expand")}`,
          wire: foldWire(open, () => {
            open = !open;
          }),
        },
      },
    ]}
    mark="none"
    wire={{}}
  />
  <Rule parts={[{ kind: "text", text: fill(say($lang, "session_new_divider"), { at: "12:04" }) }]} mark="none" wire={{}} />
  {#each ROUND as line (line.text)}
    <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression (a snippet call is the render itself; the typechecker types local snippet calls as returning void) -->
    {@render utterance(line)}
  {/each}
</Case>

<!-- The branched reading of the same rule: which turn of which
conversation the branch came from, and the way back to that
conversation, which stays as it was. -->
<Case label="talk · forked from turn 3, with the mother run linked">
  <Rule
    parts={[
      { kind: "text", text: fill(say($lang, "session_forked_divider"), { turn: "3", at: motherName(MOTHER_TASK) ?? say($lang, "fork_mother") }) },
      { kind: "text", text: "11:47" },
      { kind: "link", text: say($lang, "fork_back"), href: toFragment({ kind: "run", run: MOTHER }) },
    ]}
    mark="branch"
    wire={{}}
  />
  {#each ROUND as line (line.text)}
    <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression (a snippet call is the render itself; the typechecker types local snippet calls as returning void) -->
    {@render branchable(line)}
  {/each}
  <p class="mb-tight text-note text-text-faint" role="status">
    {fill(say($lang, "fork_pending"), { turn: "3" })}
  </p>
</Case>

<!-- The fork button in its revealed state, because a state that only
appears under a pointer is a state nobody judges: on any message it
arrives on hover or when anything inside the message holds focus, at
the end of the line under the words, and the keyboard reaches it in
the message's own tab stop. -->
<Case label="message · fork from here, revealed under the message">
  <div class="[&_[data-fork]]:opacity-100">
    {#each ROUND as line (line.text)}
      <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression (a snippet call is the render itself; the typechecker types local snippet calls as returning void) -->
      {@render branchable(line)}
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
