<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->
<script lang="ts" module>
  // What the run produced, beside the conversation that produced it.
  //
  // **Driven by the ledger, not by the address bar.** The card shows the
  // newest thing this run touched of each kind, all read from the same
  // rounds the thread beside it is drawn from. A route of its own would
  // add a second answer to "which run am I looking at", and the two
  // answers would differ the first time a person opened a link.
  //
  // **One card, three panes - not three cards.** A tool result is one of
  // three things to read, and they are not peers on the page: a file
  // somebody read is the subject and takes the body, while what changed
  // and what a command printed are two readings of the same run and take
  // one strip at the foot of the card, one at a time. Three cards would
  // make the eye choose which to look at first; one card with a head, a
  // body and a tabbed foot says which is the subject and keeps the other
  // two a click away.
  //
  // **The three are the three deeds `trace.ts` already names.** Reading,
  // changing and running are told apart by `deedOf`, which is the same
  // authority the counting sentence above the thread uses. Nothing here
  // classifies a call for itself.
  //
  // **The control that opens and closes this card is not in it.** It sits
  // beside the thread, where it is reachable when the card is closed; a
  // second one in the head would be a second place to look for one piece
  // of state, and the closed case would still need the first.
  //
  // **Two secondary actions ride the head, revealed by hover or focus
  // (ux A7).** Copy takes the output the foot is showing - a body of
  // code carries its own copy in `parts/code.svelte`'s header, so the
  // same text never offers the same action twice - and the jump opens
  // this run's own page. Both stand at the small control height with
  // the touch surface widened underneath, and the copy receipt is the
  // glyph table's check held for a moment.

  // The name the toggle points at with `aria-controls`. Written once so
  // the control and the region it opens cannot drift apart.
  export const PANEL_ID = "artifact";

  // The two readings the foot of the card holds. A closed set, because
  // the tab strip is built from it and a third reading would have to be
  // a pane rather than a string somebody added.
  export type Foot = "wrote" | "terminal";

  // How long the copy receipt holds its check mark: long enough to see
  // one, short enough that the mark never becomes the button's face.
  const RECEIPT_MS = 1200;

  // The secondary action, at the small control height. It is invisible
  // and click-through at rest, so the corner never stands between a hand
  // and the words under it, and it arrives as `fade`. The `::before`
  // widens the touch surface to 44 while the drawn control stays 28, and
  // a machine that asks for less motion gets the mark without the
  // arrival. The one difference from `parts/code.svelte`'s copy is the
  // face of the idle mark: the glyph table holds no `copy` name, so the
  // mark is drawn here the way the code view draws it, and both fold
  // into `parts/glyph.ts` the day the table gains one (ux A14).
  const SHAPE =
    "relative flex h-control-sm w-control-sm shrink-0 items-center " +
    "justify-center rounded-control text-text-quiet opacity-0 before:absolute " +
    "before:-inset-snug before:content-[''] transition-opacity ease-leave " +
    "hover:bg-raised hover:text-text pointer-events-none " +
    "group-hover:opacity-100 group-hover:pointer-events-auto group-hover:ease-arrive " +
    "group-focus-within:opacity-100 group-focus-within:pointer-events-auto " +
    "motion-reduce:transition-none";
</script>

<script lang="ts">
  import type { Call, RunId } from "../../wire";
  import { toFragment } from "../../core/route";
  import { fill, say } from "../../core/lang";
  import { ui } from "../../ui";
  import Code from "../parts/code.svelte";
  import Glyph from "../parts/glyph.svelte";
  import Tabs from "../parts/tabs.svelte";
  import type { Artifacts } from "./trace";

  interface ArtifactProps {
    readonly artifacts: Artifacts;
    // Whether the person has the card open. Drawn either way rather than
    // mounted on demand, so the toggle beside it always names a region
    // that exists.
    readonly open: boolean;
    // The run this card speaks for, when the page holding it knows one:
    // it is what the head's jump opens. Absent on a fixture that stands
    // for a run without being one.
    readonly run?: RunId | undefined;
  }

  const { artifacts, open, run }: ArtifactProps = $props();

  // The row `parts/tabs.svelte` hands its `panel` snippet, named
  // structurally rather than through the part's `Lens`: that name lives
  // in the part's instance script and does not resolve in every checker
  // in this toolchain.
  interface FootLens {
    readonly id: string;
    readonly label: string;
  }

  const { lang } = ui();

  const read = $derived(artifacts.read);
  const wrote = $derived(artifacts.wrote);
  const terminal = $derived(artifacts.terminal);

  // Which readings the foot can offer, in the order they are drawn.
  const offered = $derived.by((): Foot[] => {
    const held: Foot[] = [];
    if (terminal !== null) held.push("terminal");
    if (wrote !== null) held.push("wrote");
    return held;
  });
  let picked = $state<Foot | null>(null);
  // The person's choice while it is still on offer, and the first
  // reading there is otherwise. Derived rather than corrected by an
  // effect: a run that stops producing command output must not leave
  // the card pointing at a pane that is no longer there.
  const showing = $derived.by((): Foot | null => {
    const held = picked;
    return held !== null && offered.includes(held) ? held : (offered[0] ?? null);
  });
  const lenses = $derived(
    offered.map((foot) => ({
      id: foot,
      label: foot === "terminal" ? say($lang, "talk_panel_terminal") : say($lang, "talk_panel_change"),
    })),
  );
  const subject = $derived(read ?? wrote ?? terminal);

  function callOf(foot: Foot): Call | null {
    switch (foot) {
      case "terminal":
        return terminal;
      case "wrote":
        return wrote;
    }
  }

  // The output the foot is showing right now, and nothing when the foot
  // shows nothing: the head's copy then stands down and the code view's
  // own copy serves the body.
  const output = $derived(showing === null ? "" : callOf(showing)?.output?.head ?? "");

  let copied = $state(false);
  let receipt: ReturnType<typeof setTimeout> | undefined = undefined;

  // The receipt appears only after the write landed, so a press that
  // quietly failed shows no check rather than a lying one.
  function copy(): void {
    void navigator.clipboard.writeText(output).then(() => {
      copied = true;
      if (receipt !== undefined) clearTimeout(receipt);
      receipt = setTimeout(() => {
        copied = false;
        receipt = undefined;
      }, RECEIPT_MS);
    });
  }

  function onPick(id: string): void {
    picked = id === "terminal" ? "terminal" : "wrote";
  }
</script>

{#snippet cut(call: Call)}
  {#if (call.output?.cut ?? 0) > 0}
    <p class="border-t border-edge px-snug py-tight text-note text-text-faint">
      {fill(say($lang, "run_cut"), { n: String(call.output?.cut ?? 0) })}
    </p>
  {/if}
{/snippet}

<!-- A pane of plain output: what a command printed, or what an edit
     reported. Monospaced, scrolling inside itself, and sitting on the
     page rather than on a fill - the deepest surface on the screen is
     the content, which is what the head and the strips around it lift
     off. -->
{#snippet printed(call: Call, label: string)}
  <section class="flex min-h-0 flex-1 flex-col bg-page" aria-label={label}>
    <p class="truncate border-b border-edge px-snug py-tight font-mono text-note text-text-faint">
      {call.subject === null || call.subject === undefined || call.subject === ""
        ? call.tool
        : `${call.tool} ${call.subject}`}
    </p>
    <pre
      class="min-h-0 flex-1 overflow-auto px-snug py-tight font-mono text-note leading-relaxed text-text-quiet"
    >{call.output?.head ?? ""}</pre>
    <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression (a snippet call is the render itself; the typechecker types local snippet calls as returning void) -->
    {@render cut(call)}
  </section>
{/snippet}

{#snippet footPanel(lens: FootLens)}
  {@const call = callOf(lens.id === "terminal" ? "terminal" : "wrote")}
  {#if call !== null}
    <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression (a snippet call is the render itself; the typechecker types local snippet calls as returning void) -->
    {@render printed(call, lens.label)}
  {/if}
{/snippet}

<aside
  id={PANEL_ID}
  aria-label={say($lang, "talk_panel")}
  class={["group flex h-full min-h-0 w-full flex-col overflow-hidden bg-chrome", !open && "hidden"]}
>
  <!-- The head names the subject of the body, and takes the shell's one
       bar height rather than a second nearly-equal number of its own. -->
  {#if subject !== null}
    <header class="flex h-bar shrink-0 items-center gap-snug border-b border-edge px-snug">
      <span class="min-w-0 flex-1 truncate font-mono text-label text-text">
        {subject.subject ?? subject.tool}
      </span>
      {#if showing !== null}
        <button
          type="button"
          class={SHAPE}
          aria-label={say($lang, copied ? "code_copied" : "code_copy")}
          onclick={copy}
        >
          {#if copied}
            <Glyph name="check" size="sm" />
          {:else}
            <!-- The copy mark: two sheets, the shape `parts/code.svelte`
                 also draws. `parts/glyph.ts` has no name for it yet. -->
            <svg
              viewBox="0 0 20 20"
              class="size-glyph-sm"
              fill="none"
              stroke="currentColor"
              stroke-width="1.5"
              stroke-linecap="round"
              stroke-linejoin="round"
              aria-hidden="true"
            >
              <path
                d="M7 7h10v10H7zM13 7V5a2 2 0 0 0-2-2H5a2 2 0 0 0-2 2v6a2 2 0 0 0 2 2h2"
                vector-effect="non-scaling-stroke"
              />
            </svg>
          {/if}
        </button>
      {/if}
      {#if run !== undefined}
        <a class={SHAPE} href={toFragment({ kind: "run", run })} aria-label={say($lang, "talk_panel_jump")}>
          <Glyph name="chevron" size="sm" />
        </a>
      {/if}
    </header>
  {/if}

  {#if read !== null}
    <section
      class="flex min-h-0 flex-1 flex-col border-b border-edge bg-page"
      aria-label={say($lang, "talk_panel_code")}
    >
      <Code path={read.subject ?? ""} text={read.output?.head ?? ""} />
      <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression (a snippet call is the render itself; the typechecker types local snippet calls as returning void) -->
      {@render cut(read)}
    </section>
  {/if}

  {#if showing !== null}
    <div class="flex min-h-0 flex-1 shrink-0 flex-col">
      {#if lenses.length > 1}
        <Tabs
          label={say($lang, "talk_panel")}
          {lenses}
          current={showing}
          onPick={onPick}
          panel={footPanel}
        />
      {:else}
        {@const call = callOf(showing)}
        {#if call !== null}
          <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression (a snippet call is the render itself; the typechecker types local snippet calls as returning void) -->
          {@render printed(
            call,
            showing === "terminal" ? say($lang, "talk_panel_terminal") : say($lang, "talk_panel_change"),
          )}
        {/if}
      {/if}
    </div>
  {/if}
</aside>
