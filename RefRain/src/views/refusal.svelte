<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  
  
  import { SvelteMap } from "svelte/reactivity";
  import { get } from "svelte/store";

  import { deliverable, momentOf, urgencyOf, wakeAt } from "../core/deferral";
  import { say } from "../core/lang";
  import { recoveryFor } from "../core/recovering";
  import { ui } from "../ui";
  import type { Refused } from "./notice_recovery";
  import { recover, recoveryLabel, recoveryWhy } from "./notice_recovery";
  import type { AxError } from "../wire";
  import { branchClaims } from "./talk/branching";
  import { claims } from "./talk/handing";
  import { attending } from "./mailbox/attention";
  import Button from "./parts/button.svelte";
  import Notice from "./parts/notice.svelte";
  import type { Clock, Holder, Stand, ToastLook } from "./refusal";
  import { held as heldBy, released, within } from "./refusal";
  import Look from "./refusal.look.svelte";

  
  const VISIBLE = 3;
  const LIFE = 8_000;

  
  type Said =
    { readonly kind: "refused"; readonly refused: Refused } | { readonly kind: "no_run_in_front" };

  interface Toast {
    readonly id: number;
    readonly said: Said;
    
    
    gone: boolean;
    
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

  
  let held: Refused[] = [];
  let wake: number | undefined;
  const attention = attending(u.conversing, u.now, release);
  $effect(() => () => {
    attention.stop();
    window.clearTimeout(wake);
  });

  
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

  
  $effect(() => {
    const refusal = $belief.refusal;
    if (refusal === null || refusal === heard) {
      return;
    }
    heard = refusal;
    if (claims(get(u.conversing), refusal, $belief) || branchClaims(get(u.branches.held), refusal))
      return;
    const refused: Refused = { error: refusal, about: aboutOf(refusal) };
    if (deliverable(urgencyOf(refusal), attention.read(), u.now())) {
      push({ kind: "refused", refused });
      return;
    }
    held = [...held, refused];
    release();
  });

  
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
      stand = {
        kind: "composer",
        left: rect.left,
        width: rect.width,
        bottom: window.innerHeight - rect.top,
      };
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
  toasts={toasts.map((toast): ToastLook => ({
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
  }))}
  {body}
/>
