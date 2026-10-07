<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // Where the stretch now open began, and the earlier stretch of the
  // room folded above it (ux B7). A branch's rule says which turn of
  // which conversation it came from and leads back to that conversation,
  // which stays as it was: the act a person reads here is "a new line
  // from one turn, the original untouched" (client D44).
  import { fill, say } from "../../core/lang";
  import { toFragment } from "../../core/route";
  import { clock } from "../../core/time";
  import type { RunBelief } from "../../core/belief";
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
    // Everything before the boundary, oldest first. Folded to one line
    // and opened in place: this is the same conversation, earlier -
    // neither a dialog nor a route is the right weight for a scroll
    // (ux B7).
    readonly earlier: readonly RunBelief[];
    // How many runs the session now open holds, which decides whether
    // the earlier stretch starts open (client D80).
    readonly shown: number;
    readonly boundary: Boundary | null;
    readonly onFork: (plan: ForkPlan) => void;
    readonly onRetry: (task: string) => void;
  }

  const { earlier, shown, boundary, onFork, onRetry }: Props = $props();

  const { lang, conn } = ui();
  const belief = conn.belief;

  // The person's own choice, once they press the line; until then the
  // rule both modes share decides.
  let pressed = $state<boolean | null>(null);
  const open = $derived(pressed ?? earlierDrawn(shown, earlier.length) === "open");

  // The count is the folded runs themselves: what expands is one thread
  // per run, so the number a person reads is the number of things the
  // line is holding down.
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

  // The mother is looked up in the whole city, not only among the runs
  // folded above: a branch cut from a run outside this stretch still
  // names whom it came from, and its room's sessions say which stretch
  // the way back opens.
  const mother = $derived(boundary?.kind === "forked" ? $belief.runs[boundary.mother] : undefined);
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
      const original = originalOf(boundary.mother, mother, held !== undefined && "sessions" in held ? held.sessions.sessions : []);
      const when: RulePart[] = boundary.at === null ? [] : [{ kind: "text", text: clock($lang, boundary.at) }];
      const parts: RulePart[] = [
        {
          kind: "text",
          text: fill(say($lang, "session_forked_divider"), {
            turn: String(boundary.turn),
            at: (task === null ? null : motherName(task)) ?? say($lang, "fork_mother"),
          }),
        },
        ...when,
        { kind: "link", text: say($lang, "fork_back"), href: toFragment(original) },
      ];
      return { parts, mark: "branch", wire: {} };
    }
    // A boundary this page watched open knows its minute; one found
    // here after a reload says only that the stretch is new, which is
    // the part that is true in every case.
    if (boundary.at === null) return null;
    return {
      parts: [{ kind: "text", text: fill(say($lang, "session_new_divider"), { at: clock($lang, boundary.at) }) }],
      mark: "none",
      wire: {},
    };
  });
</script>

{#if earlier.length > 0}
  {#if open}
    <div class="fade">
      {#each earlier as run (run.run)}
        <Thread {run} {onFork} {onRetry} />
      {/each}
    </div>
  {/if}
  <div class="my-snug"><Rule {...folded} /></div>
{/if}
{#if told !== null}
  <div class="my-snug"><Rule {...told} /></div>
{/if}
