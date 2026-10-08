<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts" module>
  // The ways into a page that are not the page: the palette (Accel-/),
  // the file finder in its box (Accel-P), the back key every page but
  // the conversation stands under, and the stack of refusals over the
  // composer. The palette and the finder are drawn as specimens - the
  // same box open in this fold's flow, because a modal would cover the
  // whole route - and the refusals in the stack's own specimen stand.
  //
  // The palette shows the places with the cursor on its first row, then
  // the verbs with one this page cannot run, kept and greyed with its
  // reason, the cursor standing on it so the reason is the line read.

  import { QUERIES } from "../../core/asking";
  import { Address } from "../../wire";
  import type { Answer, FindAnswer, Query } from "../../wire";
  import { ENDPOINTS } from "./served";

  const LAB = Address.make("lab");

  const FOUND: readonly Address[] = ["Roadmap.md", "notes/roadmap-review.md", "docs/archive/old-roadmap.txt"].map(
    (path) => Address.make(path),
  );

  function answering(query: Query): Answer | undefined {
    if (query === QUERIES.endpoints) return { endpoints: ENDPOINTS };
    if (typeof query === "object" && "find" in query) {
      const find: FindAnswer = { under: LAB, text: query.find.text, paths: [...FOUND], walked: "whole" };
      return { find };
    }
    return undefined;
  }
</script>

<script lang="ts">
  import { say } from "../../core/lang";
  import { ui } from "../../ui";
  import Search from "../finder/search.svelte";
  import { modalHold, modalWire } from "../modal";
  import Modal from "../modal.look.svelte";
  import { backLookOf } from "../pages/back";
  import Back from "../pages/back.look.svelte";
  import Palette from "../palette.svelte";
  import { boxOf, whyFor } from "../palette";
  import PaletteLook from "../palette.look.svelte";
  import type { Entry } from "../palette/entry";
  import Rows from "../palette/rows.svelte";
  import Button from "../parts/button.svelte";
  import Notice from "../parts/notice.svelte";
  import RefusalLook from "../refusal.look.svelte";
  import Case from "./case.svelte";
  import Stand from "./stand.svelte";

  const { lang } = ui();

  const ignore = (): void => undefined;
  const specimen = (label: string) =>
    modalWire({ seat: "specimen", named: { "aria-label": label }, backdrop: "stays", hold: modalHold("specimen"), onClose: ignore });

  // A verb list as the palette groups it, on a page with no run in
  // front of the person: `/stop` is kept, greyed, with its reason.
  const reach = { here: null, live: false, models: 1 };
  const verb = (spelling: string, about: string): Entry => ({
    label: spelling,
    hint: about,
    why: whyFor(spelling, reach),
    act: ignore,
  });
  const ACTIONS = $derived([
    verb("/halt", say($lang, "slash_halt")),
    verb("/stop", say($lang, "slash_stop")),
  ]);
  const SESSIONS = $derived([verb("/model", say($lang, "slash_model"))]);
  const VERBS = $derived([...ACTIONS, ...SESSIONS]);
  const BACK = $derived(backLookOf({ kind: "mcp" }, $lang));
</script>

<Case label="palette · places, the cursor on the first row" width={640}>
  <Stand link={{ kind: "live", city: "sprawling" }} unread={[]} waiting={[]} answers={answering}>
    <Palette seat="specimen" onClose={ignore} />
  </Stand>
</Case>

<Case label="palette · verbs, one this page cannot run greyed with its reason" width={640}>
  <Modal wire={specimen(say($lang, "nav_palette"))} seat="specimen">
    <PaletteLook
      box={boxOf(
        { query: "/", cursor: 1, count: VERBS.length, list: "entrances-verbs", lang: $lang },
        { write: ignore, point: ignore, pick: ignore },
      )}
      nothing={undefined}
    >
      {#snippet list()}
        <Rows
          id="entrances-verbs"
          listing={{
            kind: "verbs",
            groups: [
              { section: "actions", entries: ACTIONS },
              { section: "sessions", entries: SESSIONS },
            ],
          }}
          shown={VERBS}
          cursor={1}
          onHover={ignore}
          onPick={ignore}
        />
      {/snippet}
    </PaletteLook>
  </Modal>
</Case>

<Case label="file finder · in its box, the cursor on the first path" width={640}>
  <Stand link={{ kind: "live", city: "sprawling" }} unread={[]} waiting={[]} answers={answering}>
    <Modal wire={specimen(say($lang, "finder_placeholder"))} seat="specimen">
      <Search under={LAB} titleId="entrances-finder" onClose={ignore} />
    </Modal>
  </Stand>
</Case>

<Case label="back key · to the settings, over a page">
  {#if BACK !== undefined}
    <Back {...BACK} />
  {/if}
</Case>

{#snippet toast(key: number)}
  {#if key === 0}
    <Notice
      seat="toast"
      weight="alert"
      action="cannot send to hall/mayor"
      code="E_PROVIDER"
      subject="the provider did not answer"
      recovery="check the provider on the settings page, then send again"
    >
      {#snippet actions()}
        <Button tone="quiet" label={say($lang, "dismiss")} onPress={ignore} />
      {/snippet}
    </Notice>
  {:else}
    <Notice seat="toast" weight="info" heading="no_run_in_front" next="stop_whole_city">
      {#snippet actions()}
        <Button tone="quiet" label={say($lang, "dismiss")} onPress={ignore} />
      {/snippet}
    </Notice>
  {/if}
{/snippet}

<Case label="refusal · two toasts, a refusal and a stop with no run in front" width={480}>
  <RefusalLook
    stand={{ kind: "specimen" }}
    toasts={[0, 1].map((key) => ({
      key,
      gone: false,
      wire: { onmouseenter: ignore, onmouseleave: ignore, onfocusin: ignore, onfocusout: ignore },
    }))}
    body={toast}
  />
</Case>
