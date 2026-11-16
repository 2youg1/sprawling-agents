<script lang="ts">
  import { tick, untrack, type Snippet } from "svelte";

  import { QUERIES } from "../core/asking";
  import { readAnswer } from "../core/answered";
  import { putGuide } from "../core/commands";
  import { say } from "../core/lang";
  import { MAYOR } from "../core/route";
  import { ui } from "../ui";
  import type { GuideProgress, GuideStep } from "../wire";
  import Button from "./parts/button.svelte";
  import Page from "./parts/page.svelte";
  import Unanswered from "./parts/unanswered.svelte";
  import { outstanding } from "./setup/dependencies";
  import type { Configured } from "./welcome/guide";
  import {
    STEPS,
    advanced,
    currentOf,
    left,
    opened,
    putOff,
    skipAll,
    standingOf,
  } from "./welcome/guide";
  import Body from "./welcome/body.svelte";
  import { ABOUT, bodyId, stepLookOf } from "./welcome/step";
  import Step from "./welcome/step.look.svelte";

  interface Props {
    readonly rank?: "page" | "section" | undefined;

    readonly step?: GuideStep | undefined;

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
    const chosen =
      $endpoints !== undefined && "endpoints" in $endpoints ? $endpoints.endpoints.chosen : null;
    const missing =
      $doctor !== undefined && "doctor" in $doctor ? outstanding($doctor.doctor, "use") : null;
    const stated =
      $identity !== undefined && "identity" in $identity && "stated" in $identity.identity
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

  let sent = $state<GuideProgress | null>(null);
  $effect(() => {
    if ($guide === undefined) return;
    untrack(() => {
      sent = null;
    });
  });
  const progress = $derived(sent ?? (read.kind === "held" ? read.value : {}));
  const current = $derived(currentOf(progress, configured));

  let folded = $state(false);

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
</script>

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
              <li class="border-b border-edge">
                <Step
                  {...stepLookOf(
                    {
                      step,
                      at,
                      standing: standingOf(step, progress, configured),
                      open: open === step,
                      id: headId(step),
                      lang: $lang,
                    },
                    () => {
                      toggle(step);
                    },
                  )}
                />
                {#if open === step}
                  <section
                    id={bodyId(headId(step))}
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
