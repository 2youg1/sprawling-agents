<script lang="ts" module>
  // This Source Code Form is subject to the terms of the Mozilla Public
  // License, v. 2.0. If a copy of the MPL was not distributed with this
  // file, You can obtain one at https://mozilla.org/MPL/2.0/.
  // Copyright (c) 2026 2youg1 and the sprawling contributors
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
    readonly tail: Tail;
    readonly live: boolean;
    readonly onDraft: (text: string) => void;
    readonly onSteer: (text: string) => void;
    readonly following?: Following;
    readonly terminal?: Pane;
  }

  const {
    turns,
    tail,
    live,
    onDraft,
    onSteer,
    following: openFollowing = "following",
    terminal: openTerminal = "open",
  }: Props = $props();
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

  function land(target: Target): void {
    switch (target.kind) {
      case "file": {
        const file = code?.querySelector<HTMLElement>(`[data-path="${CSS.escape(target.path)}"]`);
        if (code !== undefined && file != null) code.scrollTop = file.offsetTop;
        return;
      }
      case "entry": {
        const entry = record?.querySelector<HTMLElement>(`[data-at="${String(target.at)}"]`);
        if (record !== undefined && entry != null)
          record.scrollTop = entry.offsetTop + entry.offsetHeight - record.clientHeight;
        return;
      }
    }
  }

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
    const typing =
      event.target instanceof HTMLInputElement || event.target instanceof HTMLTextAreaElement;
    if (
      resumes({
        key: event.key,
        ctrlKey: event.ctrlKey,
        metaKey: event.metaKey,
        altKey: event.altKey,
        typing,
      })
    )
      following = "following";
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
