<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // Where the context reminder's second rung sits: a whole percent of
  // the window. The span and its reason are
  // `kernel::config::SecondThreshold`'s one construction point - thirty
  // through ninety, so the line about a handoff can still be said
  // before the window closes - and this page states the span rather
  // than judging it: the city's refusal comes back carrying the span,
  // and a page that enforced it here would be the second place the
  // rule lives.
  //
  // A box left empty is the city's own default, written as nothing at
  // all. What is in force is what `Query::Config` answers - the file's
  // word, read fresh - not this page's memory of a save.
</script>

<script lang="ts">
  import { configureContext } from "../core/commands";
  import { fill, say } from "../core/lang";
  import { ui } from "../ui";
  import type { Address } from "../wire";
  import Button from "./parts/button.svelte";
  import Field from "./parts/field.svelte";

  interface Props {
    readonly addr: Address;
  }

  const { addr }: Props = $props();
  const u = ui();
  const lang = u.lang;

  const SECOND_MIN = 30;
  const SECOND_MAX = 90;
  const SECOND_DEFAULT = 65;

  let box = $state("");
  let edited = $state(false);

  const config = $derived(u.conn.asking.ask({ config: { addr } }));
  // What the file says, or an empty box for a building that states
  // nothing - which is the default in force rather than a gap.
  const onDisk = $derived.by((): string => {
    const answer = $config;
    if (answer === undefined || !("config" in answer)) return "";
    const second = answer.config.second;
    return second === null || second === undefined ? "" : String(second.percent);
  });

  let was: Address | undefined = undefined;
  $effect(() => {
    const at = addr;
    const text = onDisk;
    if (!edited || was !== at) {
      edited = false;
      box = text;
    }
    was = at;
  });

  const span = (): string =>
    fill(say($lang, "context_second_range"), {
      min: String(SECOND_MIN),
      max: String(SECOND_MAX),
      default: String(SECOND_DEFAULT),
    });

  function save(): void {
    const percent = Number(box);
    if (!Number.isInteger(percent)) return;
    if (u.send(configureContext(addr, percent))) {
      edited = false;
    }
  }
</script>

<div class="flex flex-col gap-tight rounded-card bg-raised px-base py-snug">
  <span class="text-label font-label text-text">{say($lang, "context_second")}</span>
  <p class="text-note text-text-faint">{say($lang, "context_second_note")}</p>
  <Field
    label={say($lang, "context_second")}
    labelling="hidden"
    kind="number"
    step={1}
    suffix={say($lang, "context_second_unit")}
    mono
    value={box}
    onInput={(next: string) => {
      box = next;
      edited = true;
    }}
  />
  <div class="flex items-center gap-base">
    <Button
      label={say($lang, "context_second_save")}
      tone="primary"
      {...(edited ? {} : { why: say($lang, "context_second_unchanged") })}
      onPress={save}
    />
    <span class="text-note text-text-faint">{span()}</span>
  </div>
</div>
