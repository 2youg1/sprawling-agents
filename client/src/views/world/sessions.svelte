<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->
<script lang="ts">
  import type { Snippet } from "svelte";

  import { say } from "../../core/lang";
  import { collapsedOf, sessionTree } from "../../core/session_tree";
  import { pinningOf, tailOf } from "../../core/stretches";
  import type { Stretch } from "../../core/stretches";
  import { inUse, namedIn, tagsOf } from "../../core/tags";
  import { newestWorking } from "../../core/belief/live";
  import { cancel } from "../../core/commands";
  import type { Named } from "../../core/tags";
  import { ui } from "../../ui";
  import type { Address, Seq, Tag } from "../../wire";
  import { ticker } from "../talk/timing";
  import { dotOf, filterOf, rowOf } from "./sessions";
  import { branchFrom } from "../talk/branching";
  import type { SessionsLook } from "./sessions";
  import Look from "./sessions.look.svelte";
  import { askStretches } from "./stretches.svelte";

  interface Props {
    readonly here: Address;
    readonly session?: Seq | undefined;

    readonly narrow: boolean;

    readonly head: Snippet;
  }

  const { here, session, narrow, head }: Props = $props();

  const u = ui();
  const { lang } = u;
  const belief = u.conn.belief;
  const held = u.tags.held;
  const stretches = askStretches(u);

  const clock = ticker(u.now);

  const city = $derived($belief.city);
  function named(stretch: Stretch): Named | null {
    return namedIn(city, stretch.room, stretch.line.began, stretch.line.workspace ?? null);
  }
  function tagsFor(stretch: Stretch): readonly Tag[] {
    const name = named(stretch);
    return name === null ? [] : tagsOf($held, name);
  }

  let filter = $state<Tag | null>(null);
  const offered = $derived(city === null ? [] : inUse(stretches.all.map(tagsFor)));
  const by = $derived(filter !== null && offered.includes(filter) ? filter : null);
  const filtered = $derived(
    by === null ? stretches.all : stretches.all.filter((stretch) => tagsFor(stretch).includes(by)),
  );
  const branching = u.branches.held;
  let overrides = $state<Readonly<Record<string, boolean>>>({});
  const collapsed = $derived(collapsedOf(filtered, overrides));
  const rows = $derived(sessionTree(filtered, collapsed));

  function chosen(stretch: Stretch): boolean {
    return (
      stretch.room === here &&
      (session === undefined ? stretch.current : stretch.line.began === session)
    );
  }

  const running = $derived(filtered.some((stretch) => dotOf(stretch) === "run"));

  const look: SessionsLook = $derived({
    filter: filterOf(
      offered,
      by,
      { label: say($lang, "world_tags"), all: say($lang, "world_tags_all") },
      (tag) => {
        filter = tag;
      },
    ),
    narrow,
    rows: rows.map((node) => {
      const stretch = node.stretch;
      const tags = tagsFor(stretch);
      const origin = tailOf(stretch);
      const going = newestWorking($belief, stretch.room);
      return rowOf(stretch, {
        lang: $lang,
        now: running ? $clock : u.now(),
        chosen: chosen(stretch),
        tags,
        pinning: pinningOf(stretch, tags),
        named: named(stretch),
        depth: node.depth,
        children: node.children,
        collapsed: collapsed.has(node.key),
        onToggle: () => {
          overrides = { ...overrides, [node.key]: !collapsed.has(node.key) };
        },
        branch: origin === null ? { kind: "none" } : branchFrom($branching, stretch.room, origin),
        open: (from) => {
          u.branches.open(stretch.room, { kind: "branch", origin: from }, () => undefined);
        },
        stop:
          going === undefined
            ? null
            : () => {
                u.send(cancel(going.run));
              },
      });
    }),
  });
</script>

<section
  class="flex min-h-0 flex-1 flex-col overflow-hidden px-snug"
  aria-label={say($lang, "world_sessions")}
>
  {@render head()}
  <Look {...look} />
</section>
