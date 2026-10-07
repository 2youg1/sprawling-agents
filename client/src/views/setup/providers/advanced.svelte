<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // The seat of everything a person may state about an endpoint and
  // almost nobody has to: what this provider is called, and the settings
  // a working endpoint never needs changed. Folded away until somebody
  // asks for it, because the form's three boxes are what nearly
  // everybody came to fill in.
  //
  // The seat asks the city for its untuned figures and draws whatever
  // `./advanced.look.svelte` is; what each box means and what a press
  // writes into the form's draft are `./advanced`.

  import type { Draft } from "./draft";
  import type { SetDraft } from "./advanced";

  export interface AdvancedProps {
    readonly draft: Draft;
    readonly setDraft: SetDraft;
    // A rename retires the last report: what it said was about the
    // name the form carried then.
    readonly onRenamed: () => void;
  }
</script>

<script lang="ts">
  import { HALL } from "../../shared/buildings";
  import { ui } from "../../../ui";
  import { lookOf } from "./advanced";
  import Look from "./advanced.look.svelte";

  const { draft, setDraft, onRenamed }: AdvancedProps = $props();

  const u = ui();
  const { lang } = u;
  // The city's figures for an untuned endpoint are the same at every
  // address, and the hall is the one address every city has.
  const config = u.conn.asking.ask({ config: { addr: HALL } });
  const defaults = $derived.by(() => {
    const answer = $config;
    return answer !== undefined && "config" in answer ? answer.config.tuning : undefined;
  });

  const look = $derived(lookOf(draft, defaults, $lang, { setDraft, onRenamed }));
</script>

<Look {...look} />
