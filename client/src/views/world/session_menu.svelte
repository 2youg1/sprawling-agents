<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // One session row's menu (APG Menu Button, as `pane_menu.svelte`): pin
  // or unpin, take each tag off, add a tag. Enter, Space or Down opens it
  // on its first item, Up and Down walk the items, Escape or Tab closes it
  // with the focus back on the button. "Add a tag" turns the menu into one
  // field: Enter gives the session the word and closes, Escape closes
  // without it, and the focus goes back to the button either way.
  //
  // The Mayor's current session is pinned by being current, not by a
  // tag, so its menu offers no way to unpin it.
  import { tick } from "svelte";

  import { fill, say } from "../../core/lang";
  import type { Pinning } from "../../core/stretches";
  import { PIN, given, readTag, stripped } from "../../core/tags";
  import type { Named } from "../../core/tags";
  import { ui } from "../../ui";
  import type { Tag } from "../../wire";
  import Glyph from "../parts/glyph.svelte";

  interface Props {
    // The session as its tags name it; null where the city has no name
    // to keep tags under, which leaves the button disabled.
    readonly named: Named | null;
    // The row's own name, which the button's accessible name carries.
    readonly label: string;
    readonly tags: readonly Tag[];
    readonly pinning: Pinning;
  }

  const { named, label, tags, pinning }: Props = $props();

  const u = ui();
  const { lang } = u;
  const held = u.tags.held;
  const seat = $props.id();

  interface Item {
    readonly id: string;
    readonly word: string;
    readonly act: () => void;
  }

  const items = $derived.by((): Item[] => [
    ...(pinning === "mayor"
      ? []
      : [{ id: "pin", word: say($lang, pinning === "tagged" ? "world_unpin" : "world_pin"), act: () => { change(pinning === "tagged" ? stripped : given, PIN); } }]),
    ...tags.filter((tag) => tag !== PIN).map((tag) => ({
      id: `strip-${tag}`,
      word: fill(say($lang, "world_tag_remove"), { tag }),
      act: () => { change(stripped, tag); },
    })),
    { id: "add", word: say($lang, "world_tag_add"), act: () => { naming = true; queueMicrotask(() => field?.focus()); } },
  ]);

  let open = $state(false);
  let naming = $state(false);
  let typed = $state("");
  let trigger = $state<HTMLButtonElement | undefined>(undefined);
  let field = $state<HTMLInputElement | undefined>(undefined);
  const entries: (HTMLButtonElement | undefined)[] = [];
  const wrong = $derived(typed.trim() !== "" && readTag(typed) === null);

  function show(): void {
    open = true;
    naming = false;
    typed = "";
    queueMicrotask(() => entries[0]?.focus());
  }

  function close(): void {
    open = false;
    naming = false;
    void tick().then(() => trigger?.focus());
  }

  function change(how: typeof given, tag: Tag): void {
    if (named !== null) u.tags.retag(how($held, named, tag));
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
          naming = false;
        }
      }}
    >
      {#if naming}
        <form class="flex flex-col gap-tight p-tight" onsubmit={submit}>
          <input
            bind:this={field}
            bind:value={typed}
            class="h-control-sm rounded-control bg-page px-snug text-note text-text outline-none focus-visible:ring-1 focus-visible:ring-accent"
            aria-label={say($lang, "world_tag_name")}
            aria-invalid={wrong}
            aria-describedby="{seat}-hint"
            maxlength={24}
            onkeydown={(event) => {
              if (event.key === "Escape") {
                event.preventDefault();
                close();
              }
            }}
          />
          <p id="{seat}-hint" class={["text-note", wrong ? "text-alert" : "text-text-faint"]}>{say($lang, "world_tag_hint")}</p>
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
