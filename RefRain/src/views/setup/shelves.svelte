<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
</script>

<script lang="ts">
  import { configureSkillShelves } from "../../core/commands";
  import { fill, say } from "../../core/lang";
  import type { Key, Lang } from "../../core/lang";
  import { noticeTitle } from "../parts/notice_title";
  import { ui } from "../../ui";
  import type { Address, AxError, Query } from "../../wire";
  import Button from "../parts/button.svelte";
  import Field from "../parts/field.svelte";
  import { HALL } from "../shared/buildings";
  import { alreadyMounted, offered, withLibrary, withoutLibrary } from "./skill_libraries";

  interface Props {
    readonly building?: Address;
  }

  const { building = HALL }: Props = $props();

  const u = ui();
  const { lang } = u;
  const belief = u.conn.belief;
  const city = $derived($belief.city ?? "<city>");
  const folders = $derived<readonly { readonly name: Key; readonly path: string }[]>([
    { name: "setup_skills", path: `${city}/.sprawling/library/` },
    { name: "setup_skills_building", path: `${city}/${building}/.sprawling/skills/` },
  ]);

  const question = $derived<Query>({ skills: { building } });
  const answer = $derived(u.conn.asking.ask(question));
  const mounted = $derived.by((): readonly string[] | null => {
    const held = $answer;
    if (held === undefined || !("skills" in held)) return null;
    return held.skills.external_shelves ?? [];
  });
  const held = $derived<readonly string[]>(mounted ?? []);

  let wanted = $state("");
  const offer = $derived(offered(wanted));
  const taken = $derived(offer.kind === "path" && alreadyMounted(held, offer.path));
  const ready = $derived(offer.kind === "path" && !taken);
  const offerWhy = $derived.by((): Key | null => {
    if (offer.kind === "blank") return "skill_mount_needs_path";
    if (offer.kind === "refused") return offer.why;
    return taken ? "skill_mount_taken" : null;
  });

  let sent = $state<string | null>(null);
  let refused = $state.raw<AxError | null>(null);
  $effect(() => {
    if (mounted === null || sent === null) return;
    sent = null;
    refused = null;
    wanted = "";
  });
  let seen = $state.raw<AxError | null>($belief.refusal);
  $effect(() => {
    const error = $belief.refusal;
    if (error === null || error === seen || sent === null) return;
    seen = error;
    refused = error;
    u.conn.dismissRefusal();
  });

  function mount(): void {
    if (!ready || offer.kind !== "path") return;
    const path = offer.path;
    if (u.send(configureSkillShelves(withLibrary(held, path)))) {
      sent = path;
      refused = null;
    }
  }

  function unmount(path: string): void {
    if (held.length === 0) return;
    if (u.send(configureSkillShelves(withoutLibrary(held, path)))) {
      sent = path;
      refused = null;
    }
  }

  function refusedLine(error: AxError, where: string, lang: Lang): string {
    return fill(say(lang, "skill_mount_refused"), {
      path: where,
      why: noticeTitle(lang, error.code, error.subject),
    });
  }
</script>

<div class="flex flex-col gap-snug text-note text-text-quiet">
  {#each folders as folder (folder.name)}
    <p>{say($lang, folder.name)}</p>
    <div class="flex min-w-0 items-center gap-snug">
      <code class="min-w-0 truncate rounded-control bg-raised px-base py-snug font-mono text-text"
        >{folder.path}</code
      >
      <Button
        label={say($lang, "setup_copy")}
        tone="quiet"
        onPress={() => {
          void navigator.clipboard.writeText(folder.path);
        }}
      />
    </div>
  {/each}
  <p class="text-text-faint">{say($lang, "setup_skills_how")}</p>

  <div class="mt-base flex min-w-0 flex-col gap-snug border-t border-edge pt-base">
    <p class="font-label text-text">{say($lang, "skill_mount_title")}</p>
    {#if mounted === null}
      <p class="text-text-faint">{say($lang, "skill_mount_untold")}</p>
    {:else}
      {#if held.length === 0}
        <p class="text-text-faint">{say($lang, "skill_mount_none")}</p>
      {:else}
        <ul class="flex min-w-0 flex-col">
          {#each held as path (path)}
            <li class="flex min-w-0 items-center gap-snug py-tight">
              <code class="min-w-0 flex-1 truncate font-mono text-text">{path}</code>
              <Button
                label={say($lang, "skill_mount_remove")}
                name={fill(say($lang, "skill_mount_remove_name"), { path })}
                tone="quiet"
                loading={sent === path}
                onPress={() => {
                  unmount(path);
                }}
              />
            </li>
          {/each}
        </ul>
      {/if}
      <form
        class="flex flex-wrap items-end gap-snug"
        onsubmit={(event) => {
          event.preventDefault();
          mount();
        }}
      >
        <div class="min-w-[min(100%,40ch)] flex-1">
          <Field
            label={say($lang, "skill_mount_label")}
            value={wanted}
            mono
            placeholder={say($lang, "skill_mount_placeholder")}
            {...offer.kind === "refused" ? { error: say($lang, offer.why) } : {}}
            onInput={(value) => {
              wanted = value;
            }}
          />
        </div>
        <Button
          type="submit"
          tone={ready ? "primary" : "secondary"}
          label={say($lang, "skill_mount_add")}
          {...offerWhy === null ? {} : { why: say($lang, offerWhy) }}
        />
      </form>
      <p class="text-text-faint">{say($lang, "skill_mount_help")}</p>
      {#if sent !== null && refused === null}
        <p class="text-text-faint" role="status">{say($lang, "skill_mount_waiting")}</p>
      {/if}
      {#if refused !== null}
        <p class="text-note text-alert" role="alert">{refusedLine(refused, sent ?? "", $lang)}</p>
      {/if}
    {/if}
  </div>
</div>
