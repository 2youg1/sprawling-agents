<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // One box that reaches everything the shell does not show: every page,
  // every building, every room with a run in it, the brake, the
  // language, speaking into the composer where the city transcribes,
  // and - the moment a line begins with `/` - every verb this client
  // understands. The verbs are not a list of their own: they
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
  // reason is drawn where the row's hint is (`palette/rows.svelte`).
  //
  // **The box is a modal the platform owns** (`modal.ts`), as the file
  // finder is: the focus stays inside while it stands and goes back
  // where it was when it closes (client/Spec.lean §4-64b). The box is an
  // APG Combobox over the rows, so the row under the cursor is the one
  // a screen reader hears. What the box and the list look like is
  // `palette.look.svelte` and `palette/rows.look.svelte`.
  import { Option, Schema } from "effect";
  import { heldIn } from "../core/belief/rooms";
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
  import { modalHold, modalWire } from "./modal";
  import type { Seat } from "./modal";
  import Modal from "./modal.look.svelte";
  import { boxOf, whyFor } from "./palette";
  import Look from "./palette.look.svelte";
  import type { Entry, Group } from "./palette/entry";
  import { nothingFound } from "./palette/nothing_found";
  import { openFinder } from "./finding.svelte";
  import Rows from "./palette/rows.svelte";
  import { askToSpeak } from "./talk/speak_asked";
  import { sessionHands } from "./talk/session_hands";

  const u = ui();
  const { lang } = u;
  const belief = u.conn.belief;
  const held = u.prefs.held;
  const effort = u.effort;
  const policy = u.policy;

  interface Props {
    readonly onClose: () => void;
    // `specimen` draws the box open in the page's flow, for `#/gallery`.
    readonly seat?: Seat | undefined;
  }

  const { onClose, seat = "modal" }: Props = $props();

  let query = $state("");
  let cursor = $state(0);

  const uid = $props.id();
  const listId = `${uid}-list`;
  // Where the box stands is fixed for its life, so the attachment that
  // opens it is made once.
  // svelte-ignore state_referenced_locally
  const hold = modalHold(seat);
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
  const endpoints = u.conn.asking.ask(QUERIES.endpoints);

  const halted = $derived(cityIsShut($belief.halted));

  // What a verb typed here may reach. The palette is attached to no
  // room, so `here` is the room the address bar names and `live` is
  // the run in front of the person (`core/in_front`).
  const viewed = $derived(Option.getOrNull(current(u.bar)));
  const here = $derived(viewed !== null && viewed.kind === "talk" ? viewed.address : null);
  const live = $derived(reached(viewed === null ? undefined : runInFront($belief, viewed)));

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
    // A page a key reaches is called what the key sheet calls it, and
    // shows the chord rather than its address.
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
      { label: say($lang, LABELS.finder), hint: "", action: "finder", act: openFinder },
    ];
    // Speaking from the keyboard: the composer on this page records,
    // exactly as its microphone does, where the city transcribes
    // (client/Spec.lean §4-64b).
    if (here !== null && u.hearing() && canRecord()) {
      out.push({ label: say($lang, "palette_transcribe"), hint: here, act: askToSpeak });
    }
    out.push({
      label: halted ? RELEASE_ALL : say($lang, "city_stop"),
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

  // A verb runs, and the box closes once it has done something. It stays
  // open when the verb put words back in it - which is what `/help`
  // does - and when it did nothing at all, as a verb given an address it
  // cannot read does: the line stays for the person to correct, as it
  // does in the composer (client/Spec.lean §4-41).
  function runSlash(chosen: Slash): void {
    // A record rather than bare variables, because the closures below
    // write it and only the caller reads it back.
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

  // The slash list as three labelled sections, in the order the
  // question is usually asked: what to do, where to go, what session.
  // Each verb names its section itself, and the flat list the cursor
  // walks is read in the same order the sections draw it.
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

<!-- Escape reaches the box as a cancel request and the shell's key
handler as a key; both close it. A click on the dimmed page is the
pointer's extra way out, not the only one. -->
<Modal wire={sheet} {seat}>
  <Look
    box={boxOf(
      { query, cursor, count: shown.length, list: listId, lang: $lang },
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
    nothing={shown.length === 0 ? nothingFound(query) : undefined}
  >
    {#snippet list()}
      <Rows
        id={listId}
        listing={query.trim().startsWith("/") ? { kind: "verbs", groups: grouped } : { kind: "places" }}
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
