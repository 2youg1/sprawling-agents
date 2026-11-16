<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  
  
  import { Option, Schema } from "effect";
  import { heldIn } from "../core/belief/rooms";
  import { newestWorking } from "../core/belief/live";
  import { runInFront } from "../core/in_front";

  import { QUERIES } from "../core/asking";
  import { halt, release } from "../core/commands";
  import { LABELS } from "../core/keys";
  import type { Action } from "../core/keys";
  import { canRecord } from "../core/speaking";
  import { LANGS, endonym, say } from "../core/lang";
  import { MAYOR, current, toFragment } from "../core/route";
  import type { View } from "../core/route";
  import { cityIsShut, CITY } from "../core/scope";
  import { RELEASE_ALL, offered } from "../core/slash";
  import { SECTIONS, reached } from "../core/slash_hands";
  import type { Reached, Slash, SlashHands } from "../core/slash_hands";
  import { ui } from "../ui";
  import { Address } from "../wire";
  import type { SessionLine } from "../wire";
  import { modalHold, modalWire } from "./modal";
  import type { Arrival } from "./palette_door.svelte";
  import type { Seat } from "./modal";
  import Modal from "./modal.look.svelte";
  import {
    boxOf,
    carryPlaces,
    directEntry,
    SESSIONS_AT_REST,
    sessionPlaces,
    whyFor,
    workspaceProblem,
  } from "./palette";
  import Look from "./palette.look.svelte";
  import type { Entry, Group } from "./palette/entry";
  import { nothingFound } from "./palette/nothing_found";
  import { openFinder } from "./finding.svelte";
  import Rows from "./palette/rows.svelte";
  import Listening from "./talk/listening.svelte";
  import { noteOf, waitWhy } from "./talk/branch_note";
  import { branchFrom, branchInto } from "./talk/branching";
  import { askToSpeak } from "./talk/speak_asked";
  import { sessionHands } from "./talk/session_hands";

  const u = ui();
  const { lang } = u;
  const belief = u.conn.belief;
  const held = u.prefs.held;
  const effort = u.effort;
  const policy = u.policy;
  const branching = u.branches.held;

  interface Props {
    readonly onClose: () => void;
    
    readonly seat?: Seat | undefined;
    
    
    readonly here?: Address | undefined;
    
    
    readonly arrival?: Arrival | undefined;
    
    readonly initial?: string | undefined;
  }

  const { onClose, seat = "modal", here: named, arrival = "keys", initial = "" }: Props = $props();

  // svelte-ignore state_referenced_locally (a fixture's initial text is captured once)
  let query = $state(initial);
  let cursor = $state(0);
  
  let carrying = $state(false);

  const uid = $props.id();
  const listId = `${uid}-list`;
  
  
  // svelte-ignore state_referenced_locally
  const hold = modalHold(seat, arrival === "touch" ? "sheet" : "control");
  
  
  // svelte-ignore state_referenced_locally
  if (seat === "modal") u.prefs.findPalette();
  const sheet = $derived(
    modalWire({
      seat,
      named: { "aria-label": say($lang, "nav_palette") },
      backdrop: "closes",
      hold,
      onClose: () => {
        onClose();
      },
    }),
  );

  const city = u.conn.asking.ask(QUERIES.city);
  const buildings = $derived(
    $city !== undefined && "city" in $city ? $city.city.buildings : undefined,
  );
  const endpoints = u.conn.asking.ask(QUERIES.endpoints);

  const halted = $derived(cityIsShut($belief.halted));

  
  const viewed = $derived(Option.getOrNull(current(u.bar)));
  const here = $derived(
    named ?? (viewed !== null && viewed.kind === "talk" ? viewed.address : null),
  );
  
  
  let sessionLines = $state.raw<readonly SessionLine[]>([]);
  $effect(() => {
    const room = here;
    sessionLines = [];
    if (room === null) return;
    return u.conn.asking.ask({ sessions: { room } }).subscribe((answer) => {
      sessionLines = answer !== undefined && "sessions" in answer ? answer.sessions.sessions : [];
    });
  });
  const live = $derived(reached(viewed === null ? undefined : runInFront($belief, viewed)));
  
  
  const carryFrom = $derived(
    here !== null &&
      viewed?.kind === "talk" &&
      viewed.session === undefined &&
      (sessionLines[0]?.runs ?? 0) > 0
      ? newest(here)
      : null,
  );

  const entries = $derived.by((): Entry[] => {
    
    
    const goTo = (to: View, label: string, hint = toFragment(to)): Entry => ({
      label,
      hint,
      to,
      act: () => {
        u.go(to);
      },
    });
    
    
    const page = (action: Action, to: View): Entry => ({
      ...goTo(to, say($lang, LABELS[action])),
      action,
    });
    
    
    const viewing = viewed?.kind === "talk" ? viewed.session : undefined;
    const room = here;
    const sessions = (
      room === null ? [] : sessionPlaces({ room, viewing }, sessionLines, $lang, u.now())
    )
      .filter((place, at) => at < SESSIONS_AT_REST || query.trim() !== "")
      .map((place): Entry => {
        const origin = place.origin;
        const opening = origin === null ? null : branchFrom($branching, room ?? MAYOR, origin);
        const note = opening === null ? null : noteOf(opening, room ?? MAYOR, $lang, null);
        return {
          label: place.label,
          hint: note?.said ?? place.hint,
          to: place.to,
          why: opening === null ? undefined : waitWhy(opening),
          keeps: origin === null ? undefined : true,
          act: () => {
            if (origin === null) u.go(place.to);
            else u.branches.open(room ?? MAYOR, { kind: "branch", origin }, onClose);
          },
        };
      });
    const carry: Entry[] =
      carryFrom === null
        ? []
        : [
            {
              label: say($lang, "palette_carry"),
              hint: say($lang, "palette_carry_hint"),
              why: live === null ? undefined : "palette_carry_working",
              keeps: true,
              act: () => {
                carrying = true;
                query = "";
                cursor = 0;
              },
            },
          ];
    const out: Entry[] = [
      ...sessions,
      ...carry,
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
      { label: say($lang, LABELS.finder), hint: "", action: "finder", act: openFinder },
    ];
    
    
    if (here !== null && u.hearing() && canRecord()) {
      out.push({ label: say($lang, "palette_transcribe"), hint: here, act: askToSpeak });
    }
    out.push({
      label: halted ? RELEASE_ALL : say($lang, "city_stop"),
      hint: "halt",
      // wording-ok: `halt` is the command frame's own verb and is
      
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
        out.push(
          building.adoptable
            ? goTo(
                { kind: "raise", address: building.addr },
                building.addr,
                say($lang, "city_adoptable"),
              )
            : goTo(
                { kind: "building", address: building.addr },
                building.addr,
                say($lang, "palette_building"),
              ),
        );
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

  const models = $derived.by(() => {
    const answer = $endpoints;
    if (answer === undefined || !("endpoints" in answer)) return [];
    return answer.endpoints.endpoints.flatMap((endpoint) =>
      endpoint.models.map((row) => ({ endpoint: endpoint.name, model: row.id })),
    );
  });
  function newest(room: string): Reached | null {
    return reached(heldIn($belief, room).at(-1));
  }

  
  function runSlash(chosen: Slash): void {
    
    
    const written: { line: string | null; acted: boolean } = { line: null, acted: false };
    const hands: SlashHands = {
      command: (command) => {
        const sent = u.send(command);
        if (sent) written.acted = true;
        return sent;
      },
      go: (view) => {
        written.acted = true;
        u.go(view);
      },
      here,
      live,
      newest,
      buildings,
      models,
      effort: $effort,
      setEffort: (level) => {
        written.acted = true;
        u.chooseEffort(level);
      },
      policy: $policy,
      setPolicy: u.choosePolicy,
      goal: say($lang, "talk_goal"),
      write: (line) => {
        written.line = line;
        query = line;
        cursor = 0;
      },
      ...sessionHands(u, here, viewed?.kind === "talk" ? viewed.session : undefined),
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
    if (written.line === "" || (written.line === null && written.acted)) {
      onClose();
    }
  }

  
  const grouped = $derived.by((): Group[] => {
    const typed = offered(query.trim());
    return SECTIONS.map((section) => ({
      section,
      entries: typed
        .filter((each) => each.section === section)
        .map((each) => ({
          label: each.grammar === "" ? each.spelling : `${each.spelling} ${each.grammar}`,
          hint: say($lang, each.about),
          why: whyFor(each.spelling, { here, live: live !== null, models: models.length }),
          act: () => {
            runSlash(each);
          },
        })),
    })).filter((group) => group.entries.length > 0);
  });
  const commands = $derived(grouped.flatMap((group) => group.entries));

  
  const carried = $derived.by((): Entry[] => {
    const from = carryFrom;
    if (!carrying || here === null || from === null) return [];
    const known = [...$belief.rooms.keys()].flatMap((room) =>
      Option.toArray(Schema.decodeOption(Address)(room)),
    );
    return carryPlaces({
      here,
      rooms: known,
      typed: Option.getOrNull(Schema.decodeOption(Address)(query.trim())),
      working: (room) => newestWorking($belief, room) !== undefined,
    }).map((place) => {
      const opening = branchInto($branching, place.to);
      return {
        label: place.to,
        hint: noteOf(opening, place.to, $lang, null)?.said ?? "",
        why: place.why ?? waitWhy(opening),
        keeps: true,
        act: () => {
          u.branches.open(
            place.to,
            { kind: "branch", origin: { run: from.run, at_seq: from.at } },
            onClose,
          );
        },
      };
    });
  });

  const shown = $derived.by((): Entry[] => {
    const needle = query.trim().toLowerCase();
    if (needle.startsWith("/") && !carrying) {
      return commands;
    }
    const listed = carrying ? carried : entries;
    const direct =
      carrying || needle.startsWith("/") ? null : directEntry(query, buildings ?? [], $lang);
    const all: Entry[] =
      direct === null
        ? listed
        : [
            {
              ...direct,
              act: () => {
                u.go(direct.to);
              },
            },
            ...listed.filter(
              (entry) => entry.to === undefined || toFragment(entry.to) !== direct.hint,
            ),
          ];
    
    
    const searched = (entry: Entry): string =>
      carrying ? entry.label : `${entry.label} ${entry.hint}`;
    return needle === ""
      ? all
      : all.filter((entry) => searched(entry).toLowerCase().includes(needle));
  });

  function pick(entry: Entry | undefined): void {
    if (entry === undefined || entry.why !== undefined) return;
    entry.act();
    
    
    if (entry.keeps !== true && (carrying || !query.trim().startsWith("/"))) {
      onClose();
    }
  }
</script>


{#snippet listening()}
  {#if here !== null}<Listening room={here} />{/if}
{/snippet}

<Modal wire={sheet} {seat}>
  <Look
    box={boxOf(
      {
        asking: carrying ? "carry" : "places",
        query,
        cursor,
        count: shown.length,
        list: listId,
        lang: $lang,
      },
      {
        write: (line) => {
          query = line;
          cursor = 0;
        },
        point: (at) => {
          cursor = at;
        },
        pick: () => {
          pick(shown.at(cursor));
        },
      },
    )}
    nothing={shown.length === 0
      ? carrying || query.trim().startsWith("/")
        ? nothingFound(query)
        : (workspaceProblem(query, buildings ?? []) ?? nothingFound(query))
      : undefined}
    foot={here !== null && query.trim() === "" && !carrying ? listening : undefined}
  >
    {#snippet list()}
      <Rows
        id={listId}
        listing={query.trim().startsWith("/") && !carrying
          ? { kind: "verbs", groups: grouped }
          : { kind: "places" }}
        {shown}
        {cursor}
        onHover={(at) => {
          cursor = at;
        }}
        onPick={pick}
      />
    {/snippet}
  </Look>
</Modal>
