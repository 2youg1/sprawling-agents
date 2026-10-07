<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // The city's own layer, written from here (refrain roadmap UG): the
  // standing thinking level `[model] effort` and whether the city renews
  // a warm prompt cache before it expires, both in the city's
  // `CONFIG.toml`, through `ConfigureCity`, one fact per pick so a pick
  // never rewrites the other. Each pick takes effect for the runs that
  // start after it, so the card has no button (`card.svelte` without
  // `onSave`); its foot says saved only once the city's file reads back
  // changed.
  //
  // The level shown is what `Query::Config` answers for the hall when
  // the city's file settled it. Whether the cache is kept warm is not on
  // any answer, so that control starts with nothing chosen and the file
  // itself, folded at the foot of this group, is where the value reads.

  import { configureCity, EFFORTS } from "../../core/commands";
  import { readDocument } from "../../core/document";
  import { say } from "../../core/lang";
  import { ui } from "../../ui";
  import type { Effort, KeepWarm } from "../../wire";
  import Segmented from "../parts/segmented.svelte";
  import { HALL } from "../shared/buildings";
  import Card from "./card.svelte";
  import { CITY_CONFIG } from "./files";
  import { HELD, answered, awaitReceipt, refused, sent } from "./saving";
  import type { Saving, Slot } from "./saving";

  const KEEP_WARM: readonly KeepWarm[] = ["off", "five_minute"];

  const u = ui();
  const { lang } = u;
  const belief = u.conn.belief;
  const config = u.conn.asking.ask({ config: { addr: HALL } });
  const file = u.conn.asking.ask({ document: { at: CITY_CONFIG } });

  const effort = $derived.by((): Effort | null => {
    const answer = $config;
    const settled = answer !== undefined && "config" in answer ? answer.config.effort : null;
    return settled?.from === "city" ? settled.effort : null;
  });
  // The file's text is the version a pick waits to see move.
  const text = $derived.by(() => {
    const read = readDocument($file);
    return read.kind === "held" ? read.value.text : read.kind === "unavailable" ? "" : null;
  });

  let warm = $state.raw<KeepWarm | null>(null);
  let saving = $state.raw<Saving>(HELD);
  const slot: Slot = { now: () => saving, mark: (next) => (saving = next) };

  $effect(() => {
    if (text !== null) saving = answered(saving, text);
  });
  let seen = $belief.refusal;
  $effect(() => {
    const error = $belief.refusal;
    if (error === null || error === seen) return;
    seen = error;
    saving = refused(saving, error);
  });

  function write(keepWarm: KeepWarm | null, level: Effort | null): void {
    if (text === null || !u.send(configureCity(keepWarm, level))) return;
    const mine = sent(text);
    saving = mine;
    awaitReceipt(mine, slot);
  }

  const WARM_WORD: Record<KeepWarm, "city_keep_warm_off" | "city_keep_warm_five_minute"> = {
    off: "city_keep_warm_off",
    five_minute: "city_keep_warm_five_minute",
  };
</script>

<Card title="city_layer" note="city_layer_note" {saving} settled="settings_next_run">
  <div class="flex flex-col gap-tight">
    <span class="text-note text-text-quiet">{say($lang, "city_effort")}</span>
    <Segmented
      label={say($lang, "city_effort")}
      tone="accent"
      options={EFFORTS.map((each) => ({ value: each, label: say($lang, `effort_${each}`) }))}
      held={effort}
      onPick={(next: Effort) => {
        write(null, next);
      }}
    />
  </div>
  <div class="flex flex-col gap-tight">
    <span class="text-note text-text-quiet">{say($lang, "city_keep_warm")}</span>
    <Segmented
      label={say($lang, "city_keep_warm")}
      options={KEEP_WARM.map((each) => ({ value: each, label: say($lang, WARM_WORD[each]) }))}
      held={warm}
      onPick={(next: KeepWarm) => {
        warm = next;
        write(next, null);
      }}
    />
  </div>
</Card>
