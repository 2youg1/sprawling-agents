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
</script>

<script lang="ts">
  import { fill, say } from "../../core/lang";
  import { ui } from "../../ui";
  import type { Turn } from "../../wire";
  import CodeColumn from "./code_column.svelte";
  import Terminal from "./terminal.svelte";
  import { traceOf } from "./trace";
  import type { Target } from "./trace";

  interface Props {
    readonly turns: readonly Turn[];
    readonly onDraft: (text: string) => void;
    readonly onSteer: (text: string) => void;
  }

  const { turns, onDraft, onSteer }: Props = $props();
  const { lang } = ui();

  type Following = "following" | "paused";
  type Pane = "open" | "folded";

  const SCROLLING = new Set(["ArrowUp", "ArrowDown", "PageUp", "PageDown", "Home", "End", " "]);

  let following = $state<Following>("following");
  let terminal = $state<Pane>("open");
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

  function jump(path: string): void {
    following = "paused";
    land({ kind: "file", path });
  }

  function scrolledBy(event: KeyboardEvent): void {
    if (SCROLLING.has(event.key)) following = "paused";
  }

  function resumeBy(event: KeyboardEvent): void {
    const typing = event.target instanceof HTMLInputElement || event.target instanceof HTMLTextAreaElement;
    if (event.key.toLowerCase() !== "f" || typing || event.ctrlKey || event.metaKey || event.altKey) return;
    following = "following";
  }

  const PANE = "relative min-h-0 flex-1 overflow-y-auto focus-visible:outline-2 focus-visible:outline-accent";
</script>

<svelte:window onkeydown={resumeBy} />

<section class="flex min-h-0 flex-1 flex-col" aria-label={say($lang, "mon_monitor")}>
  <header class="flex flex-wrap items-center gap-snug border-b border-edge bg-chrome px-pane py-snug text-note">
    <span class="text-label text-text">{say($lang, "mon_code")}</span>
    <span class="text-text-faint figure">{fill(say($lang, "mon_files_n"), { n: String(trace.files.length) })}</span>
    <button
      type="button"
      class="ms-auto inline-flex h-control-sm items-center rounded-control px-snug text-label {following === 'following'
        ? 'text-accent'
        : 'bg-raised text-text'}"
      aria-pressed={following === "following"}
      onclick={() => (following = following === "following" ? "paused" : "following")}
      >{say($lang, following === "following" ? "mon_following" : "mon_paused")}</button
    >
    <button
      type="button"
      class="inline-flex h-control-sm items-center rounded-control px-snug text-label text-text-quiet hover:bg-raised"
      aria-expanded={terminal === "open"}
      onclick={() => (terminal = terminal === "open" ? "folded" : "open")}
      >{say($lang, terminal === "open" ? "mon_terminal_fold" : "mon_terminal_open")}</button
    >
  </header>
  <div class="flex min-h-0 flex-1">
    {#if terminal === "folded"}
      <nav class="w-rail-open shrink-0 overflow-y-auto border-e border-edge py-tight" aria-label={say($lang, "mon_files")}>
        {#each trace.files as file (file.path)}
          <button
            type="button"
            class="block w-full truncate px-snug py-tight text-start font-mono text-note text-text-quiet hover:bg-raised"
            onclick={() => {
              jump(file.path);
            }}>{file.path}</button
          >
        {/each}
      </nav>
    {/if}
    <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions (the pane takes focus so the keyboard can scroll it, and a scrolling key pauses following) -->
    <div
      bind:this={code}
      class="{PANE} {terminal === 'open' ? 'flex-[1.4] border-e border-edge' : ''}"
      role="region"
      aria-label={say($lang, "mon_code")}
      tabindex="0"
      onwheel={() => (following = "paused")}
      ontouchmove={() => (following = "paused")}
      onkeydown={scrolledBy}
    >
      <CodeColumn files={trace.files} {onDraft} {onSteer} />
    </div>
    {#if terminal === "open"}
      <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions (the pane takes focus so the keyboard can scroll it, and a scrolling key pauses following) -->
      <div
        bind:this={record}
        class="{PANE} bg-page"
        role="region"
        aria-label={say($lang, "mon_terminal")}
        tabindex="0"
        onwheel={() => (following = "paused")}
        ontouchmove={() => (following = "paused")}
        onkeydown={scrolledBy}
      >
        <Terminal entries={trace.entries} />
      </div>
    {/if}
  </div>
</section>
