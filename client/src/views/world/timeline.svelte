<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // The chosen session's timeline (client-SPEC 7K): every turn, every
  // call and every checkpoint, in the order the Ledger wrote them, each
  // at the instant the Ledger gave it to the millisecond. The date and
  // the zone are said once, in the head, so a row carries only the time
  // of day; a turn says how soon the model answered and what it read and
  // wrote, a call how long it took and - a command - the code it exited
  // with, and a checkpoint which commit it is.
  //
  // **It is redrawn on Ledger events and nothing else**: every row is
  // read from the session's rounds, which a streamed token does not
  // change, so a long reply does not repaint the timeline under it.
  //
  // A call opens on the right side through the one door every opener
  // uses (`inspect/open.svelte.ts`); a checkpoint picks its commit, which
  // the commits pane marks and opens.
  import { tick } from "svelte";

  import { fill, say } from "../../core/lang";
  import type { Key } from "../../core/lang";
  import { isoDay, isoTime, kilo } from "../../core/time";
  import { ui } from "../../ui";
  import type { Call, CommitAnswer, GitOid, RunId, Turn } from "../../wire";
  import { openCall } from "../inspect/open.svelte";
  import Produced from "../talk/produced.svelte";
  import { pickCommit } from "./chosen.svelte";
  import { speedOf } from "./speed";

  interface Props {
    readonly run: RunId | null;
    readonly turns: readonly Turn[];
    // The commits the page holds, for a checkpoint's time and parent.
    readonly commits: readonly CommitAnswer[];
    readonly picked: GitOid | null;
  }

  const { run, turns, commits, picked }: Props = $props();
  const { lang } = ui();

  type Row =
    | { readonly kind: "turn"; readonly key: string; readonly at: number; readonly turn: Turn }
    | { readonly kind: "call"; readonly key: string; readonly at: number | null; readonly call: Call }
    | {
        readonly kind: "checkpoint";
        readonly key: string;
        readonly at: number | null;
        readonly oid: GitOid;
        readonly commit: CommitAnswer | undefined;
      };

  // A turn, then what happened inside it in Ledger order: its calls and
  // its checkpoints sorted together by the sequence that wrote them.
  const rows = $derived(
    turns.flatMap((turn): Row[] => {
      const inside = [
        ...turn.calls.map((call) => ({ seq: call.at, row: callRow(call) })),
        ...turn.notes.flatMap((note) =>
          "checkpointed" in note ? [{ seq: note.checkpointed.at, row: checkpointRow(note.checkpointed.oid) }] : [],
        ),
      ].sort((a, b) => a.seq - b.seq);
      return [{ kind: "turn", key: `t${String(turn.opened)}`, at: turn.t, turn }, ...inside.map((each) => each.row)];
    }),
  );

  function callRow(call: Call): Row {
    return { kind: "call", key: `c${String(call.at)}`, at: call.answered ?? null, call };
  }

  function checkpointRow(oid: GitOid): Row {
    const commit = commits.find((each) => each.oid === oid);
    return { kind: "checkpoint", key: `k${oid}`, at: commit?.at ?? null, oid, commit };
  }

  // The session's day, said once: the first turn's date in UTC. Where a
  // row falls on a later day - a session that ran past midnight - the new
  // date is said once above it, so no time of day reads as the head's day
  // when it is not.
  const day = $derived(turns[0] === undefined ? "" : isoDay(turns[0].t));
  const dayBreaks = $derived(
    rows.reduce<{ readonly last: string; readonly said: readonly string[] }>(
      (held, row) => {
        const on = row.at === null ? held.last : isoDay(row.at);
        return { last: on, said: [...held.said, on === held.last ? "" : on] };
      },
      { last: day, said: [] },
    ).said,
  );

  function instant(at: number | null): string {
    return at === null ? "—" : isoTime(at);
  }

  // How long a call took, when both of its times are measurements: in
  // milliseconds under a second, in seconds to the millisecond above.
  function took(call: Call): string {
    if (call.timing !== "measured" || call.answered === undefined || call.answered === null) return "";
    const ms = call.answered - call.called;
    return ms < 1_000
      ? fill(say($lang, "world_ms"), { n: String(ms) })
      : fill(say($lang, "world_s"), { n: (ms / 1_000).toFixed(3) });
  }

  const OUTCOME: Record<Call["outcome"], Key | null> = { waiting: "world_running", answered: null, failed: "results_failed" };

  // A picked commit brings its checkpoint into view.
  let list = $state<HTMLOListElement | undefined>(undefined);
  $effect(() => {
    const oid = picked;
    if (oid === null || list === undefined) return;
    const box = list;
    void tick().then(() => {
      box.querySelector(`[data-oid="${oid}"]`)?.scrollIntoView({ block: "nearest" });
    });
  });

  // One row on one line where the pane is wide enough for the four
  // cells, and on two where it is not: the instant and the measure above,
  // the kind and the subject under them. A cell never wraps inside itself.
  const ROW =
    "grid grid-cols-[8ch_minmax(0,1fr)_auto] items-center gap-x-base rounded-card px-snug -mx-snug py-tight text-note whitespace-nowrap @min-[400px]:h-control @min-[400px]:grid-cols-[14ch_8ch_minmax(0,1fr)_auto] @min-[400px]:py-0";
  const AT = "figure col-span-2 text-text-faint @min-[400px]:col-span-1";
  const KIND = "row-start-2 col-start-1 truncate @min-[400px]:row-start-1 @min-[400px]:col-start-2";
  const SUBJECT = "row-start-2 col-start-2 col-span-2 truncate @min-[400px]:row-start-1 @min-[400px]:col-start-3 @min-[400px]:col-span-1";
  const MEASURE = "figure row-start-1 col-start-3 justify-self-end text-text-faint @min-[400px]:col-start-4";
</script>

<section class="@container flex min-h-0 flex-1 flex-col pt-snug" aria-label={say($lang, "world_timeline")}>
  <h3 class="flex shrink-0 justify-between py-snug text-note text-text-faint">
    <span>{say($lang, "world_timeline")}</span>
    {#if day !== ""}<span class="figure">{fill(say($lang, "world_timeline_day"), { day })}</span>{/if}
  </h3>
  <ol bind:this={list} class="min-h-0 flex-1 overflow-y-auto [mask-image:linear-gradient(to_bottom,black_calc(100%_-_48px),transparent)]">
    {#each rows as row, index (row.key)}
      <li>
        {#if (dayBreaks[index] ?? "") !== ""}
          <p class="figure mt-snug py-tight text-note text-text-faint">{fill(say($lang, "world_timeline_day"), { day: dayBreaks[index] ?? "" })}</p>
        {/if}
        {#if row.kind === "turn"}
          {@const speed = speedOf([row.turn])}
          <div class="{ROW} mt-snug text-text">
            <span class={AT}>{instant(row.at)}</span>
            <span class="{KIND} font-label">{fill(say($lang, "run_turn_n"), { n: String(row.turn.number) })}</span>
            <span class="{SUBJECT} text-text-quiet">
              {speed === null ? "" : fill(say($lang, "world_ttft"), { n: String(speed.ttft) })}
            </span>
            <span class={MEASURE}>
              {row.turn.used === undefined || row.turn.used === null
                ? ""
                : fill(say($lang, "world_tokens_io"), { input: kilo(row.turn.used.input), output: kilo(row.turn.used.output) })}
            </span>
          </div>
        {:else if row.kind === "call"}
          {@const outcome = OUTCOME[row.call.outcome]}
          {@const at = row.call.at}
          <button
            type="button"
            class="{ROW} w-full text-left text-text-quiet hover:wash"
            disabled={run === null}
            onclick={() => {
              if (run !== null) openCall({ run, at });
            }}
          >
            <span class={AT}>{instant(row.at)}</span>
            <span class="{KIND} text-text-faint">{row.call.tool}</span>
            <span class={SUBJECT}>{row.call.subject ?? ""}</span>
            <span class="{MEASURE} flex items-center gap-snug">
              {#if row.call.outcome === "waiting"}
                <span class="size-dot animate-pulse rounded-pill bg-accent" aria-hidden="true"></span>
              {/if}
              {took(row.call)}
              {#if row.call.exit_code !== undefined && row.call.exit_code !== null}
                <span class={row.call.exit_code === 0 ? "text-accent" : "text-alert"}>
                  {fill(say($lang, "mon_exited"), { code: String(row.call.exit_code) })}
                </span>
              {/if}
              {#if outcome !== null}<span class={row.call.outcome === "failed" ? "text-alert" : ""}>{say($lang, outcome)}</span>{/if}
            </span>
          </button>
        {:else if row.kind === "checkpoint"}
          {@const commit = row.commit}
          {@const parent = commit?.parents?.[0]}
          <button
            type="button"
            data-oid={row.oid}
            class={[
              ROW,
              "relative w-full text-left text-text",
              row.oid === picked ? "wash-strong" : "hover:wash",
            ]}
            aria-current={row.oid === picked ? "true" : undefined}
            disabled={commit === undefined}
            onclick={() => {
              if (commit !== undefined) pickCommit(commit);
            }}
          >
            {#if row.oid === picked}
              <span class="absolute top-snug bottom-snug left-0 w-hair rounded-pill bg-accent" aria-hidden="true"></span>
            {/if}
            <span class={AT}>{instant(row.at)}</span>
            <span class="{KIND} text-accent">{say($lang, "world_checkpoint")}</span>
            <span class="{SUBJECT} figure">{row.oid.slice(0, 7)}</span>
            <span class={MEASURE}>
              {#if parent !== undefined}<Produced base={parent} head={row.oid} />{/if}
            </span>
          </button>
        {/if}
      </li>
    {/each}
  </ol>
</section>

