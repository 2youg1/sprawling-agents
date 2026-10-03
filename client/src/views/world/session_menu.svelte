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
  import { tick } from "svelte";

  import { readAnswer } from "../../core/answered";
  import { MODES, WRITE_LIMITS, changeRunPolicy, nameSession } from "../../core/commands";
  import { fill, say } from "../../core/lang";
  import type { Pinning } from "../../core/stretches";
  import { PIN, given, kept, readTag, stripped } from "../../core/tags";
  import type { Named } from "../../core/tags";
  import { ui } from "../../ui";
  import type { RunId, RunPolicy, Tag } from "../../wire";
  import Glyph from "../parts/glyph.svelte";

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

  const u = ui();
  const { lang } = u;
  const held = u.tags.held;
  const seat = $props.id();

  interface Item {
    readonly id: string;
    readonly word: string;
    readonly act: () => void;
  }

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
  let trigger = $state<HTMLButtonElement | undefined>(undefined);
  let field = $state<HTMLInputElement | undefined>(undefined);
  const entries: (HTMLButtonElement | undefined)[] = [];
  const wrong = $derived(naming === "tag" && typed.trim() !== "" && readTag(typed) === null);

  function enter(which: "tag" | "name", start: string): void {
    naming = which;
    typed = start;
    queueMicrotask(() => field?.focus());
  }

  function show(): void {
    open = true;
    naming = null;
    typed = "";
    queueMicrotask(() => entries[0]?.focus());
  }

  function close(): void {
    open = false;
    naming = null;
    void tick().then(() => trigger?.focus());
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

  function walk(event: KeyboardEvent): void {
    const live = entries.filter((entry): entry is HTMLButtonElement => entry !== undefined);
    const now = live.findIndex((entry) => entry === document.activeElement);
    switch (event.key) {
      case "ArrowDown":
        event.preventDefault();
        live[(now + 1) % live.length]?.focus();
        return;
      case "ArrowUp":
        event.preventDefault();
        live[(now - 1 + live.length) % live.length]?.focus();
        return;
      case "Escape":
        event.preventDefault();
        close();
        return;
      case "Tab":
        open = false;
        return;
      default:
        return;
    }
  }

  function submit(event: SubmitEvent): void {
    event.preventDefault();
    if (naming === "name") {
      rename(typed);
      return;
    }
    const tag = readTag(typed);
    if (tag !== null) change(given, tag);
  }
</script>

<div class="relative">
  <button
    bind:this={trigger}
    type="button"
    class="grid size-control-sm place-items-center rounded-control text-text-quiet hover:wash hover:text-text aria-expanded:wash aria-expanded:text-text disabled:text-text-disabled"
    aria-haspopup="menu"
    aria-expanded={open}
    aria-controls="{seat}-menu"
    aria-label={fill(say($lang, "world_row_menu"), { room: label })}
    title={named === null ? say($lang, "world_tags_offline") : undefined}
    disabled={named === null}
    onclick={() => {
      if (open) close();
      else show();
    }}
    onkeydown={(event) => {
      if (event.key === "ArrowDown" && !open) {
        event.preventDefault();
        show();
      }
    }}
  >
    <Glyph name="more" size="sm" />
  </button>
  {#if open}
    <div
      id="{seat}-menu"
      class="absolute top-full right-0 z-10 flex min-w-[20ch] flex-col rounded-card bg-raised p-tight shadow-float"
      onfocusout={(event) => {
        if (!(event.relatedTarget instanceof Node && event.currentTarget.contains(event.relatedTarget))) {
          open = false;
          naming = null;
        }
      }}
    >
      {#if naming !== null}
        <form class="flex flex-col gap-tight p-tight" onsubmit={submit}>
          <input
            bind:this={field}
            bind:value={typed}
            class="h-control-sm rounded-control bg-page px-snug text-note text-text"
            aria-label={say($lang, naming === "tag" ? "world_tag_name" : "world_rename_field")}
            aria-invalid={wrong}
            aria-describedby="{seat}-hint"
            maxlength={naming === "tag" ? 24 : NAME_MAX}
            onkeydown={(event) => {
              if (event.key === "Escape") {
                event.preventDefault();
                close();
              }
            }}
          />
          <p id="{seat}-hint" class={["text-note", wrong ? "text-alert" : "text-text-faint"]}>{say($lang, naming === "tag" ? "world_tag_hint" : "world_rename_hint")}</p>
        </form>
      {:else}
        <ul role="menu" tabindex="-1" aria-label={fill(say($lang, "world_row_menu"), { room: label })} onkeydown={walk}>
          {#each items as item, index (item.id)}
            <li role="none">
              <button
                bind:this={entries[index]}
                type="button"
                role="menuitem"
                class="flex h-control-sm w-full items-center rounded-control px-snug text-left text-note text-text hover:wash focus-visible:wash"
                onclick={item.act}
              >
                {item.word}
              </button>
            </li>
          {/each}
        </ul>
      {/if}
    </div>
  {/if}
</div>
