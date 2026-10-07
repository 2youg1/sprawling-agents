<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // The one thing that may float over a page uninvited: a refusal, in
  // the three parts the city wrote it in, standing over the composer
  // (client/Spec.lean §4-35). Three may stand at once; a fourth pushes the
  // oldest away, and every refusal that reached a corner is in the
  // drawer afterwards whatever happens here.
  //
  // **The page's answer to a stop key with no run in front of it stands
  // in the same stack and nowhere else** (client D16): it says
  // what the person just did, and the drawer keeps what the city said.
  //
  // **A toast leaves by itself.** Eight seconds, paused while the
  // pointer or the focus is on it - a person reading one is never
  // racing it (`refusal.ts` keeps that clock). The departure is the display toggle `parts/dialog`
  // already models (`transition-behavior: allow-discrete` fading the
  // opacity first and hiding the box at the end), not a script timing
  // the animation; the timer here decides *when* a toast goes, never
  // how it looks going. A departure is cut when the next arrival
  // prunes it, which `cut needs no class` covers.
  //
  // **Over the composer, because that is where the eye is** when a send
  // or a stop is refused: the stack stands on the composer's upper edge
  // and takes the width of the column the composer is in, so it moves
  // with that column when the right pane opens (client/Spec.lean §4-35). A
  // page with no composer centres it at the foot.
  //
  // **When it steps forward is `core/deferral.ts`'s** (client
  // D26): what stops the work until the person acts comes at once;
  // an ordinary refusal waits, held here, for the person to send, to
  // leave the box empty, or to come back to this tab, and meanwhile
  // only marks the mailbox key. The mailbox keeps every refusal
  // whatever this stack does, so one the person read there is dropped
  // here rather than shown late.
  //
  // **What a recovery's control says and does is
  // `notice_recovery.ts`'s**, shared with the drawer (client/Spec.lean
  // §4-35).
  //
  // This is the seat: it keeps the toasts and their clocks and draws
  // each notice; how the stack and a toast look is
  // `refusal.look.svelte`'s.
  import { SvelteMap } from "svelte/reactivity";
  import { get } from "svelte/store";

  import { deliverable, momentOf, urgencyOf, wakeAt } from "../core/deferral";
  import { say } from "../core/lang";
  import { recoveryFor } from "../core/recovering";
  import { ui } from "../ui";
  import type { Refused } from "./notice_recovery";
  import { recover, recoveryLabel, recoveryWhy } from "./notice_recovery";
  import type { AxError } from "../wire";
  import { claims } from "./talk/handing";
  import { attending } from "./mailbox/attention";
  import Button from "./parts/button.svelte";
  import Notice from "./parts/notice.svelte";
  import type { Clock, Holder, Stand, ToastLook } from "./refusal";
  import { held as heldBy, released, within } from "./refusal";
  import Look from "./refusal.look.svelte";

  // How many may stand at once, and how long one waits before it goes.
  const VISIBLE = 3;
  const LIFE = 8_000;

  // A refusal carries what its subject names, folded where every
  // notice's subject is folded (`core/belief/shape`), so the deed acts
  // on the city's own reading rather than a second grammar of its own.
  type Said = { readonly kind: "refused"; readonly refused: Refused } | { readonly kind: "no_run_in_front" };

  interface Toast {
    readonly id: number;
    readonly said: Said;
    // Set means it is leaving: the look's display toggle plays the
    // departure, and the next arrival sweeps it out of the list.
    gone: boolean;
    // How much of its life is left, and what holds it.
    clock: Clock;
  }

  const { stopsWithoutRun }: { readonly stopsWithoutRun: number } = $props();

  const u = ui();
  const { lang } = u;
  const belief = u.conn.belief;

  let toasts = $state<Toast[]>([]);
  let follow = 0;
  let heard: AxError | null = null;
  const timers = new SvelteMap<number, number>();

  // What the refusal's subject names, as the belief folded it when the
  // notice arrived. `null` is the honest answer for a subject that is a
  // sentence rather than a name.
  function aboutOf(error: AxError): Refused["about"] {
    const found = $belief.notices.find(
      (notice) => notice.error.code === error.code && notice.error.subject === error.subject,
    );
    return found?.about ?? null;
  }

  function arm(toast: Toast): void {
    stop(toast);
    toast.clock = { ...toast.clock, since: u.now() };
    timers.set(
      toast.id,
      window.setTimeout(() => {
        timers.delete(toast.id);
        leave(toast);
      }, toast.clock.left),
    );
  }

  function hold(toast: Toast, by: Holder): void {
    stop(toast);
    toast.clock = heldBy(toast.clock, by, u.now());
  }

  function letGo(toast: Toast, by: Holder): void {
    const after = released(toast.clock, by, u.now());
    toast.clock = after.clock;
    if (after.runs && !toast.gone) arm(toast);
  }

  function stop(toast: Toast): void {
    const held = timers.get(toast.id);
    if (held !== undefined) {
      window.clearTimeout(held);
      timers.delete(toast.id);
    }
  }

  function leave(toast: Toast): void {
    stop(toast);
    toast.gone = true;
  }

  function push(said: Said): void {
    // A departure that finished is swept when the next one arrives -
    // by then its fade has played, so the sweep is never seen.
    const kept = toasts.filter((each) => !each.gone);
    for (const oldest of kept.slice(0, Math.max(0, kept.length + 1 - VISIBLE))) {
      leave(oldest);
    }
    const next: Toast = {
      id: follow,
      said,
      gone: false,
      clock: { left: LIFE, since: u.now(), holders: [] },
    };
    follow += 1;
    toasts = [...kept, next];
    arm(next);
  }

  // The refusals waiting for a moment, oldest first, and the timer that
  // wakes when a box left empty becomes one.
  let held: Refused[] = [];
  let wake: number | undefined;
  const attention = attending(u.conversing, u.now, release);
  $effect(() => () => {
    attention.stop();
    window.clearTimeout(wake);
  });

  // Whether the mailbox still calls this refusal unread: one the person
  // already read there has been told, and is not told again here.
  function unread(error: AxError): boolean {
    return get(belief).notices.some((notice) => notice.error === error && !notice.seen);
  }

  function release(): void {
    window.clearTimeout(wake);
    held = held.filter((refused) => unread(refused.error));
    if (held.length === 0) return;
    const seen = attention.read();
    const now = u.now();
    if (momentOf(seen, now) !== null) {
      for (const refused of held) push({ kind: "refused", refused });
      held = [];
      return;
    }
    const due = wakeAt(seen, now);
    if (due !== null) wake = window.setTimeout(release, due - now);
  }

  // Every refusal the city comes back with is toasted once, except the
  // one the open conversation draws as a card where its reply would
  // have been (`talk/handing.ts`): the same refusal in two places is
  // one refusal said twice. The belief hands out a new value per
  // arrival, so identity is the arrival.
  $effect(() => {
    const refusal = $belief.refusal;
    if (refusal === null || refusal === heard) {
      return;
    }
    heard = refusal;
    if (claims(get(u.conversing), refusal, $belief)) return;
    const refused: Refused = { error: refusal, about: aboutOf(refusal) };
    if (deliverable(urgencyOf(refusal), attention.read(), u.now())) {
      push({ kind: "refused", refused });
      return;
    }
    held = [...held, refused];
    release();
  });

  // Where the stack stands: on the composer's upper edge, as wide as its
  // column, or nowhere in particular when the page has none. Measured
  // only while a toast stands, and again whenever the composer's box
  // moves or the window changes size.
  let stand = $state.raw<Stand>({ kind: "foot" });
  const standing = $derived(toasts.some((toast) => !toast.gone));
  $effect(() => {
    if (!standing) return;
    const box = document.querySelector("main textarea");
    const form = box instanceof HTMLElement ? box.closest("form") : null;
    if (form === null) {
      stand = { kind: "foot" };
      return;
    }
    const measure = (): void => {
      const rect = form.getBoundingClientRect();
      stand = { kind: "composer", left: rect.left, width: rect.width, bottom: window.innerHeight - rect.top };
    };
    measure();
    const watching = new ResizeObserver(measure);
    watching.observe(form);
    window.addEventListener("resize", measure);
    return () => {
      watching.disconnect();
      window.removeEventListener("resize", measure);
    };
  });

  // Each press the shell counted is answered once; the count only grows.
  let answered = 0;
  $effect(() => {
    if (stopsWithoutRun === answered) return;
    answered = stopsWithoutRun;
    push({ kind: "no_run_in_front" });
  });
</script>

{#snippet body(id: number)}
  {@const toast = toasts.find((each) => each.id === id)}
  {#if toast !== undefined}
    {#if toast.said.kind === "refused"}
      {@const refused = toast.said.refused}
      <Notice
        seat="toast"
        weight="alert"
        action={refused.error.action}
        code={refused.error.code}
        subject={refused.error.subject}
        recovery={refused.error.recovery}
      >
        {#snippet actions()}
          <div class="flex flex-col gap-tight">
            <Button
              tone="quiet"
              label={say($lang, "dismiss")}
              onPress={() => {
                // Waving one away is still the answer to what was
                // asked, so the corner lets go and the drawer keeps it.
                u.conn.dismissRefusal();
                leave(toast);
              }}
            />
            {#each recoveryFor(refused.error) as recovery (recoveryLabel(recovery, $lang))}
              {@const why = recoveryWhy(u, recovery, refused)}
              <Button
                tone="quiet"
                label={recoveryLabel(recovery, $lang)}
                {...why === undefined ? {} : { why: say($lang, why) }}
                onPress={() => {
                  recover(u, recovery, refused);
                }}
              />
            {/each}
          </div>
        {/snippet}
      </Notice>
    {:else}
      <Notice seat="toast" weight="info" heading="no_run_in_front" next="stop_whole_city">
        {#snippet actions()}
          <Button
            tone="quiet"
            label={say($lang, "dismiss")}
            onPress={() => {
              leave(toast);
            }}
          />
        {/snippet}
      </Notice>
    {/if}
  {/if}
{/snippet}

<Look
  {stand}
  toasts={toasts.map(
    (toast): ToastLook => ({
      key: toast.id,
      gone: toast.gone,
      wire: {
        onmouseenter: () => {
          hold(toast, "pointer");
        },
        onmouseleave: () => {
          letGo(toast, "pointer");
        },
        onfocusin: () => {
          hold(toast, "focus");
        },
        onfocusout: (event) => {
          if (!within(event)) letGo(toast, "focus");
        },
      },
    }),
  )}
  {body}
/>
