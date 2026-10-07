<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // One session row's menu (APG Menu Button, as `pane_menu.svelte`): pin
  // or unpin, take each tag off, add a tag, rename the session, and - for
  // a room's current session - switch its mode or its write limit from
  // the run's next safe point (`Command::ChangeRunPolicy`, kernel D21;
  // model and effort stay what the session opened with). Enter, Space or Down opens it
  // on its first item, Up and Down walk the items, Escape or Tab closes it
  // with the focus back on the button. "Add a tag" and "rename" each turn
  // the menu into one field: Enter gives the session the word or the
  // name and closes, Escape closes without it, and the focus goes back to
  // the button either way.
  //
  // The Mayor's current session is pinned by being current, not by a
  // tag, so its menu offers no way to unpin it.
  //
  // This file is the seat: it decides the items and what each does,
  // owns whether the menu is open and the field it turns into, and
  // draws whatever `./session_menu.look.svelte` is; the keys are
  // `./menu.ts`.
  import { tick } from "svelte";

  import { readAnswer } from "../../core/answered";
  import { MODES, WRITE_LIMITS, changeRunPolicy, nameSession } from "../../core/commands";
  import { fill, say } from "../../core/lang";
  import type { Pinning } from "../../core/stretches";
  import { PIN, given, kept, readTag, stripped } from "../../core/tags";
  import type { Named } from "../../core/tags";
  import { ui } from "../../ui";
  import type { RunId, RunPolicy, Tag } from "../../wire";
  import { drawnElements } from "./drawn";
  import { sessionMenuLookOf } from "./session_menu";
  import type { Item, SessionMenuLook } from "./session_menu";
  import Look from "./session_menu.look.svelte";

  interface Props {
    // The session as its tags name it; null where the city has no name
    // to keep tags under, which leaves the button disabled.
    readonly named: Named | null;
    // The row's own name, which the button's accessible name carries.
    readonly label: string;
    readonly tags: readonly Tag[];
    readonly pinning: Pinning;
    // The session's display name ("" for none), and the run whose
    // policy a change starts from: the room's current run, or null for
    // a session the city no longer goes on with.
    readonly session: { readonly name: string; readonly run: RunId | null };
  }

  const { named, label, tags, pinning, session }: Props = $props();

  // The longest display name the field takes: a row shows a line of it.
  const NAME_MAX = 80;
  // The longest tag the field takes, as the hint under it says.
  const TAG_MAX = 24;

  const u = ui();
  const { lang } = u;
  const held = u.tags.held;
  const seat = $props.id();

  // The policy the run opened under, asked only while the menu is open;
  // until it is held the menu offers no policy change, because a change
  // carries the whole policy and a guessed field would be sent with it.
  let opened = $state<RunPolicy | null>(null);
  $effect(() => {
    const run = session.run;
    opened = null;
    if (!open || run === null) return;
    return u.conn.asking.ask({ rounds: { run } }).subscribe((answer) => {
      const read = readAnswer(answer, (held) => ("rounds" in held ? held.rounds.opening?.policy ?? null : undefined));
      opened = read.kind === "held" ? read.value : null;
    });
  });

  // The policy now in force: the room's newest change inside this
  // session (`run_policy_changed`), else the one the run opened under.
  const belief = u.conn.belief;
  const policy = $derived.by((): RunPolicy | null => {
    if (named === null || session.run === null) return null;
    const changed = $belief.policies[named.room] ?? null;
    return changed !== null && changed.seq > named.began ? changed.policy : opened;
  });

  function policyItems(now: RunPolicy): Item[] {
    const mode = MODES.find((each) => each !== now.mode);
    const write = WRITE_LIMITS.find((each) => each !== now.write);
    return [
      ...(mode === undefined
        ? []
        : [{ id: "mode", word: fill(say($lang, "world_mode_to"), { mode: say($lang, `mode_${mode}`) }), act: () => { repolicy({ ...now, mode }); } }]),
      ...(write === undefined
        ? []
        : [{ id: "write", word: fill(say($lang, "world_write_to"), { limit: say($lang, `admission_value_${write}`) }), act: () => { repolicy({ ...now, write }); } }]),
    ];
  }

  const items = $derived.by((): Item[] => [
    ...(pinning === "mayor"
      ? []
      : [{ id: "pin", word: say($lang, pinning === "tagged" ? "world_unpin" : "world_pin"), act: () => { change(pinning === "tagged" ? stripped : given, PIN); } }]),
    ...(named === null || !kept($held, named) ? [] : tags).filter((tag) => tag !== PIN).map((tag) => ({
      id: `strip-${tag}`,
      word: fill(say($lang, "world_tag_remove"), { tag }),
      act: () => { change(stripped, tag); },
    })),
    { id: "add", word: say($lang, "world_tag_add"), act: () => { enter("tag", ""); } },
    { id: "rename", word: say($lang, "world_rename"), act: () => { enter("name", session.name); } },
    ...(policy === null ? [] : policyItems(policy)),
  ]);

  let open = $state(false);
  // Which field the menu has turned into, if any.
  let naming = $state<"tag" | "name" | null>(null);
  let typed = $state("");
  const drawn = drawnElements();
  const wrong = $derived(naming === "tag" && typed.trim() !== "" && readTag(typed) === null);

  function live(): HTMLElement[] {
    return items.flatMap((item) => drawn.get(item.id) ?? []);
  }

  function enter(which: "tag" | "name", start: string): void {
    naming = which;
    typed = start;
    queueMicrotask(() => drawn.get("field")?.focus());
  }

  function show(): void {
    open = true;
    naming = null;
    typed = "";
    queueMicrotask(() => live()[0]?.focus());
  }

  function close(): void {
    open = false;
    naming = null;
    void tick().then(() => drawn.get("trigger")?.focus());
  }

  function change(how: typeof given, tag: Tag): void {
    if (named !== null) u.tags.retag(how($held, named, tag));
    close();
  }

  function rename(name: string): void {
    if (named !== null) u.conn.command(nameSession(named.room, named.began, name));
    close();
  }

  function repolicy(next: RunPolicy): void {
    if (named !== null) u.conn.command(changeRunPolicy(named.room, next));
    close();
  }

  function submit(): void {
    if (naming === "name") {
      rename(typed);
      return;
    }
    const tag = readTag(typed);
    if (tag !== null) change(given, tag);
  }

  const look: SessionMenuLook = $derived(
    sessionMenuLookOf(
      {
        uid: seat,
        open,
        why: named === null ? say($lang, "world_tags_offline") : undefined,
        name: fill(say($lang, "world_row_menu"), { room: label }),
        items,
        naming:
          naming === null
            ? null
            : {
                label: say($lang, naming === "tag" ? "world_tag_name" : "world_rename_field"),
                hint: say($lang, naming === "tag" ? "world_tag_hint" : "world_rename_hint"),
                typed,
                wrong,
                max: naming === "tag" ? TAG_MAX : NAME_MAX,
              },
      },
      {
        live,
        show,
        close,
        leave: () => {
          open = false;
          naming = null;
        },
        type: (next) => {
          typed = next;
        },
        submit,
        hold: drawn.hold,
      },
    ),
  );
</script>

<Look {...look} />
