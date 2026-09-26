<script lang="ts" module>
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What this machine has, as the city found it when it started. The
// first step of the first half hour used to be one command to copy and
// no way of knowing whether it had worked; this is the answer.
//
// The answer and its three states each have their own component under
// `views/machine/`, re-exported here so the gallery reaches the same
// pieces the page reaches. This file owns only the ask: the city said
// it has no such answer means nobody has asked this machine since it
// was served, because serving one no longer spends the seconds it takes
// to start thirty-two programs and ask each its version. The page that
// shows the answer is the page that asks for it - once per opening, and
// through the same command the button sends, so "look at this machine"
// keeps one authority.

export { default as MachineReport } from "./machine/report.svelte";
export { default as MachineSkeleton } from "./machine/skeleton.svelte";
export { default as MachineUnchecked } from "./machine/unchecked.svelte";

// The one command this screen exists to hand over.
const DOCTOR = "sprawling doctor --install";
</script>

<script lang="ts">
  import { untrack } from "svelte";
  import { QUERIES } from "../core/asking";
  import { doctorInstall, doctorRefresh } from "../core/commands";
  import { say } from "../core/lang";
  import { ui } from "../ui";
  import type { DoctorAnswer } from "../wire";
  import { answered, plan, refused, running, started, type Walk } from "./setup/installing";
  import Button from "./parts/button.svelte";
  import Copy from "./machine/copy.svelte";
  import Report from "./machine/report.svelte";
  import Skeleton from "./machine/skeleton.svelte";
  import Unchecked from "./machine/unchecked.svelte";

  const u = ui();
  const lang = u.lang;
  const link = u.conn.state;
  const belief = u.conn.belief;
  const held = u.conn.asking.ask(QUERIES.doctor);

  const answer = $derived.by((): DoctorAnswer | undefined => {
    const now = $held;
    return now !== undefined && "doctor" in now ? now.doctor : undefined;
  });
  const unasked = $derived.by((): boolean => {
    const now = $held;
    return now !== undefined && !("doctor" in now);
  });

  let asking = $state(false);
  // A fresh answer ends the wait, whoever asked for it.
  $effect(() => {
    if (answer !== undefined) {
      asking = false;
    }
  });

  // The city writes one log line when it has looked at this machine
  // again, and that line is how this page knows the probe is over. A
  // read sent on the same tick as the command would arrive first and
  // answer with the machine as it was before, because probing is a
  // dozen programs started and asked their version.
  //
  // A city running with its log off writes no such line. `check again`
  // is then the whole mechanism, which is why it stays a control the
  // person can press rather than something the page does for them.
  const looked = $derived($belief.logs.filter((line) => line.module === "bin::doctor").length);
  let seenAt = -1;
  $effect(() => {
    const now = looked;
    if (seenAt < 0) {
      seenAt = now;
      return;
    }
    if (now !== seenAt) {
      seenAt = now;
      u.conn.asking.refresh(QUERIES.doctor);
    }
  });

  // Each of the two asks straight away as well, so the page is never
  // left waiting on a line that may not come; that first answer is
  // whatever the city holds now, and the log line brings the second.
  function recheck(): void {
    asking = true;
    u.send(doctorRefresh());
    u.conn.asking.refresh(QUERIES.doctor);
  }

  let opened = false;
  $effect(() => {
    if (opened || !unasked) return;
    opened = true;
    recheck();
  });

  function install(item: string): void {
    asking = true;
    u.send(doctorInstall(item));
    u.conn.asking.refresh(QUERIES.doctor);
  }

  // One press installs everything the develop tier is missing, one
  // item at a time (`views/setup/installing`): each fresh answer and
  // each refusal moves the walk on, and the item it moves to is sent.
  let walk = $state.raw<Walk | null>(null);
  const planned = $derived(answer === undefined ? [] : plan(answer));

  function installAll(): void {
    walk = started(planned);
  }

  $effect(() => {
    const now = answer;
    const held = untrack(() => walk);
    if (now === undefined || held === null) return;
    walk = answered(held, now);
  });

  const refusal = $derived($belief.refusal);
  $effect(() => {
    const now = refusal;
    const held = untrack(() => walk);
    if (now === null || held === null) return;
    walk = refused(held, now);
  });

  let sent: string | null = null;
  $effect(() => {
    const next = walk === null ? null : running(walk);
    if (next === null || next === sent) return;
    sent = next;
    install(next);
  });

  const reaching = $derived.by((): boolean => {
    const kind = $link.kind;
    return kind === "opening" || kind === "handshaking" || kind === "live";
  });
</script>

<div class="flex min-w-0 flex-col gap-wide">
  <div class="flex flex-wrap items-center gap-snug">
    <Button
      label={say($lang, "machine_recheck")}
      tone={planned.length > 0 ? "secondary" : "primary"}
      loading={asking}
      onPress={recheck}
    />
    <!-- wording-ok: the one command this screen exists to hand over;
    a machine spelling, identical in both languages (client-SPEC 4-10) -->
    <code class="rounded-control bg-chrome px-base py-tight font-mono text-note text-text">
      {DOCTOR}
    </code>
    <Copy text={DOCTOR} />
  </div>
  {#if answer !== undefined}
    <Report {answer} onInstall={install} {planned} onInstallAll={installAll} {walk} />
  {:else if reaching}
    <Skeleton />
  {:else}
    <Unchecked onRecheck={recheck} />
  {/if}
</div>
