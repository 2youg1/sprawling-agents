# Licenses and attribution for `skills/`

Each skill in this directory states its license in its frontmatter, here,
and in `docs/third-party.md` §5. The repository owner's own skills carry
MPL-2.0, the license of the rest of this tree; a skill adapted from someone
else's work keeps that work's license, and is here only because that license
permits redistribution. This file travels with
the directory — in the tree and in the release archive — so the obligations
reach every copy.

## `why`, `how`, `blast-radius` — MIT

Modified adaptations of pstack skills — `pstack/skills/why`,
`pstack/skills/how` and `pstack/skills/blast-radius` in
[cursor/plugins](https://github.com/cursor/plugins), the skill collection by
Lauren Tan (poteto). Copyright (c) 2026 Lauren Tan, MIT Licence.

**What changed, and who changed it.** Modifications by 2youg1, 2026: the
operating posture and the failure modes, the synthesis templates and the
confidence-tier examples, and the critique rubric condensed from pstack's
`principle-*` skills (`principle-boundary-discipline` among them) into the
lenses one sequential reviewer walks. Each file carries the statement beside
its source note.

### MIT License

Copyright (c) 2026 Lauren Tan

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

## `sdd`, `tutor`, `translation` — MPL-2.0

English translations and adaptations by 2youg1, 2026, of the author's own
Chinese-language open-source skills, published under AGPL-3.0-or-later. The
author licenses these three files under MPL-2.0, the license in `LICENSE` at
the root of this tree; the originals remain AGPL-3.0-or-later. The translation
skill keeps the byline its original wore — KL9 ＆ Claude Fable 5.

## `gauge` — MPL-2.0

The repository owner's own skill, written for this tree under MPL-2.0, the
license in `LICENSE` at the root of this tree.

## `playback` — MPL-2.0

Written for this repository by 2youg1 and the sprawling contributors, 2026,
under MPL-2.0, the license in `LICENSE` at the root of this tree. It states
the playback bundle's data contract and the checks `sprawling playback check`
runs, so it changes with the code that writes them.

Its reference page, `playback/template.html`, embeds a subset of Geist Mono
(Copyright (c) 2023 Vercel, in collaboration with basement.studio) under the
SIL Open Font License 1.1, whose text is `playback/OFL.txt`; the rest of the
page is MPL-2.0 with the skill.

## `authority-review` — MIT

Modified adaptation of the Thermos plugin (`cursor/plugins`, MIT), the same upstream as
`branch-audit` was drawn from: `thermo-nuclear-review` and
`thermo-nuclear-code-quality-review`. **What changed, and who changed it.** Modifications by
2youg1, 2026: the two parallel rubrics become two sequential passes in one context, with a rule
that pass one's findings are written down before pass two's rubric is read, and pass two is
recut as one lens over the whole diff - a fact with more than one authoritative definition,
ranked by how close the copies are to disagreeing, with the six shapes that carry most findings
and a requirement to name the definition that survives. Copyright (c) 2026 the Thermos plugin's
authors, MIT Licence.
