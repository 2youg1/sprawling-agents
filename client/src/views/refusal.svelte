<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // The one thing that may float over a page uninvited: a refusal, in
  // the three parts the city wrote it in, seated at the bottom right
  // (client-SPEC 4-35). Three may stand at once; a fourth pushes the
  // oldest away, and everything that reached a corner is in the drawer
  // afterwards whatever happens here.
  //
  // **A toast leaves by itself.** Eight seconds, paused while the
  // pointer or the focus is on it - a person reading one is never
  // racing it. The departure is the display toggle `parts/dialog`
  // already models (`transition-behavior: allow-discrete` fading the
  // opacity first and hiding the box at the end), not a script timing
  // the animation; the timer here decides *when* a toast goes, never
  // how it looks going. A departure is cut when the next arrival
  // prunes it, which `cut needs no class` covers.
  //
  // **The corner is bottom right because bottom left is the rail's
  // hover zone.** A toast over the rail's open edge would be dismissed
  // by every hand reaching for a navigation row.
  //
  // **What a recovery's control says and does is
  // `notice_recovery.ts`'s**, shared with the drawer (client-SPEC
  // 4-35).
  import { SvelteMap } from "svelte/reactivity";
  import { get } from "svelte/store";

  import { say } from "../core/lang";
  import { recoveryFor } from "../core/recovering";
  import { ui } from "../ui";
  import type { About } from "./notice_recovery";
  import { recover, recoveryLabel, recoveryWhy } from "./notice_recovery";
  import type { AxError } from "../wire";
  import { claims } from "./talk/handing";
  import Button from "./parts/button.svelte";
  import Notice from "./parts/notice.svelte";

  // How many may stand at once, and how long one waits before it goes.
  const VISIBLE = 3;
  const LIFE = 8_000;

  interface Toast {
    readonly id: number;
    readonly error: AxError;
    // What its subject names, folded where every notice's subject is
    // folded (`core/belief/shape`), so the deed acts on the city's own
    // reading rather than a second grammar of its own.
    readonly about: About;
    // Set means it is leaving: the display toggle below plays the
    // departure, and the next arrival sweeps it out of the list.
    gone: boolean;
    // How much of its life is left, and when the stretch being counted
    // began - the two halves of a hover pause.
    left: number;
    since: number;
  }

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
  function aboutOf(error: AxError): About {
    const found = $belief.notices.find(
      (notice) => notice.error.code === error.code && notice.error.subject === error.subject,
    );
    return found?.about ?? null;
  }

  function arm(toast: Toast): void {
    stop(toast);
    toast.since = u.now();
    timers.set(
      toast.id,
      window.setTimeout(() => {
        timers.delete(toast.id);
        leave(toast);
      }, toast.left),
    );
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

  function push(error: AxError): void {
    // A departure that finished is swept when the next one arrives -
    // by then its fade has played, so the sweep is never seen.
    const kept = toasts.filter((each) => !each.gone);
    for (const oldest of kept.slice(0, Math.max(0, kept.length + 1 - VISIBLE))) {
      leave(oldest);
    }
    const next: Toast = {
      id: follow,
      error,
      about: aboutOf(error),
      gone: false,
      left: LIFE,
      since: u.now(),
    };
    follow += 1;
    toasts = [...kept, next];
    arm(next);
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
    push(refusal);
  });
</script>

<!-- Top right, where no page keeps a control a person needs at the
moment a refusal arrives: in the bottom corner the stack sat over the
composer's send button, which is the retry. -->
<ul class="fixed top-wide right-pane flex flex-col items-end gap-snug">
  {#each toasts as toast (toast.id)}
    <li
      class={[
        "transition-[opacity,display] transition-discrete duration-200 ease-standard motion-reduce:transition-none",
        toast.gone ? "hidden opacity-0" : "opacity-100",
      ]}
      onmouseenter={() => {
        stop(toast);
        toast.left = Math.max(0, toast.left - (u.now() - toast.since));
      }}
      onmouseleave={() => {
        if (!toast.gone) arm(toast);
      }}
    >
      <Notice
        seat="toast"
        weight="alert"
        action={toast.error.action}
        code={toast.error.code}
        subject={toast.error.subject}
        recovery={toast.error.recovery}
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
            {#each recoveryFor(toast.error) as recovery (recoveryLabel(recovery, $lang))}
              {@const why = recoveryWhy(u, recovery, toast)}
              <Button
                tone="quiet"
                label={recoveryLabel(recovery, $lang)}
                {...why === undefined ? {} : { why: say($lang, why) }}
                onPress={() => {
                  recover(u, recovery, toast);
                }}
              />
            {/each}
          </div>
        {/snippet}
      </Notice>
    </li>
  {/each}
</ul>
