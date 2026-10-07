<script lang="ts" module>
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Following one run over the agent's shoulder: the files it changed on
// one side and its terminal on the other (the workbench). The terminal
// folds away; folded, a column of the changed files takes its place as
// the way to jump around the code.
//
// Following is on when the page opens and lands on whatever the agent
// touched last - the file it edited or the call it made. The first
// wheel turn, touch drag or scrolling key in either pane pauses it,
// because a person who scrolls is reading, and a page that pulls the
// text away mid-sentence is fighting them. Those three inputs are what
// a person does and a scroll the page makes itself is not, which is why
// pausing listens to them and not to the `scroll` event. `F`, or the
// follow button, takes it up again; the key is heard page-wide because
// it only means something while this view is on the page.
//
// This file is the seat (client D95): it holds the two states and the
// two panes, and `./monitor.ts` turns them into the value the look
// (`monitor.look.svelte`) draws.
</script>

<script lang="ts">
  import { ui } from "../../ui";
  import type { Turn } from "../../wire";
  import CodeColumn from "./code_column.svelte";
  import Look from "./monitor.look.svelte";
  import Terminal from "./terminal.svelte";
  import type { Tail } from "../../core/live_output";
  import { lookOf, resumes } from "./monitor";
  import type { Following, Pane } from "./monitor";
  import { traceOf } from "./trace";
  import type { Target } from "./trace";

  interface Props {
    readonly turns: readonly Turn[];
    // What the running command has written so far; `NO_TAIL` when none runs.
    readonly tail: Tail;
    // Whether the run still takes steers; a finished run refuses them.
    readonly live: boolean;
    readonly onDraft: (text: string) => void;
    readonly onSteer: (text: string) => void;
    // How the monitor opens; the gallery draws the folded and paused
    // states from these, and the page takes the defaults.
    readonly following?: Following;
    readonly terminal?: Pane;
  }

  const { turns, tail, live, onDraft, onSteer, following: openFollowing = "following", terminal: openTerminal = "open" }: Props = $props();
  const { lang } = ui();

  // svelte-ignore state_referenced_locally (the props say how the monitor opens; the person moves both after that)
  let following = $state<Following>(openFollowing);
  // svelte-ignore state_referenced_locally (as above)
  let terminal = $state<Pane>(openTerminal);
  let code = $state<HTMLElement | undefined>(undefined);
  let record = $state<HTMLElement | undefined>(undefined);

  const trace = $derived(traceOf(turns));

  $effect(() => {
    if (following === "following" && trace.latest !== null) land(trace.latest);
  });

  // Each pane scrolls itself and nothing around it: `scrollIntoView`
  // would also move every scrolling ancestor, and the page a monitor
  // sits in is not the agent's to scroll. The panes are `relative`, so
  // an offset inside one is measured from its own top.
  function land(target: Target): void {
    switch (target.kind) {
      case "file": {
        const file = code?.querySelector<HTMLElement>(`[data-path="${CSS.escape(target.path)}"]`);
        if (code !== undefined && file != null) code.scrollTop = file.offsetTop;
        return;
      }
      case "entry": {
        const entry = record?.querySelector<HTMLElement>(`[data-at="${String(target.at)}"]`);
        if (record !== undefined && entry != null) record.scrollTop = entry.offsetTop + entry.offsetHeight - record.clientHeight;
        return;
      }
    }
  }

  // The same two attachments on every draw, so a redraw keeps its panes.
  const holdCode = (node: HTMLElement): (() => void) => {
    code = node;
    return () => {
      code = undefined;
    };
  };
  const holdRecord = (node: HTMLElement): (() => void) => {
    record = node;
    return () => {
      record = undefined;
    };
  };

  const look = $derived(
    lookOf({ following, terminal, paths: trace.files.map((file) => file.path) }, $lang, {
      follow: (next) => {
        following = next;
      },
      fold: (next) => {
        terminal = next;
      },
      jump: (path) => {
        following = "paused";
        land({ kind: "file", path });
      },
      holdCode,
      holdRecord,
    }),
  );

  function resumeBy(event: KeyboardEvent): void {
    const typing = event.target instanceof HTMLInputElement || event.target instanceof HTMLTextAreaElement;
    if (resumes({ key: event.key, ctrlKey: event.ctrlKey, metaKey: event.metaKey, altKey: event.altKey, typing })) following = "following";
  }
</script>

<svelte:window onkeydown={resumeBy} />

{#snippet column()}
  <CodeColumn files={trace.files} {live} {onDraft} {onSteer} />
{/snippet}

{#snippet printed()}
  <Terminal entries={trace.entries} {tail} />
{/snippet}

<Look {...look} {column} {printed} />
