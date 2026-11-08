<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // The blank side columns of the shell on a full page: a quiet way back
  // to the conversation. They sit under the edge keys' layer, so the
  // three foot keys stay the click that reaches them, and they hide on
  // one column where there are no sides to press.

  import { say } from "../../core/lang";
  import { ui } from "../../ui";

  interface Props {
    readonly onHome: () => void;
  }

  const { onHome }: Props = $props();
  const { lang } = ui();
  const label = $derived(say($lang, "page_home"));
</script>

<button
  type="button"
  class="home-side col-start-1 row-start-2 narrow:hidden"
  tabindex="-1"
  aria-label={label}
  onclick={onHome}
></button>
<button
  type="button"
  class="home-side col-start-12 row-start-2 narrow:hidden"
  tabindex="-1"
  aria-label={label}
  onclick={onHome}
></button>

<style>
  .home-side {
    align-self: stretch;
    z-index: 0;
    min-width: 0;
    min-height: 0;
    padding: 0;
    border: 0;
    border-radius: var(--radius-control);
    background: transparent;
    cursor: pointer;
    transition: background-color var(--transition-duration-short) var(--ease-leave);
  }
  .home-side:hover,
  .home-side:focus-visible {
    background-color: color-mix(in oklch, var(--color-raised) 55%, transparent);
    transition-timing-function: var(--ease-arrive);
  }
  .home-side:focus-visible {
    outline: 2px solid var(--color-accent);
    outline-offset: -2px;
  }
</style>
