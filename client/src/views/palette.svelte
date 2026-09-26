<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // One box that reaches everything the bar does not show: every page,
  // every building, every room with a run in it, the brake, the
  // language, and - the moment a line begins with `/` - every verb this
  // client understands. The verbs are not a list of their own: they
  // come from `core/slash.ts`, which is the same table the composer's
  // `/` menu reads, so a spelling learned in one place works in the
  // other.
  //
  // **The verbs are grouped by what they reach for** - things to do,
  // places to go, sessions to open or feed (ux 7-8) - because thirteen
  // spellings in one column is a list nobody scans twice. The grouping
  // is this screen's arrangement of the one table, never a second list.
  //
  // **A verb this place cannot run is greyed with its reason, never
  // hidden** (ux 7-8): `/steer` with nothing running and `/dispatch`
  // outside a room do nothing rather than guess (`SlashHands`), and a
  // list that omits them teaches that the verb does not exist. The
  // reason is drawn where the row's hint is, and the row stays
  // focusable with `aria-disabled` so the reason is reachable (7-2).
  import { Option, Schema } from "effect";
  import { onMount } from "svelte";
  import { heldIn } from "../core/belief/rooms";

  import { QUERIES } from "../core/asking";
  import { halt, release } from "../core/commands";
  import { LABELS } from "../core/keys";
  import type { Action } from "../core/keys";
  import type { Key } from "../core/lang";
  import { LANGS, endonym, say } from "../core/lang";
  import { MAYOR, current, toFragment } from "../core/route";
  import type { View } from "../core/route";
  import { cityIsShut, CITY } from "../core/scope";
  import { completed } from "../core/completion";
  import { SECTIONS, offered, reached } from "../core/slash";
  import type { Reached, Section, Slash, SlashHands } from "../core/slash";
  import { ui } from "../ui";
  import { Address } from "../wire";
  import Empty from "./parts/empty.svelte";
  import { SECTION_WORD } from "./palette/sections";
  import { Kbd } from "./parts/kbd.svelte";

  interface Entry {
    readonly label: string;
    readonly hint: string;
    // Present on a page a key reaches: the row shows that key's chord
    // where other rows show their hint.
    readonly action?: Action | undefined;
    readonly act: () => void;
    // Present means the verb cannot run here, and names why.
    readonly why?: Key | undefined;
  }

  const u = ui();
  const { lang } = u;
  const belief = u.conn.belief;
  const held = u.prefs.held;
  const effort = u.effort;

  const { onClose }: { readonly onClose: () => void } = $props();

  let query = $state("");
  let cursor = $state(0);
  let box = $state<HTMLInputElement | undefined>(undefined);

  onMount(() => {
    box?.focus();
  });

  const city = u.conn.asking.ask(QUERIES.city);
  const endpoints = u.conn.asking.ask(QUERIES.endpoints);

  const halted = $derived(cityIsShut($belief.halted));

  const entries = $derived.by((): Entry[] => {
    // The hint beside a page is the address bar's own spelling of it,
    // read from the router rather than written again here.
    const goTo = (to: View, label: string, hint = toFragment(to)): Entry => ({
      label,
      hint,
      act: () => {
        u.go(to);
      },
    });
    // A page a key reaches is called what the key sheet and the rail
    // call it, and shows the chord rather than its address.
    const page = (action: Action, to: View): Entry => ({ ...goTo(to, say($lang, LABELS[action])), action });
    const out: Entry[] = [
      page("go.talk", { kind: "talk", address: MAYOR }),
      page("go.city", { kind: "city" }),
      page("go.setup", { kind: "setup" }),
      page("go.mcp", { kind: "mcp" }),
      page("go.registry", { kind: "registry" }),
      page("go.record", { kind: "record", lens: "ledger" }),
      goTo({ kind: "record", lens: "archive" }, say($lang, "rec_archive")),
      goTo({ kind: "record", lens: "bin" }, say($lang, "rec_bin")),
      page("go.cost", { kind: "cost" }),
      goTo({ kind: "welcome" }, say($lang, "setup_rerun")),
    ];
    out.push({
      label: halted ? say($lang, "city_release") : say($lang, "city_stop"),
      hint: "halt",
      // wording-ok: `halt` is the command frame's own verb and is
      // spelled alike in both languages (design 4-10).
      act: () => {
        u.send(halted ? release(CITY) : halt(CITY));
      },
    });
    for (const tongue of LANGS) {
      if (tongue !== $held.lang) {
        out.push({
          label: endonym(tongue),
          hint: say($lang, "setup_language"),
          act: () => {
            u.prefs.setLang(tongue);
          },
        });
      }
    }
    const answer = $city;
    if (answer !== undefined && "city" in answer) {
      for (const building of answer.city.buildings) {
        out.push(goTo({ kind: "building", address: building.addr }, building.addr, say($lang, "palette_building")));
      }
    }
    for (const room of $belief.rooms.keys()) {
      if (room === MAYOR) continue;
      const address = Option.getOrNull(Schema.decodeOption(Address)(room));
      if (address !== null) {
        out.push(goTo({ kind: "talk", address }, room, say($lang, "palette_room")));
      }
    }
    return out;
  });

  // What a verb typed here may reach. The palette is attached to no
  // room, so `here` is whatever room the address bar names and `live`
  // is the newest run still going anywhere.
  const models = $derived.by(() => {
    const answer = $endpoints;
    if (answer === undefined || !("endpoints" in answer)) return [];
    return answer.endpoints.endpoints.flatMap((endpoint) =>
      endpoint.models.map((row) => ({ endpoint: endpoint.name, model: row.id })),
    );
  });
  const here = $derived.by((): Address | null => {
    const at = Option.getOrNull(current(u.bar));
    return at !== null && at.kind === "talk" ? at.address : null;
  });
  const live = $derived(reached($belief.live.at(-1)));

  function newest(room: string): Reached | null {
    return reached(heldIn($belief, room).at(-1));
  }

  // Why a verb cannot run from this box, as a `lang.json` key. Every
  // verb is reachable; the ones missing a capability say which.
  const NEEDS_ROOM: Key = "palette_needs_room";
  const NEEDS_RUN: Key = "palette_needs_run";
  const NEEDS_MODEL: Key = "palette_needs_model";

  function whyFor(spelling: string): Key | undefined {
    switch (spelling) {
      case "/dispatch":
        return here === null ? NEEDS_ROOM : undefined;
      case "/steer":
      case "/stop":
        return live === null ? NEEDS_RUN : undefined;
      case "/model":
        return models.length === 0 ? NEEDS_MODEL : undefined;
      default:
        return undefined;
    }
  }

  // A verb runs, and the box closes unless the verb put words back in
  // it - which is what `/help` does, and the one reason to stay open.
  function runSlash(chosen: Slash): void {
    // A record rather than a bare variable, because the closure below
    // writes it and only the caller reads it back.
    const written: { line: string | null } = { line: null };
    const hands: SlashHands = {
      command: u.send,
      go: u.go,
      here,
      live,
      newest,
      models,
      effort: $effort,
      setEffort: u.chooseEffort,
      goal: say($lang, "talk_goal"),
      write: (line) => {
        written.line = line;
        query = line;
        cursor = 0;
      },
    };
    const line = query.trim();
    const needs = chosen.grammar.startsWith("<");
    const cut = line.search(/\s/);
    const verb = cut < 0 ? line : line.slice(0, cut);
    const rest = cut < 0 ? "" : line.slice(cut + 1).trim();
    if (verb !== chosen.spelling || (needs && rest === "")) {
      query = `${chosen.spelling} `;
      cursor = 0;
      return;
    }
    chosen.run(hands, { verb, words: rest === "" ? [] : rest.split(/\s+/), rest });
    if (written.line === null || written.line === "") {
      onClose();
    }
  }

  // The slash list as three labelled sections, in the order the
  // question is usually asked: what to do, where to go, what session.
  // Each verb names its section itself, and the flat list the cursor
  // walks is read in the same order the sections draw it.
  const grouped = $derived.by((): { section: Section; entries: Entry[] }[] => {
    const typed = offered(query.trim());
    return SECTIONS.map((section) => ({
      section,
      entries: typed
        .filter((each) => each.section === section)
        .map((each) => ({
          label: each.grammar === "" ? each.spelling : `${each.spelling} ${each.grammar}`,
          hint: say($lang, each.about),
          why: whyFor(each.spelling),
          act: () => {
            runSlash(each);
          },
        })),
    })).filter((group) => group.entries.length > 0);
  });
  const commands = $derived(grouped.flatMap((group) => group.entries));

  const shown = $derived.by((): Entry[] => {
    const needle = query.trim().toLowerCase();
    if (needle.startsWith("/")) {
      return commands;
    }
    const all = entries;
    return needle === ""
      ? all
      : all.filter((entry) => `${entry.label} ${entry.hint}`.toLowerCase().includes(needle));
  });

  function pick(entry: Entry | undefined): void {
    if (entry === undefined || entry.why !== undefined) return;
    entry.act();
    // A verb decides for itself whether the box stays; everything else
    // is a place to go, and going there closes it.
    if (!query.trim().startsWith("/")) {
      onClose();
    }
  }
</script>

<!-- Escape, answered by the shell's key handler, closes this box; the
click on the scrim is the pointer's extra way out, not the only one. -->
<div
  class="fixed inset-0 flex items-start justify-center bg-page/70 pt-section"
  role="presentation"
  onclick={(event) => {
    if (event.target === event.currentTarget) onClose();
  }}
>
  <div
    class="w-full max-w-measure rounded-panel bg-raised p-snug shadow-float"
    role="dialog"
    aria-label={say($lang, "nav_palette")}
  >
    <input
      bind:this={box}
      class="w-full rounded-control bg-raised px-base py-snug text-body placeholder:text-text-disabled"
      placeholder={say($lang, "palette_placeholder")}
      value={query}
      oninput={(event) => {
        query = event.currentTarget.value;
        cursor = 0;
      }}
      onkeydown={(event) => {
        if (event.key === "Tab" && query.trim().startsWith("/")) {
          event.preventDefault();
          query = completed(query.trim());
          cursor = 0;
        } else if (event.key === "ArrowDown") {
          event.preventDefault();
          cursor = Math.min(cursor + 1, shown.length - 1);
        } else if (event.key === "ArrowUp") {
          event.preventDefault();
          cursor = Math.max(cursor - 1, 0);
        } else if (event.key === "Enter") {
          event.preventDefault();
          pick(shown.at(cursor));
        }
      }}
    />
    <ul class="mt-snug max-h-palette overflow-y-auto">
      {#if query.trim().startsWith("/")}
        {#each grouped as group (group.section)}
          <li>
            <h2 class="px-base pt-snug text-label font-label text-text-faint">
              {say($lang, SECTION_WORD[group.section])}
            </h2>
          </li>
          {#each group.entries as entry (entry.label)}
            {@const at = shown.indexOf(entry)}
            <li>
              <button
                type="button"
                class={[
                  "flex w-full items-center justify-between gap-snug rounded-control px-base py-snug text-left text-body",
                  entry.why === undefined ? "hover:bg-raised" : "text-text-disabled",
                  at === cursor ? "bg-raised" : "",
                ]}
                aria-disabled={entry.why !== undefined}
                onmouseenter={() => {
                  if (at >= 0) cursor = at;
                }}
                onclick={() => {
                  pick(entry);
                }}
              >
                <span class="truncate font-mono">{entry.label}</span>
                <span class="shrink-0 text-note text-text-disabled">
                  {entry.why === undefined ? entry.hint : say($lang, entry.why)}
                </span>
              </button>
            </li>
          {/each}
        {/each}
      {:else}
        {#each shown as entry, at (entry.label)}
          <li>
            <button
              type="button"
              class={[
                "flex w-full items-center justify-between gap-snug rounded-control px-base py-snug text-left text-body hover:bg-raised",
                at === cursor ? "bg-raised" : "",
              ]}
              onmouseenter={() => {
                cursor = at;
              }}
              onclick={() => {
                pick(entry);
              }}
            >
              <span class="truncate font-mono">{entry.label}</span>
              <span class="shrink-0 text-note text-text-disabled">
                {#if entry.action === undefined}
                  {entry.hint}
                {:else}
                  <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression, @typescript-eslint/no-unsafe-call (a snippet call is the render itself; svelte-check types this imported snippet fine, and typescript-eslint does not resolve exports of another .svelte module) -->
                  {@render Kbd({ action: entry.action })}
                {/if}
              </span>
            </button>
          </li>
        {/each}
      {/if}
    </ul>
    {#if shown.length === 0}
      <Empty missing="part_no_match" />
    {/if}
  </div>
</div>
