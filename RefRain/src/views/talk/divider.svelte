<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  import { fill, say } from "../../core/lang";
  import { toFragment } from "../../core/route";
  import { clock } from "../../core/time";
  import type { RunBelief } from "../../core/belief";
  import type { RoundsAnswer, Seq } from "../../wire";
  import { readable } from "svelte/store";
  import { ui } from "../../ui";
  import { foldWire } from "./fold";
  import { motherName, originalOf } from "./forking";
  import type { Boundary, ForkPlan } from "./forking";
  import { earlierDrawn } from "./earlier";
  import type { RuleLook, RulePart } from "./rule";
  import Rule from "./rule.look.svelte";
  import Thread from "./thread.svelte";

  interface Props {
    readonly session?: Seq | null;
    readonly earlier: readonly RunBelief[];
    readonly shown: number;
    readonly boundary: Boundary | null;
    readonly onFork: (plan: ForkPlan) => void;
    readonly onRetry: (task: string) => void;
  }

  const { earlier, shown, boundary, onFork, onRetry, session = null }: Props = $props();

  const { lang, conn } = ui();
  const belief = conn.belief;

  let pressed = $state<boolean | null>(null);
  let previousSession: Seq | null | undefined;
  $effect(() => {
    if (session === previousSession) return;
    previousSession = session;
    pressed = null;
  });
  const drawn = $derived(earlierDrawn(shown, earlier.length, boundary?.kind === "forked"));
  const open = $derived(pressed ?? drawn === "open");

  const folded = $derived<RuleLook>({
    parts: [
      {
        kind: "fold",
        fold: {
          label: `${fill(say($lang, "session_previous"), { n: String(earlier.length) })} · ${say($lang, open ? "session_collapse" : "session_expand")}`,
          wire: foldWire(open, () => {
            pressed = !open;
          }),
        },
      },
    ],
    mark: "none",
    wire: {},
  });

  const mother = $derived(boundary?.kind === "forked" ? $belief.runs[boundary.mother] : undefined);
  let motherRounds = $state<RoundsAnswer | undefined>(undefined);
  $effect(() => {
    if (boundary?.kind !== "forked" || boundary.turn !== null) return;
    return conn.asking.ask({ rounds: { run: boundary.mother } }).subscribe((answer) => {
      motherRounds = answer !== undefined && "rounds" in answer ? answer.rounds : undefined;
    });
  });
  const turn = $derived.by(() => {
    if (boundary?.kind !== "forked") return 0;
    if (boundary.turn !== null) return boundary.turn;
    return (
      [...(motherRounds?.turns ?? [])]
        .reverse()
        .find((each) => each.opened <= boundary.origin.at_seq)?.number ?? 0
    );
  });
  const lines = $derived(
    mother?.addr === null || mother?.addr === undefined
      ? readable(undefined)
      : conn.asking.ask({ sessions: { room: mother.addr } }),
  );

  const told = $derived.by((): RuleLook | null => {
    if (boundary === null) return null;
    if (boundary.kind === "forked") {
      const task = mother?.task ?? null;
      const held = $lines;
      const original = originalOf(
        boundary.mother,
        mother,
        held !== undefined && "sessions" in held ? held.sessions.sessions : [],
      );
      const when: RulePart[] =
        boundary.at === null ? [] : [{ kind: "text", text: clock($lang, boundary.at) }];
      const parts: RulePart[] = [
        {
          kind: "text",
          text: fill(say($lang, "session_forked_divider"), {
            turn: String(turn),
            at: (task === null ? null : motherName(task)) ?? say($lang, "fork_mother"),
          }),
        },
        ...when,
        { kind: "link", text: say($lang, "fork_back"), href: toFragment(original) },
      ];
      return { parts, mark: "branch", wire: {} };
    }
    if (boundary.at === null) return null;
    return {
      parts: [
        {
          kind: "text",
          text: fill(say($lang, "session_new_divider"), { at: clock($lang, boundary.at) }),
        },
      ],
      mark: "none",
      wire: {},
    };
  });
</script>

{#if drawn !== "none"}
  {#if open}
    <div class="fade">
      {#each earlier as run (run.run)}
        <Thread
          {run}
          through={boundary?.kind === "forked" && boundary.mother === run.run
            ? boundary.origin.at_seq
            : null}
          {onFork}
          {onRetry}
        />
      {/each}
    </div>
  {/if}
  <div class="my-snug"><Rule {...folded} /></div>
{/if}
{#if told !== null}
  <div class="my-snug"><Rule {...told} /></div>
{/if}
