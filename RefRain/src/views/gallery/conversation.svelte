<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  import type { Doing } from "../../core/doing";
  import { sendingInto } from "../../core/doing";
  import { fill, say } from "../../core/lang";
  import { toFragment } from "../../core/route";
  import { offered } from "../../core/slash";
  import type { Turn } from "../../wire";
  import { RunId, Seq, TimeMs } from "../../wire";
  import { motherName } from "../talk/forking";
  import type { Utterance } from "./served";
  import { CALLS, EARLIER_SEGMENT, ONE_QUESTION, ROUND, TURNS, WAITING } from "./served";
  import { SAYING } from "./saying";

  export { ONE_QUESTION } from "./served";

  const POSTURES: readonly Doing[] = [
    { kind: "thinking" },
    { kind: "calling", tool: "exec", subject: "just check" },
    { kind: "waiting" },
    { kind: "frozen", completion: "done" },
    { kind: "unknown" },
  ];

  const MOTHER = RunId.make("0199c0de-1a2b-4c3d-8e4f-5a6b7c8d9e0f");
  const MOTHER_TASK = EARLIER_SEGMENT.find((line) => line.speaker === "person")?.text ?? "";
  const LANDED: Turn = {
    calls: [],
    notes: [],
    number: 1,
    opened: Seq.make(1),
    t: TimeMs.make(1),
    timing: "measured",
  };
</script>

<script lang="ts">
  import { ui } from "../../ui";
  import Popover from "../parts/popover.svelte";
  import Composer from "../talk/composer.svelte";
  import ComposerLook from "../talk/composer.look.svelte";
  import type { ComposerLook as ComposerLookValue } from "../talk/composer";
  import Coin from "../talk/coin.svelte";
  import Gauge from "../talk/gauge.svelte";
  import SettingsRow from "../talk/settings_row.svelte";
  import TypedLine from "../talk/typed_line.svelte";
  import AddressLine from "../talk/address_line.look.svelte";
  import { addressLineOf, boxWords } from "../talk/address_line";
  import Saying from "../talk/saying.svelte";
  import { WaitingCards } from "../talk/waiting.svelte";
  import Asked from "../talk/asked.svelte";
  import Person from "../talk/person.svelte";
  import { foldWire } from "../talk/fold";
  import Rule from "../talk/rule.look.svelte";
  import Case from "./case.svelte";
  import { AT_DEEPSEEK, MANY_PROVIDERS, pickerFacts } from "./picked";

  const { lang } = ui();
  let open = $state(false);
  const picker = pickerFacts(MANY_PROVIDERS, AT_DEEPSEEK, null);
  const overlap: ComposerLookValue = $derived({
    form: { "aria-label": say($lang, "region_composer") },
    over: false,
    box: { rows: 1, "aria-label": say($lang, "talk_placeholder_mayor") },
    menu: undefined,
    door: inBox,
    band: undefined,
    keys: composerKeys,
    line: composerLine,
    under: openSettings,
  });
</script>

{#snippet utterance(line: Utterance)}
  {#if line.speaker === "person"}
    <div class="my-base flex flex-col items-end">
      <div
        class="max-w-[83%] rounded-panel bg-speech px-pane py-base text-body leading-read whitespace-pre-wrap"
      >
        {line.text}
      </div>
      <div class="mt-tight text-note text-text-faint">{say($lang, "talk_you")}</div>
    </div>
  {:else if line.speaker === "resident"}
    <div class="my-base text-body">
      <div class="mb-tight text-note text-text-faint">{say($lang, "talk_resident")}</div>
      <div class="whitespace-pre-wrap leading-read">{line.text}</div>
    </div>
  {/if}
{/snippet}

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
    content={{
      binary: false,
      bytes: 96,
      locator: ONE_QUESTION.artifact,
      text: "rm -rf target\n# frees 4.2 GiB; the next build starts cold",
      truncated: false,
    }}
  />
</Case>

{#snippet inBox()}
  <AddressLine
    {...addressLineOf($lang, "hall/mayor", null, () => undefined)}
    seat={{ at: "box", ...boxWords($lang) }}
  />
{/snippet}

{#snippet inBoxSession()}
  <AddressLine
    {...addressLineOf($lang, "hall/mayor", Seq.make(23), () => undefined)}
    seat={{ at: "box", ...boxWords($lang) }}
  />
{/snippet}

{#snippet composerKeys()}
  <Gauge room={null}>
    <Coin face="idle" sending="dispatch" onStop={() => undefined} />
  </Gauge>
{/snippet}

{#snippet composerLine()}
  <TypedLine text="" box={undefined} lit={false} />
{/snippet}

{#snippet openSettings()}
  <SettingsRow {picker} room={null} draws="everything" kept={false} menu="open" />
{/snippet}

<Case label="composer · an empty room lifts the box, the address line in its placeholder">
  <div class="flex flex-col items-center gap-base pt-[18vh] text-center">
    <div class="w-full">
      <Composer
        placeholder={say($lang, "talk_placeholder_mayor")}
        sending="dispatch"
        door={inBox}
        onSend={() => false}
        onStop={() => false}
      />
    </div>
  </div>
</Case>

<Case label="composer · permissions open over the address line">
  <div class="flex min-h-[14rem] flex-col justify-end">
    <ComposerLook {...overlap} />
  </div>
</Case>

<Case
  label="composer · docked once the room has a thread, the address and session in its placeholder"
>
  <div class="px-pane pb-pane">
    <Composer
      started
      placeholder={say($lang, "talk_placeholder_mayor")}
      sending="dispatch"
      door={inBoxSession}
      hearing
      onSend={() => false}
      onStop={() => false}
    />
  </div>
</Case>

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

<Case label="talk · one round, and the box docked">
  <div class="flex min-h-output flex-col">
    <div class="flex-1">
      {#each ROUND as line (line.text)}
        <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression (a snippet call is the render itself; the typechecker types local snippet calls as returning void) -->
        {@render branchable(line)}
      {/each}
    </div>
    <div class="px-pane pb-pane">
      <Composer
        started
        placeholder={say($lang, "talk_placeholder_mayor")}
        sending="dispatch"
        onSend={() => false}
        onStop={() => false}
      />
    </div>
  </div>
</Case>

<Case label="session divider · the previous segment folded away">
  {#if open}
    <div class="fade">
      {#each EARLIER_SEGMENT as line (line.text)}
        <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression (a snippet call is the render itself; the typechecker types local snippet calls as returning void) -->
        {@render utterance(line)}
      {/each}
    </div>
  {/if}
  <div class="my-snug flex flex-col gap-snug">
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
    <Rule
      parts={[{ kind: "text", text: fill(say($lang, "session_new_divider"), { at: "12:04" }) }]}
      mark="none"
      wire={{}}
    />
  </div>
  {#each ROUND as line (line.text)}
    <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression (a snippet call is the render itself; the typechecker types local snippet calls as returning void) -->
    {@render utterance(line)}
  {/each}
</Case>

<Case label="talk · forked from turn 3, with the mother run linked">
  <Rule
    parts={[
      {
        kind: "text",
        text: fill(say($lang, "session_forked_divider"), {
          turn: "3",
          at: motherName(MOTHER_TASK) ?? say($lang, "fork_mother"),
        }),
      },
      { kind: "text", text: "11:47" },
      {
        kind: "link",
        text: say($lang, "fork_back"),
        href: toFragment({ kind: "run", run: MOTHER }),
      },
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

<Case label="message · fork from here, revealed under the message">
  <div class="[&_[data-fork]]:opacity-100">
    {#each ROUND as line (line.text)}
      <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression (a snippet call is the render itself; the typechecker types local snippet calls as returning void) -->
      {@render branchable(line)}
    {/each}
  </div>
</Case>

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
