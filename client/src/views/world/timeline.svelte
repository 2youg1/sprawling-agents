<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // The chosen session's timeline (client/Spec.lean §7K): every turn, every
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
  //
  // This file is the seat: it scrolls a picked checkpoint into view and
  // opens and picks; `./timeline.ts` reads the rows, and
  // `./timeline.look.svelte` draws them.
  import { tick } from "svelte";

  import { ui } from "../../ui";
  import type { CommitAnswer, GitOid, RunId, Turn } from "../../wire";
  import { openCall } from "../inspect/open.svelte";
  import { pickCommit } from "./chosen.svelte";
  import { drawnElements } from "./drawn";
  import { timelineOf } from "./timeline";
  import type { TimelineLook } from "./timeline";
  import Look from "./timeline.look.svelte";

  interface Props {
    readonly run: RunId | null;
    readonly turns: readonly Turn[];
    // The commits the page holds, for a checkpoint's time and parent.
    readonly commits: readonly CommitAnswer[];
    readonly picked: GitOid | null;
  }

  const { run, turns, commits, picked }: Props = $props();
  const { lang } = ui();

  // The list the look drew, which a picked checkpoint is scrolled in.
  const drawn = drawnElements();

  // A picked commit brings its checkpoint into view.
  $effect(() => {
    const oid = picked;
    if (oid === null) return;
    void tick().then(() => {
      drawn.get("list")?.querySelector(`[data-oid="${oid}"]`)?.scrollIntoView({ block: "nearest" });
    });
  });

  const look: TimelineLook = $derived(
    timelineOf(
      { lang: $lang, turns, commits, picked },
      {
        open: run === null ? undefined : (at) => {
          openCall({ run, at });
        },
        pick: pickCommit,
        list: drawn.hold("list"),
      },
    ),
  );
</script>

<section class="@container flex min-h-0 flex-1 flex-col overflow-hidden pt-snug" aria-label={look.title}>
  <Look {...look} />
</section>
