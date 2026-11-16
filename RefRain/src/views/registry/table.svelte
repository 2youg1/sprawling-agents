<script lang="ts">
  import { toFragment } from "../../core/route";
  import { clock } from "../../core/time";
  import { say } from "../../core/lang";
  import { ui } from "../../ui";
  import AskMayor from "../city/ask_mayor.svelte";
  import type { RegistryLine } from "../../wire";
  import EmptyState from "../parts/empty.svelte";
  import { Table } from "../parts/table";

  interface Props {
    readonly assets: readonly RegistryLine[];
  }

  const { assets }: Props = $props();

  const { lang } = ui();

  function keyOf(line: RegistryLine): string {
    return `${String(line.at)}\u0000${line.addr}\u0000${line.kind}\u0000${line.subject}`;
  }

  const lines = $derived([...assets].sort((a, b) => b.at - a.at));
</script>

{#snippet atCell(line: RegistryLine)}
  <span class="whitespace-nowrap text-note text-text-faint">{clock($lang, line.at)}</span>
{/snippet}

{#snippet kindCell(line: RegistryLine)}
  <span class="whitespace-nowrap font-mono text-note text-text-quiet">{line.kind}</span>
{/snippet}

{#snippet addrCell(line: RegistryLine)}
  <a
    href={toFragment({ kind: "building", address: line.addr })}
    class="block truncate font-mono text-note text-text-quiet underline decoration-edge underline-offset-2 transition-colors ease-leave hover:decoration-accent hover:ease-arrive"
  >
    {line.addr}
  </a>
{/snippet}

{#snippet subjectCell(line: RegistryLine)}
  <span class="line-clamp-2 min-w-0 text-note text-text">{line.subject}</span>
{/snippet}

<Table
  caption={say($lang, "nav_registry")}
  rows={lines}
  {keyOf}
  columns={[
    {
      key: "at",
      header: say($lang, "registry_when"),
      summary: true,
      min: "12ch",
      render: atCell,
      compare: (a: RegistryLine, b: RegistryLine) => a.at - b.at,
    },
    {
      key: "kind",
      header: say($lang, "registry_kind"),
      summary: true,
      min: "12ch",
      render: kindCell,
      compare: (a: RegistryLine, b: RegistryLine) => a.kind.localeCompare(b.kind),
    },
    {
      key: "addr",
      header: say($lang, "registry_where"),
      min: "24ch",
      render: addrCell,
      compare: (a: RegistryLine, b: RegistryLine) => a.addr.localeCompare(b.addr),
    },
    {
      key: "subject",
      header: say($lang, "registry_what"),
      min: "12ch",
      render: subjectCell,
      compare: (a: RegistryLine, b: RegistryLine) => a.subject.localeCompare(b.subject),
    },
  ]}
>
  {#snippet empty()}
    <EmptyState missing="registry_empty" seat="inset">
      {#snippet action()}
        <AskMayor />
      {/snippet}
    </EmptyState>
  {/snippet}
</Table>
