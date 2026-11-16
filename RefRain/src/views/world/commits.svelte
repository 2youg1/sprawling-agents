

<script lang="ts">
  
  
  import { readAnswer } from "../../core/answered";
  import { fill, say } from "../../core/lang";
  import { MAYOR, buildingOf, roomOf, toFragment } from "../../core/route";
  import { ui } from "../../ui";
  import type { Address, CommitAnswer } from "../../wire";
  import { openChanges } from "../inspect/open.svelte";
  import Unanswered from "../parts/unanswered.svelte";
  import { phaseOf } from "../runs/lineage";
  import { commitsIn, pickCommit, pickedCommit, sessionRun } from "./chosen.svelte";
  import { HEIGHT, rowOf, widthOf } from "./commits";
  import type { CommitsLook } from "./commits";
  import Look from "./commits.look.svelte";
  import { graphOf } from "./lanes";

  interface Props {
    readonly here: Address;
  }

  const { here }: Props = $props();

  const u = ui();
  const { lang } = u;
  const belief = u.conn.belief;

  const asked = $derived(commitsIn(here));
  const answer = $derived(u.conn.asking.ask(asked));
  const read = $derived(
    readAnswer($answer, (held) => ("commits" in held ? held.commits : undefined)),
  );
  const commits = $derived(read.kind === "held" ? read.value.commits : []);
  const graph = $derived(graphOf(commits));

  const session = $derived(sessionRun($belief, here));
  const picked = $derived(pickedCommit(here));

  function pick(commit: CommitAnswer): void {
    pickCommit(commit);
    const parent = commit.parents?.[0];
    if (parent !== undefined) openChanges({ base: parent, head: commit.oid });
    if (commit.actor !== here) u.go({ kind: "talk", address: commit.actor });
  }

  const look: CommitsLook = $derived({
    width: widthOf(graph),
    height: HEIGHT,
    rows: commits.map((commit, row) => {
      const doing = $belief.runs[commit.run]?.doing;
      return rowOf({
        commit,
        graph,
        row,
        picked: commit.oid === picked,
        here: commit.actor === here,
        phase: doing === undefined ? "done" : phaseOf(doing),
        mine: (at) => commits[at]?.run === session,
        room: roomOf(commit.actor),
        pick,
      });
    }),
    labels: {
      oid: say($lang, "world_commit_oid"),
      b3: say($lang, "world_commit_b3"),
      parents: say($lang, "world_parents"),
      at: say($lang, "world_commit_at"),
    },
    more:
      read.kind === "held" && read.value.more && here !== MAYOR
        ? {
            href: toFragment({ kind: "building", address: buildingOf(here) }),
            word: fill(say($lang, "world_commits_more"), { building: buildingOf(here) }),
          }
        : undefined,
  });
</script>

<div class="min-h-0 flex-1 overflow-y-auto">
  {#if read.kind === "unavailable"}
    <Unanswered query={read.query} {asked} />
  {:else if read.kind === "held" && commits.length === 0}
    <p class="py-snug text-note text-text-faint">{say($lang, "world_no_commits")}</p>
  {:else}
    <Look {...look} />
  {/if}
</div>
