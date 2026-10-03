<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // The first-run guide (refrain §3-15, client/Spec.lean §7G): five steps in
  // one column, only the first required. Each step is the city's real
  // door for that job - the provider form and the `main` model choice,
  // the doctor's report with its installs, the governing documents, the
  // shelves, the MCP page - so finishing a step here and finishing it in
  // the settings are one act with one receipt.
  //
  // **Done is the city's answer; put off is the person's.** `guide.ts`
  // keeps the two apart: a step is drawn done only when the city's
  // configuration says so, and the progress the city keeps for this
  // guide (`Query::Guide`, `Command::PutGuide`) records only what the
  // person looked at, put off, and where the guide reopens. The progress
  // writes no ledger record, so after each write the page asks for it
  // again, and until that answer lands it draws the write it sent.
  //
  // **Leaving is one choice, made two ways.** "Start the conversation"
  // and "skip all optional" both open the Mayor's room with nothing sent
  // and nothing written into the box, and the guide is then left; the
  // shell still opens it at launch while the city has no provider
  // endpoint (client D54).
  //
  // **A step done moves the guide on.** When the open step turns done in
  // the city's answer, the next step nobody has done or put off opens and
  // the done one folds (client D55); a body arrives with `drop` and goes
  // at once.

  import { tick, untrack, type Snippet } from "svelte";

  import { QUERIES } from "../core/asking";
  import { readAnswer } from "../core/answered";
  import { putGuide } from "../core/commands";
  import { say } from "../core/lang";
  import type { Key } from "../core/lang";
  import { MAYOR } from "../core/route";
  import { ui } from "../ui";
  import type { GuideProgress, GuideStep } from "../wire";
  import Button from "./parts/button.svelte";
  import Glyph from "./parts/glyph.svelte";
  import Page from "./parts/page.svelte";
  import Unanswered from "./parts/unanswered.svelte";
  import { outstanding } from "./setup/dependencies";
  import type { Configured, Standing } from "./welcome/guide";
  import { STEPS, advanced, currentOf, left, opened, putOff, skipAll, standingOf } from "./welcome/guide";
  import Body from "./welcome/body.svelte";

  interface Props {
    // Whether this layout is the page or a region inside one: a
    // document may have exactly one heading of the page's own rank.
    readonly rank?: "page" | "section" | undefined;
    // The step a `#/welcome/<step>` link opens (client/Spec.lean §4-63).
    readonly step?: GuideStep | undefined;
    // The shell's back key, which `pages.svelte` draws and hands over so
    // it stands on this page's one column rather than at the frame's
    // left edge, far from the column it leads away from.
    readonly back?: Snippet | undefined;
  }

  const { rank = "page", step, back }: Props = $props();

  const u = ui();
  const { lang } = u;
  const uid = $props.id();

  const endpoints = u.conn.asking.ask(QUERIES.endpoints);
  const doctor = u.conn.asking.ask(QUERIES.doctor);
  const identity = u.conn.asking.ask(QUERIES.identity);
  const guide = u.conn.asking.ask(QUERIES.guide);

  const configured = $derived.by((): Configured => {
    const chosen = $endpoints !== undefined && "endpoints" in $endpoints ? $endpoints.endpoints.chosen : null;
    const missing = $doctor !== undefined && "doctor" in $doctor ? outstanding($doctor.doctor, "use") : null;
    const stated = $identity !== undefined && "identity" in $identity && "stated" in $identity.identity
      ? $identity.identity.stated
      : null;
    return {
      provider: chosen === null ? null : chosen.some((each) => each.tag === "main"),
      dependencies: missing === null ? null : missing.length === 0,
      texts: stated === null ? null : named(stated.user_id) || named(stated.mayor),
      skills: null,
      mcp: null,
    };
  });

  function named(value: string | null | undefined): boolean {
    return value !== null && value !== undefined && value.trim() !== "";
  }

  const read = $derived(readAnswer($guide, (held) => ("guide" in held ? held.guide : undefined)));
  // The write this page sent and the city has not answered yet.
  let sent = $state<GuideProgress | null>(null);
  $effect(() => {
    if ($guide === undefined) return;
    untrack(() => {
      sent = null;
    });
  });
  const progress = $derived(sent ?? (read.kind === "held" ? read.value : {}));
  const current = $derived(currentOf(progress, configured));
  // The one step drawn open: the current one, unless the person folded it.
  let folded = $state(false);
  // A step a link named is drawn open without telling the city the guide
  // moved there: a link opens, it does not write. Choosing a step clears it.
  let linked = $state<GuideStep | null>(null);
  $effect(() => {
    const asked = step ?? null;
    untrack(() => {
      linked = asked;
      folded = false;
      if (asked !== null) void tick().then(() => document.getElementById(headId(asked))?.focus());
    });
  });
  const open = $derived(folded ? null : (linked ?? current));
  const ready = $derived(configured.provider === true);

  // The city's previous reading, to tell a step the person just did from
  // an answer that only now arrived.
  let before: Configured | null = null;
  $effect(() => {
    const after = configured;
    untrack(() => {
      const was = before;
      const shown = open;
      before = after;
      if (was === null || shown === null) return;
      const next = advanced(progress, shown, was, after);
      if (next === null) return;
      linked = null;
      folded = false;
      write(next);
    });
  });

  function write(next: GuideProgress): void {
    sent = next;
    u.send(putGuide(next));
    u.conn.asking.refresh(QUERIES.guide);
  }

  function toggle(step: GuideStep): void {
    const shown = open;
    linked = null;
    if (shown === step) {
      folded = true;
      return;
    }
    folded = false;
    write(opened(progress, step));
  }

  // Putting a step off closes its body, so the focus moves to the step
  // the guide moved on to rather than to the top of the document.
  function later(step: Exclude<GuideStep, "provider">): void {
    folded = false;
    linked = null;
    const next = putOff(progress, step);
    write(next);
    void tick().then(() => {
      document.getElementById(headId(next.at ?? step))?.focus();
    });
  }

  function leave(next: GuideProgress): void {
    write(next);
    u.prefs.setWelcomed(true);
    u.go({ kind: "talk", address: MAYOR });
  }

  function headId(step: GuideStep): string {
    return `${uid}-${step}`;
  }

  const TITLE: Record<GuideStep, Key> = {
    provider: "guide_step_provider",
    dependencies: "guide_step_dependencies",
    texts: "guide_step_texts",
    skills: "guide_step_skills",
    mcp: "guide_step_mcp",
  };
  const ABOUT: Record<GuideStep, Key> = {
    provider: "guide_step_provider_about",
    dependencies: "guide_step_dependencies_about",
    texts: "guide_step_texts_about",
    skills: "guide_step_skills_about",
    mcp: "guide_step_mcp_about",
  };
  const WORD: Record<Standing, Key | null> = {
    configured: "guide_standing_configured",
    required: "guide_standing_required",
    skipped: "guide_standing_skipped",
    seen: "guide_standing_seen",
    untouched: null,
  };
  const INK: Record<Standing, string> = {
    configured: "text-text",
    required: "text-alert",
    skipped: "text-text-faint",
    seen: "text-text-quiet",
    untouched: "text-text-faint",
  };
</script>

<!-- One column, standing in the middle part of the shell's silver cut
(client D24), where the conversation stands: the steps, their rules and the
fields inside them share its two edges, so the page has one right edge
rather than one per kind of row. -->
<div class="flex min-w-0 flex-1 flex-col @min-[64rem]:silver-columns">
  <div class="flex min-w-0 flex-1 flex-col @min-[64rem]:col-start-2">
    {@render back?.()}
    <Page title={say($lang, "welcome_title")} note={say($lang, "welcome_note")} {rank}>
      <div class="@container min-w-0">
        <div class="flex min-w-0 flex-col">
          {#if read.kind === "unavailable"}
            <Unanswered query={read.query} asked={QUERIES.guide} />
          {/if}
          <ol aria-label={say($lang, "guide_steps")}>
            {#each STEPS as step, at (step)}
              {@const standing = standingOf(step, progress, configured)}
              {@const word = WORD[standing]}
              <li class="border-b border-edge">
                <h2>
                  <button
                    id={headId(step)}
                    type="button"
                    class="grid w-full grid-cols-[4ch_minmax(0,1fr)_auto] items-baseline gap-x-base py-base text-left transition-colors hover:wash"
                    aria-expanded={open === step}
                    aria-controls={`${headId(step)}-body`}
                    onclick={() => {
                      toggle(step);
                    }}
                  >
                    <span class="figure text-heading text-text-faint">{String(at + 1).padStart(2, "0")}</span>
                    <span class="min-w-0 text-label font-label text-text">{say($lang, TITLE[step])}</span>
                    <span class={["flex items-center gap-tight text-note", INK[standing]]}>
                      {#if standing === "configured"}
                        <Glyph name="check" class="size-glyph-sm" />
                      {/if}
                      {#if word !== null}{say($lang, word)}{/if}
                    </span>
                  </button>
                </h2>
                {#if open === step}
                  <section
                    id={`${headId(step)}-body`}
                    aria-labelledby={headId(step)}
                    class="drop flex min-w-0 flex-col gap-base pb-wide pl-[calc(4ch+var(--spacing-base))] @max-[40rem]:pl-0"
                  >
                    <p class="text-note text-text-quiet">{say($lang, ABOUT[step])}</p>
                    <Body {step} />
                    {#if step !== "provider"}
                      <div class="flex justify-end">
                        <Button
                          label={say($lang, "guide_later")}
                          tone="quiet"
                          onPress={() => {
                            later(step);
                          }}
                        />
                      </div>
                    {/if}
                  </section>
                {/if}
              </li>
            {/each}
          </ol>
          <div class="flex flex-wrap items-center gap-base pt-wide">
            <Button
              label={say($lang, "guide_start")}
              tone="primary"
              {...ready ? {} : { why: say($lang, "guide_start_needs_main") }}
              onPress={() => {
                leave(left(progress));
              }}
            />
            <Button
              label={say($lang, "guide_put_off_rest")}
              tone="quiet"
              onPress={() => {
                leave(skipAll(progress));
              }}
            />
          </div>
        </div>
      </div>
    </Page>
  </div>
</div>
