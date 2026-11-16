<script lang="ts" module>
  // wording-ok: the project's own release page; a proper noun identical

  const RELEASES = "https://github.com/2youg1/sprawling-agents/releases";
</script>

<script lang="ts">
  import { readable } from "svelte/store";
  import type { Readable } from "svelte/store";

  import { QUERIES } from "../core/asking";
  import { say } from "../core/lang";
  import { ui } from "../ui";
  import type { Answer, ReleaseAnswer } from "../wire";
  import Button from "./parts/button.svelte";
  import ReleaseAnswerView from "./release/answer.svelte";

  const u = ui();
  const lang = u.lang;

  let slot: Readable<Answer | undefined> = $state(readable<Answer | undefined>(undefined));
  let asking = $state(false);

  const answer = $derived.by((): ReleaseAnswer | undefined => {
    const now = $slot;
    return now !== undefined && "release" in now ? now.release : undefined;
  });

  $effect(() => {
    if (answer !== undefined) {
      asking = false;
    }
  });

  function check(): void {
    asking = true;

    slot = u.conn.asking.ask(QUERIES.release);
    u.conn.asking.refresh(QUERIES.release);
  }
</script>

<section class="flex min-w-0 flex-col gap-snug">
  <div class="flex flex-wrap items-center gap-snug">
    <Button
      label={say($lang, "release_check_registries")}
      tone="secondary"
      loading={asking}
      onPress={check}
    />
    <a class="text-note text-accent underline" href={RELEASES} target="_blank" rel="noreferrer">
      {RELEASES}
    </a>
  </div>
  {#if answer !== undefined}
    <ReleaseAnswerView {answer} />
  {/if}
</section>
