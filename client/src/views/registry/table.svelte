<script lang="ts">
  // This Source Code Form is subject to the terms of the Mozilla Public
  // License, v. 2.0. If a copy of the MPL was not distributed with this
  // file, You can obtain one at https://mozilla.org/MPL/2.0/.
  // Copyright (c) 2026 2youg1 and the sprawling contributors

  // The registry as a sortable table. The four columns are the four
  // facts the wire carries about a filed asset, and every column that can
  // be ordered by reorders from the newest first below - which is
  // `parts/table`'s own behaviour and not a second sort written here
  // (client-SPEC 4-24).

  import { MAYOR, toFragment } from "../../core/route";
  import { clock } from "../../core/time";
  import { say } from "../../core/lang";
  import { ui } from "../../ui";
  import type { RegistryLine } from "../../wire";
  import EmptyState from "../parts/empty.svelte";
  import { Table } from "../parts/table";

  interface Props {
    readonly assets: readonly RegistryLine[];
  }

  const { assets }: Props = $props();

  const { lang } = ui();

  // The identity of one line. The wire gives an asset no id of its own,
  // and these four fields are what the city wrote about it, so the four
  // together are as much identity as the record carries.
  function keyOf(line: RegistryLine): string {
    return `${String(line.at)}\u0000${line.addr}\u0000${line.kind}\u0000${line.subject}`;
  }

  // Newest first, because the question a person opens a registry with
  // is what was filed last.
  const lines = $derived([...assets].sort((a, b) => b.at - a.at));
</script>

{#snippet atCell(line: RegistryLine)}
  <span class="whitespace-nowrap text-note text-text-faint">{clock($lang, line.at)}</span>
{/snippet}

{#snippet kindCell(line: RegistryLine)}
  <!-- The city's own word for what was filed, which no language
  translates because it is the event's own spelling. -->
  <span class="whitespace-nowrap font-mono text-note text-text-quiet">{line.kind}</span>
{/snippet}

{#snippet addrCell(line: RegistryLine)}
  <!-- An address is an identifier: one line, cut off with an ellipsis
  when the column is narrower than the address, never broken. -->
  <a
    href={toFragment({ kind: "building", address: line.addr })}
    class="block truncate font-mono text-note text-text-quiet underline decoration-edge underline-offset-2 hover:decoration-accent"
  >
    {line.addr}
  </a>
{/snippet}

{#snippet subjectCell(line: RegistryLine)}
  <!-- What the asset is about is the one prose column here, so it is
  the one that takes a second line: two lines of a sentence say what it
  is about, and a third only lengthens the row. -->
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
    <!-- An empty registry is a city that has not been asked for
    anything yet: an asset is filed by a run, and a run begins in the
    conversation with the Mayor. -->
    <EmptyState missing="registry_empty" seat="inset">
      {#snippet action()}
        <a
          href={toFragment({ kind: "talk", address: MAYOR })}
          class="inline-flex h-control items-center rounded-control bg-accent px-base text-label text-on-accent hover:bg-accent-hover"
        >
          {say($lang, "city_ask_mayor")}
        </a>
      {/snippet}
    </EmptyState>
  {/snippet}
</Table>
