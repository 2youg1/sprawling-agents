<script lang="ts" module>
  // This Source Code Form is subject to the terms of the Mozilla Public
  // License, v. 2.0. If a copy of the MPL was not distributed with this
  // file, You can obtain one at https://mozilla.org/MPL/2.0/.
  // Copyright (c) 2026 2youg1 and the sprawling contributors
  export { default as MachineReport } from "./machine/report.svelte";
  export { default as MachineSkeleton } from "./machine/skeleton.svelte";
  export { default as MachineUnchecked } from "./machine/unchecked.svelte";

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

  $effect(() => {
    if (answer !== undefined) {
      asking = false;
    }
  });

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

  function recheck(): void {
    asking = true;
    u.send(doctorRefresh());
    u.conn.asking.refresh(QUERIES.doctor);
  }

  let opened = false;
  $effect(() => {
    const live = $link.kind === "live";
    if (opened || !live) return;
    opened = true;
    recheck();
  });

  let newest = $state.raw<Readonly<Record<string, DoctorNewest>>>({});
  const names = $derived(
    answer === undefined ? "" : answer.items.map((each) => each.name).join(" "),
  );
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

    <code
      class="flex h-control items-center rounded-control border border-edge px-base font-mono text-note text-text"
    >
      {DOCTOR}
    </code>
    <Copy text={DOCTOR} />
  </div>
  {#if answer !== undefined}
    <Report
      {answer}
      onInstall={install}
      {planned}
      onInstallAll={installAll}
      {walk}
      {newest}
      onInstallPack={installPack}
    />
  {:else if reaching}
    <Skeleton />
  {:else}
    <Unchecked onRecheck={recheck} />
  {/if}
</div>
