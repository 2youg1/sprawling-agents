<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // The remote group drawn from one standing (client-SPEC 4-57): what
  // the door is and the console verbs that open it, an invitation to
  // pair, the seed shown once, and the device this browser holds. Every
  // press goes back to the caller; the one status line is always
  // mounted, so what a press came back with is announced when it lands.
  // Drawn by `remote.svelte` in the settings panel and, one standing at
  // a time, by the gallery.

  import { tick } from "svelte";

  import { fill, say } from "../../core/lang";
  import { encode } from "../../core/remote/base32";
  import { isoDay, isoTime } from "../../core/time";
  import { ui } from "../../ui";
  import Button from "../parts/button.svelte";
  import Dialog from "../parts/dialog.svelte";
  import { LACKS, VERBS, grouped, toldKey, type Standing } from "./remote";

  interface Props {
    readonly standing: Standing;
    readonly onPair: () => void;
    readonly onSeen: () => void;
    readonly onLock: () => void;
    readonly onForget: () => void;
  }

  const { standing, onPair, onSeen, onLock, onForget }: Props = $props();
  const { lang } = ui();
  const uid = $props.id();

  let forgetting = $state(false);
  let deviceHeading = $state<HTMLElement | undefined>(undefined);
  let forgetButton = $state<HTMLElement | undefined>(undefined);

  // The seed leaves the page by a press, and the focus lands on the
  // device it belongs to rather than on the body.
  let was: Standing["kind"] = "reading";
  $effect(() => {
    const now = standing.kind;
    if (was === "seed" && now === "paired") void tick().then(() => deviceHeading?.focus());
    was = now;
  });

  const told = $derived(
    (standing.kind === "invited" || standing.kind === "paired") && standing.told !== null
      ? toldKey(standing.told)
      : null,
  );

  const LABEL = "text-note text-text-faint";
  const VALUE = "font-mono text-note text-text break-all";
</script>

<div class="flex flex-col gap-wide">
  {#if standing.kind === "unpaired" || standing.kind === "unreadable" || standing.kind === "lacking"}
    <p class="text-body text-text-quiet">{say($lang, "remote_about")}</p>
    {#if standing.kind === "unreadable"}
      <p class="text-body text-alert">{say($lang, "remote_unreadable")}</p>
    {:else if standing.kind === "lacking"}
      <p class="text-body text-alert">{say($lang, LACKS[standing.lack])}</p>
    {/if}
    <section class="flex flex-col" aria-labelledby={`${uid}-console`}>
      <h3 id={`${uid}-console`} class="pb-tight text-label font-label text-text">{say($lang, "remote_console")}</h3>
      <dl class="flex flex-col">
        {#each VERBS as [spelling, does] (spelling)}
          <div class="grid grid-cols-[minmax(0,16rem)_minmax(0,1fr)] gap-x-base border-b border-edge py-snug @max-measure/page:grid-cols-1">
            <dt><code class="font-mono text-note text-text">{spelling}</code></dt>
            <dd class="text-note text-text-quiet">{say($lang, does)}</dd>
          </div>
        {/each}
      </dl>
    </section>
    <p class="text-note text-text-faint">{say($lang, "remote_route")}</p>
    {#if standing.kind === "unpaired"}
      <p class="text-note text-text-faint">{say($lang, "remote_not_paired")}</p>
    {/if}
  {:else if standing.kind === "invited"}
    <section class="flex flex-col gap-base" aria-labelledby={`${uid}-invited`}>
      <h3 id={`${uid}-invited`} class="text-label font-label text-text">{say($lang, "remote_invited")}</h3>
      <p class={VALUE}>{fill(say($lang, "remote_invited_city"), { fingerprint: grouped(encode(standing.invitation.city).slice(0, 12)) })}</p>
      <p class="text-body text-text-quiet">{say($lang, "remote_invited_about")}</p>
      <div>
        <Button label={say($lang, standing.busy ? "remote_pairing" : "remote_pair")} tone="primary" loading={standing.busy} onPress={onPair} />
      </div>
    </section>
  {:else if standing.kind === "seed"}
    <section class="flex flex-col gap-base" aria-labelledby={`${uid}-seed`}>
      <h3 id={`${uid}-seed`} class="text-label font-label text-text">{say($lang, "remote_seed")}</h3>
      <output class="asks rounded-card px-base py-snug font-mono text-heading text-text" aria-label={say($lang, "remote_seed")}>{grouped(standing.seed)}</output>
      <p class="text-body text-text-quiet">{say($lang, "remote_seed_about")}</p>
      {#if !standing.persisted}
        <p class="text-note text-text-faint">{say($lang, "remote_persist_refused")}</p>
      {/if}
      <div>
        <Button label={say($lang, "remote_seed_seen")} tone="primary" onPress={onSeen} />
      </div>
    </section>
  {:else if standing.kind === "paired"}
    <section class="flex flex-col gap-base" aria-labelledby={`${uid}-device`}>
      <h3 id={`${uid}-device`} bind:this={deviceHeading} tabindex="-1" class="text-label font-label text-text">{say($lang, "remote_device")}</h3>
      <dl class="grid grid-cols-[minmax(0,8rem)_minmax(0,1fr)] gap-x-base gap-y-snug">
        <dt class={LABEL}>{say($lang, "remote_device_city")}</dt>
        <dd class={VALUE}>{grouped(encode(standing.device.fingerprint))}</dd>
        <dt class={LABEL}>{say($lang, "remote_device_id")}</dt>
        <dd class={VALUE}>{encode(standing.device.id)}</dd>
        <dt class={LABEL}>{say($lang, "remote_device_at")}</dt>
        <dd class={VALUE}>{`${isoDay(standing.device.at)} ${isoTime(standing.device.at)}`}</dd>
      </dl>
      <div class="flex flex-wrap gap-snug">
        <Button label={say($lang, standing.busy ? "remote_locking" : "remote_lock")} loading={standing.busy} onPress={onLock} />
        <span bind:this={forgetButton} class="contents">
          <Button label={say($lang, "remote_forget")} tone="destructive" onPress={() => (forgetting = true)} />
        </span>
      </div>
    </section>
  {/if}
  <p role="status" class="text-body text-text-quiet empty:hidden">{told === null ? "" : fill(say($lang, told.key), { code: told.code ?? "" })}</p>
</div>

<Dialog
  open={forgetting}
  title={say($lang, "remote_forget_title")}
  detail={say($lang, "remote_forget_detail")}
  confirmLabel={say($lang, "remote_forget_confirm")}
  cancelLabel={say($lang, "part_cancel")}
  destructive
  onConfirm={() => {
    forgetting = false;
    onForget();
  }}
  onCancel={() => {
    forgetting = false;
    forgetButton?.querySelector("button")?.focus();
  }}
/>
