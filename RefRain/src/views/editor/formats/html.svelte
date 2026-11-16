<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  
  
  import { say } from "../../../core/lang";
  import { ui } from "../../../ui";
  import Frame from "./frame.svelte";
  import Label from "./label.svelte";

  interface Props {
    readonly text: string;
    readonly drafted: boolean;
    readonly name: string;
  }

  const { text, drafted, name }: Props = $props();

  const lang = ui().lang;
</script>

<div class="flex min-h-0 flex-1 flex-col">
  <Label
    format="html"
    tool={say($lang, "format_tool_browser")}
    settings={[say($lang, "format_html_settings")]}
    version={null}
    notes={drafted ? [say($lang, "format_from_draft")] : []}
  />
  <Frame page={text} scheme="authored" title={name} />
</div>
