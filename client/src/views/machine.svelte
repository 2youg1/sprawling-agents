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
// shows the answer is the page that asks for it - once per opening,
// whether or not the city holds an older answer, and through the same
// command the button sends, so "look at this machine" keeps one
// authority (`crates/sprawling/Spec.lean` §8-120). Once an answer is here, each
// item's newest release is asked of its publisher, one question per
// item, and filled in as the answers arrive.

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
  import type { DoctorAnswer, DoctorNewest } from "../wire";
  import { answered, plan, refused, running, started, type Walk } from "./setup/installing";
  import { stillAsking } from "./setup/versions";
  import Button from "./parts/button.svelte";
  import Copy from "./parts/copy.svelte";
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
  // then brings the answer in, which is why it stays a control the
  // person can press beside the check the page makes when it opens.
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

  // Every opening checks this machine at once, once the link can carry
  // the command; an answer the city already holds is drawn meanwhile.
  let opened = false;
  $effect(() => {
    const live = $link.kind === "live";
    if (opened || !live) return;
    opened = true;
    recheck();
  });

  // Each item's newest release, by name. Keyed on the list of names, so
  // a fresh answer about the same items asks nothing again.
  let newest = $state.raw<Readonly<Record<string, DoctorNewest>>>({});
  const names = $derived(answer === undefined ? "" : answer.items.map((each) => each.name).join(" "));
  $effect(() => {
    const asked = names === "" ? [] : names.split(" ");
    const stops = asked.map((item) =>
      u.conn.asking.ask({ upstream_version: { item } }).subscribe((now) => {
        if (now === undefined || !("upstream" in now)) return;
        newest = { ...untrack(() => newest), [now.upstream.item]: now.upstream.newest };
      }),
    );
    return () => {
      for (const stop of stops) stop();
    };
  });

  // The city answers `asking` while it reads a publisher, rather than
  // holding every other question of this page behind the network; the
  // items it is still reading are asked again after a pause, until each
  // has its reading (`crates/sprawling/Spec.lean` §8-120).
  const ASK_AGAIN_MS = 1500;
  const pending = $derived(stillAsking(newest).join(" "));
  $effect(() => {
    if (pending === "") return;
    const again = setTimeout(() => {
      for (const item of pending.split(" ")) u.conn.asking.refresh({ upstream_version: { item } });
    }, ASK_AGAIN_MS);
    return () => {
      clearTimeout(again);
    };
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

  function installPack(names: readonly string[]): void {
    walk = started(names);
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
    a machine spelling, identical in both languages (client/Spec.lean §4-10) -->
    <code class="flex h-control items-center rounded-control border border-edge px-base font-mono text-note text-text">
      {DOCTOR}
    </code>
    <Copy text={DOCTOR} />
  </div>
  {#if answer !== undefined}
    <Report {answer} onInstall={install} {planned} onInstallAll={installAll} {walk} {newest} onInstallPack={installPack} />
  {:else if reaching}
    <Skeleton />
  {:else}
    <Unchecked onRecheck={recheck} />
  {/if}
</div>
