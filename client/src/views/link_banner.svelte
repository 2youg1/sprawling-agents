<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // The page lost the city. Drawn with the banner the halt uses and in
  // the slot the halt uses, above every page, because both say the same
  // kind of thing - nothing on this page is moving, and here is why - and
  // a person should learn one place to look. The one control ends the
  // ladder's wait early.
  export interface LinkBannerProps {
    // Counted from one: the attempt the ladder is on.
    readonly attempt: number;
    // Words said since the link went down, sent once it is back.
    readonly unsent: number;
    readonly onRetry: () => void;
  }
</script>

<script lang="ts">
  import { fill, say } from "../core/lang";
  import { ui } from "../ui";
  import Banner from "./parts/banner.svelte";
  import Button from "./parts/button.svelte";

  const { attempt, unsent, onRetry }: LinkBannerProps = $props();
  const { lang } = ui();

  const detail = $derived(
    [
      fill(say($lang, "link_lost_detail"), { n: String(attempt) }),
      ...(unsent > 0 ? [fill(say($lang, "link_unsent"), { n: String(unsent) })] : []),
    ].join(" · "),
  );
</script>

<Banner text={say($lang, "link_lost")} {detail} weight="alert">
  {#snippet action()}
    <Button label={say($lang, "link_retry_now")} tone="secondary" onPress={onRetry} />
  {/snippet}
</Banner>
