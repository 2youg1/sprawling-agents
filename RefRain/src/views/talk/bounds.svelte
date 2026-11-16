

<script lang="ts">
  
  
  import { readAnswer } from "../../core/answered";
  import { QUERIES } from "../../core/asking";
  import { say } from "../../core/lang";
  import { ui } from "../../ui";
  import type { Address } from "../../wire";
  import Fact from "./fact.look.svelte";
  import Sandbox from "./sandbox.svelte";

  interface Props {
    readonly room: Address | null;
  }

  const { room }: Props = $props();

  const u = ui();
  const { lang } = u;
  const governance = u.conn.asking.ask(QUERIES.governance);

  interface Bound {
    readonly words: string;
    readonly worrying: boolean;
  }

  const gate = $derived.by((): Bound => {
    const read = readAnswer($governance, (held) =>
      "governance" in held ? held.governance : undefined,
    );
    switch (read.kind) {
      case "asking":
        return { words: "—", worrying: false };
      case "unavailable":
        return { words: say($lang, "facts_unreadable"), worrying: true };
      case "held":
        return {
          words:
            read.value.autonomy === "owner"
              ? say($lang, "autonomy_owner")
              : say($lang, "autonomy_clerk"),
          worrying: false,
        };
    }
  });
</script>

<Fact glyph="gate" heard={say($lang, "facts_autonomy")} asks={gate.worrying}>{gate.words}</Fact>
<Sandbox {room} />
