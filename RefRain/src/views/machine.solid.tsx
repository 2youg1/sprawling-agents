// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createEffect, createMemo, createSignal, Match, Switch } from "solid-js";
import { QUERIES } from "../core/asking";
import { doctorInstall, doctorRefresh } from "../core/commands";
import { say } from "../core/lang";
import { storeValue } from "../core/solid_store";
import { ui } from "../ui";
import type { DoctorAnswer, DoctorNewest } from "../wire";
import Report from "./machine/report.solid";
import Skeleton from "./machine/skeleton.solid";
import Unchecked from "./machine/unchecked.solid";
import Button from "./parts/button.solid";
import Copy from "./parts/copy.solid";
import { answered, plan, refused, running, started, type Walk } from "./setup/installing";
import { stillAsking } from "./setup/versions";

export { Report as MachineReport, Skeleton as MachineSkeleton, Unchecked as MachineUnchecked };

const DOCTOR = "sprawling doctor --install";
const ASK_AGAIN_MS = 1500;

export default function Machine() {
  const u = ui();
  const lang = storeValue(u.lang);
  const link = storeValue(u.conn.state);
  const belief = storeValue(u.conn.belief);
  const held = storeValue(u.conn.asking.ask(QUERIES.doctor));
  const answer = createMemo<DoctorAnswer | undefined>(() => {
    const current = held();
    return current !== undefined && "doctor" in current ? current.doctor : undefined;
  });
  const [asking, setAsking] = createSignal(false);
  createEffect(answer, (current) => {
    if (current !== undefined) setAsking(false);
  });

  const looked = createMemo(
    () => belief().logs.filter((line) => line.module === "bin::doctor").length,
  );
  let seenAt = -1;
  createEffect(looked, (now) => {
    if (seenAt >= 0 && now !== seenAt) u.conn.asking.refresh(QUERIES.doctor);
    seenAt = now;
  });

  function recheck(): void {
    setAsking(true);
    u.send(doctorRefresh());
    u.conn.asking.refresh(QUERIES.doctor);
  }

  let opened = false;
  createEffect(
    () => link().kind === "live",
    (live) => {
      if (opened || !live) return;
      opened = true;
      recheck();
    },
  );

  const [newest, setNewest] = createSignal<Readonly<Record<string, DoctorNewest>>>({});
  const names = createMemo(
    () =>
      answer()
        ?.items.map((item) => item.name)
        .join(" ") ?? "",
  );
  createEffect(names, (known) => {
    if (known === "") return;
    const stops = known.split(" ").map((item) =>
      u.conn.asking.ask({ upstream_version: { item } }).subscribe((reply) => {
        if (reply !== undefined && "upstream" in reply) {
          setNewest((old) => ({ ...old, [reply.upstream.item]: reply.upstream.newest }));
        }
      }),
    );
    return () => {
      for (const stop of stops) stop();
    };
  });
  const pending = createMemo(() => stillAsking(newest()).join(" "));
  createEffect(pending, (asked) => {
    if (asked === "") return;
    const timer = setTimeout(() => {
      for (const item of asked.split(" ")) u.conn.asking.refresh({ upstream_version: { item } });
    }, ASK_AGAIN_MS);
    return () => clearTimeout(timer);
  });

  function install(item: string): void {
    setAsking(true);
    u.send(doctorInstall(item));
    u.conn.asking.refresh(QUERIES.doctor);
  }
  const [walk, setWalk] = createSignal<Walk | null>(null);
  const planned = createMemo(() => {
    const current = answer();
    return current === undefined ? [] : plan(current);
  });
  let sent: string | null = null;
  function installPack(names: readonly string[]): void {
    sent = null;
    setWalk(started(names));
  }
  createEffect(answer, (now) => {
    const current = walk();
    if (now !== undefined && current !== null) setWalk(answered(current, now));
  });
  createEffect(
    () => belief().refusal,
    (now) => {
      const current = walk();
      if (now !== null && current !== null) setWalk(refused(current, now));
    },
  );
  createEffect(walk, (current) => {
    const next = current === null ? null : running(current);
    if (next === null || next === sent) return;
    sent = next;
    install(next);
  });

  const reaching = () => {
    const kind = link().kind;
    return kind === "opening" || kind === "handshaking" || kind === "live";
  };
  return (
    <div class="flex min-w-0 flex-col gap-wide">
      <div class="flex flex-wrap items-center gap-snug">
        <Button
          label={say(lang(), "machine_recheck")}
          tone={planned().length > 0 ? "secondary" : "primary"}
          loading={asking()}
          onPress={recheck}
        />
        <code class="flex h-control items-center rounded-control border border-edge px-base font-mono text-note text-text">
          {DOCTOR}
        </code>
        <Copy text={DOCTOR} />
      </div>
      <Switch fallback={<Unchecked onRecheck={recheck} />}>
        <Match when={answer()}>
          {(current) => (
            <Report
              answer={current()}
              onInstall={install}
              planned={planned()}
              onInstallAll={() => installPack(planned())}
              walk={walk()}
              newest={newest()}
              onInstallPack={installPack}
            />
          )}
        </Match>
        <Match when={reaching()}>
          <Skeleton />
        </Match>
      </Switch>
    </div>
  );
}
