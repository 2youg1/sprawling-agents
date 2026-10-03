# How a screen gets built here

This is the method for changing anything under `client/`: a new screen, a new state of an old one, or a control moved from one place to another. It is written for a person or an agent who has the repository open and a screen to change.

The view layer is exempt from the two rituals the Rust crates follow, writing the SPEC first and watching a test fail before the fix. What makes an interface good is cheap iteration, and a SPEC and a failing test for every visual change would tax that iteration to buy a kind of correctness the view layer was not losing. The exemption is not an exemption from judgement: four machine rules, one gate that measures the drawn page, and a walk a person makes through the running window take the place of the rituals.

## What to read first

- [`client/Spec.lean`](../client/Spec.lean) section 9 and the parts it names under `client/spec/`, the **interaction contract**: for each part, the WAI-ARIA pattern it implements, what each key does, the exact `aria-*` values, and where the focus goes when the part closes. This half of the client is not exempt from SPEC-first, because a keyboard user cannot see that a screen looks right.
- *The design, entry by entry*, near the end of this page: the grid, the tokens, the surfaces and the shell's parts as the person approved them, each under the label the code cites (§4-33, §7D and the rest).
- `client/src/views/parts/`, the controls the client already has — button, field, table, segmented control, combobox, popover, dialog, tooltip and the rest. A screen composes these parts and does not write a control of its own, because a second implementation of a control is where two behaviours begin to differ.
- `client/src/theme.css`, the only file allowed to name a colour, and the tokens for spacing, radius, type size and control height that every class in a view is built from.
- `client/src/lang.json`, the only place a word a reader sees is written, in English and in Chinese.
- The fixture on `#/gallery` for the screen you are changing, under `client/src/views/gallery/`.

## The loop

```bash
cd client && bun run dev     # the page, reloading as you edit
just build-web               # the bundle the binary embeds, in crates/sprawling/web-dist
cargo xtask render           # where the boxes of #/gallery landed in a real engine
just check-client            # lint, typecheck, the client's own tests
```

`cargo xtask render --width <px>` narrows a run to one of the widths the gate knows, which is the fast loop while one layout is being settled. A screen is finished when `cargo xtask render` is green against `#/gallery`, `just check-client` passes, and a person has walked it as the last section describes.

## Building a screen, in order

1. **Place it on the grid.** The shell is one CSS grid of twelve columns with 32 px margins, 24 px gutters and an 8 px baseline (§4-33 below). A region is placed by the column lines it starts and ends on, a panel of the world layer takes the shell's columns through `subgrid`, and no page container is centred with `mx-auto`, so two edges that should line up do so because they are the same line rather than because somebody measured them.
2. **Settle it against the shipped stylesheet.** Start from the tokens in `theme.css` and the parts that exist. When the design needs a size, a gap or a colour the tokens do not have, the change is to `theme.css`, where the colour gate can judge it, and not to a value written into one view. The interface is set in one face, Geist Mono, and hierarchy comes from size, weight and the grey ramp alone.
3. **Give each fact one home on the screen.** Before a reading is added, find where §7D below seats it in each tier. A fact that already has a home on that screen is not drawn a second time; it is moved, or left where it is.
4. **Choose parts by what the person has to do.** A small exclusive choice that a person reads at a glance is a segmented control. A list longer than a settings card can hold as one row of equal cells — the font families this machine has installed, the editors a file can open in — is a native `<select>` styled with the control tokens, because it works the same way under a keyboard, a screen reader and a touch screen at any width. A long list the person searches is the combobox. A choice that cannot be made says why through `Tip` before the click, instead of refusing afterwards.
5. **Put every word in `lang.json`.** Add the key with both languages, then call `say`. A proper noun that is the same word in both languages still goes into the table, with the same spelling twice.
6. **Give every state a fixture on `#/gallery`.** Empty, loading, full, refused, too long, in both lightings. The shell has its own: an empty room, each of the three tiers, the right pane open, and a phone width. A state with no fixture is a state nobody looks at twice, and the gate cannot measure what the gallery does not draw.
7. **State the width each fixture is drawn at.** `client/src/views/gallery/case.svelte` draws a component at the conversation's width unless the case says otherwise, a page at the window it is a page of, and a container under test at exactly its own width, inside a dashed frame that says the box is a specimen rather than the page.
8. **Run the gate, then walk the screen.** `cargo xtask render` measures; the walk below is for everything a measurement cannot see.

## The four machine rules

**Every word comes from `client/src/lang.json`.** A view calls `say` and does not write a sentence. `cargo xtask wording` reads the places where a reader is handed words — in a `.svelte` view, a text node and a spoken attribute such as `placeholder`, `title`, `alt` or an `aria-*` label; in a `.ts` module, the parts of a refusal — and refuses a literal in any of them. A mark that is not a word, such as a separator glyph hidden from screen readers, carries `wording-ok: <reason>` on its line or the line above.

**Every colour comes from `client/src/theme.css`.** `cargo xtask color` scans every other file for `oklch(`, `rgb(`, `#rrggbb` and their relatives, and holds the tokens themselves to a set of properties in both lightings: an eleven-rung grey ramp that climbs, no pure black or white, one hue axis and its single complement, exactly two chroma ratios, and text that reaches the APCA tier it claims on the brightest surface text may sit on.

**The bundle stays inside its budget.** `cargo xtask budget` weighs the built client against `tools/xtask/budgets.toml` and refuses a reading above the budget, or more than the slack above the best recorded reading.

**The client's own lint rules hold.** eslint refuses `any`, `as`, `throw`, `try` and a `switch` that is not exhaustive. A value read from outside the page — a stored preference, a frame from the socket, the value of a `<select>` — is narrowed by looking it up in the list of values the code knows (`EDITORS.find((each) => each === word)`), never cast.

## What `cargo xtask render` asserts

It opens the built bundle's `#/gallery` in a headless Chromium-family engine and measures what was drawn. It opens the page once at each width `tools/xtask/src/render/pass.rs` names — 768, 1280 and 2560 CSS pixels — because the columns are laid out by container queries and a property that holds at one width is a property about that width. Then it opens the page once in the light and once in a forced-colour mode, at the width the product is read at most, and each pass checks that the page drew the lighting it asked for.

Each assertion generalises a defect that shipped once:

1. Every control a person can operate has an accessible name.
2. Every landmark says which region it is.
3. A page has exactly one first heading.
4. The geometry `browser::survey` reads is within bounds: one left edge for the regions in the main column, one first mark down a navigation column, and nothing drawn outside the box that holds it.
5. Every popover shows an option, no text is crushed below a readable width, and no key hint is drawn underlined.

It asserts properties, never a picture. A screenshot comparison fails on a font hint and passes a page that is wrong in a way nobody photographed.

A missing bundle, a missing engine and a gallery that drew nothing are each a violation that names which one it was, because a gate that goes quiet when it cannot find its subject is green for the wrong reason. The first two name what supplies the missing part: `just build-web`, or a browser at `SPRAWLING_BROWSER`. The engine is looked for in the path in `SPRAWLING_BROWSER`, then among the components `sprawling doctor` installed, then at the usual install paths of Edge, Chrome and Chromium.

## The walk through the running window

`render` measures properties and cannot see the rest, so the last check is a person with the running window and this list, once per fixture and once per real screen, in both lightings.

1. **Contrast.** `text-quiet` and `text-faint` hold the WCAG ratio for body text against the pane behind them with the root's `data-theme` in either state, and large text and graphics hold the ratio for large text. The colour gate reads the tokens; only an eye reads which tier a screen put where.
2. **Chinese typography.** No line opens with a closing bracket or a full stop, numbers beside Chinese characters sit on the mono figure face, and running text is no wider than the `talk` container.
3. **Motion in both states.** Under `data-motion="off"`, and with the machine asking for reduced motion and no `data-motion="on"`, every animation reaches its final state without a transition and the typing cursor stops blinking. A reduced-motion user loses decoration and never loses information.
4. **Focus.** Walk every screen with Tab alone: the ring is visible on every stop, never clipped, and returns to where it left when a dialog closes.
5. **Hit areas.** Icon buttons measure at least `control-sm` on a desktop pointer, and their `::before` expansion reaches the 44-point touch target.
6. **Empty states.** Each has a shape, a sentence naming what is missing, and an action out. The action may be absent; the sentence may not.
7. **Notices.** A refusal with a form goes inline beside its field, one without a page goes to a toast, and history sits in the drawer. The role matches the seat.
8. **Tables.** Figure columns are right-aligned on `w-figure`, id columns hold `min-w-[24ch]`, and nothing wraps in the middle of a token.
9. **No overflow.** No box draws outside the box that holds it, popovers at the viewport edge included.
10. **Every word sourced.** The wording gate reads the source; the eye confirms the painted result, where a fixture value is the only exception.
11. **Whether it looks right.** The gate measures where boxes landed and what a screen reader is told; it has no opinion about whether a screen is worth looking at. Open `#/gallery` in a real browser and decide.

## The design, entry by entry

These entries say how the client looks: the stylesheet's tokens, the grid, the surfaces, and the shell's parts. Each keeps the label the code and the specification cite it by (§4-33, §7D and so on), so a comment that names `docs/frontend-method.md` §7D lands on the heading below that carries §7D. What a part owes a keyboard or a screen reader, and every interface of `client/src/core/`, is in `client/Spec.lean`; an entry here cites those as `client/Spec.lean` §<label> and the decisions as `client D<n>`. When an entry here and the specification disagree, the specification wins, and this entry is corrected in the same change.

### §3-3 `src/theme.css`

After `@import "tailwindcss"`, the `@theme` block sets Tailwind's default palette, fonts, type sizes, weights, radii, spacing and container widths to `initial`, and then declares this project's tokens: the grey ramp `g0`…`g10`; `accent`, `alert`, `accent-hover`, `alert-hover` and `accent-solid`; the inks `text`, `text-quiet`, `text-faint` and `text-disabled`; the faces `sans` and `mono`; the sizes and weights `figure`, `title`, `heading`, `label`, `body` and `note`; the spacing `tight`, `snug`, `base`, `pane`, `wide` and `section`; the widths `measure` and `page`; the radii `panel`, `card`, `control` and `pill`; the curves `arrive` and `leave` and the durations `short`, `panel` and `page`; glass, as the role `glass` and `--glass-opacity`; the corner exponent `--corner-exponent`; and the blend tier's `--blend-opacity` (§4-43). Every coloured token keeps `calc(<chroma> * var(--chroma))`, so removing colour is still one factor set to zero. **This is the only file in the client where a colour literal may appear**, and `xtask color` holds it to that.

### §4-33 Layout: one 12-column grid, alignment by construction, widths capped by kind of content

**First, the shell is one CSS grid**: 12 columns, 32 px margins, 24 px gutters, and an 8 px baseline that every line height and gap is a multiple of. **The columns are not equal** (client D24): the window, margins included, is cut once at 1 : √2 : 1, three columns go into each side part and six into the middle part, so column lines 4 and 10 are the two silver lines, each centred in a gutter. The middle six columns take their width from that cut and the six side columns share the rest, so a page stays symmetric whatever its margins. The ratio has one authority, `--silver: sqrt(2)` in `theme.css`; `frame` derives `--silver-side` from it and its own container's width, registered as a `<length>` so descendants inherit pixels rather than the formula. At 1920 px a side column is 156.8 px and a middle column 108.5 px, and the middle part is 771 px (795 px on the window), wide enough for the 760 px reading column. The conversation page's `<main>` is not a query container (`main.workspace { container-type: normal }`): a query container carries layout containment, the engine treats it as an independent grid, its `subgrid` reads as `none`, and the panes fall into implicit columns and draw over each other at the left edge; nothing on the conversation page asks `page`. `main.overflow-y-auto` is written into the same `scrollbar-gutter: stable both-edges` rule as `main`, because the utility's plain `stable` would otherwise win and shift every page left by half a scrollbar. **Every region is placed by column lines**: the conversation column, the world layer's panels, the right side and the edge keys are each written as a `grid-column` start and end, and the world layer's panels stand on the same columns through `subgrid`, so two panels share a left edge because they stand on one column line, not because somebody measured them. The columns are the shell container's, not the window's. `mx-auto` appears on no page container; it is allowed only inside a reading column (the conversation thread, a long document). Placement per tier:

| Tier | World layer | Conversation | With the right side open |
|---|---|---|---|
| zen | not rendered | columns 4–9 | the conversation does not move; right side 10–12, so the two stand at √2 : 1 |
| blend | sessions 1–3, commits 10–12 | columns 4–9 | sessions stay at 1–3, the commits fold away; the rest as in zen |
| panorama | sessions 1–3, chosen session 4–9, commits 10–12 (the person's order and widths, default 3, 6, 3) | under the chosen session, the taller of the two rows split 1 : √2, the whole thread with its composer | sessions 1–3, chosen session and conversation 4–9, the commits fold away, right side 10–12 |

The edge keys stand at the foot of column 1 in all three tiers, and the sessions column leaves their height free at its foot. **A shell narrower than 768 px has one column** (the question is asked of the shell's container, not the window, client D30): the margins shrink to 16 px, the conversation takes the whole width, the edge keys move to the start of the composer's chip row, and the world layer and the right side are each a full-screen sheet (`client/Spec.lean` §4-52). **Every other page** (city, building, record…) takes columns 2–11 (lines 2 to 12), centred on the screen as the conversation is: column 1 is the edge keys', and column 12 mirrors it. A page in columns writes `silver-columns`, a middle column exactly as wide as the shell's middle part and two sides sharing the rest, so its middle column's edges stand on the shell's two silver lines (`client/Spec.lean` §4-50 其一).

**Second, width is capped in three steps by kind of content, not by page**: `measure` (520) for paragraphs, `talk` (760) for the conversation, `page` (1040) for forms with tables; tables and code blocks are not capped and grow with their container up to `wide` (1120). Different blocks of one page take different caps, because capping a whole page at its narrowest kind squeezes forms and tables into a paragraph's width. **Third, a grid column is at least 320 px**: `grid-cols-[repeat(auto-fit,minmax(320px,1fr))]`, with no breakpoint, because a breakpoint asks about the container and a column width asks about the content. Table columns have their own rule: text columns `min-w-[12ch]`, figure columns `w-figure`, id columns `min-w-[24ch]`; past the container a table scrolls sideways, and **it never breaks a token across lines**.

### §4-34 Size tokens: control heights, the icon grid, radii, three shadows, hit areas

Without a control-height token every view builds its own height from `py-tight` or `py-snug`, and three buttons on one row come out 26, 28 and 30 px high. The tokens are `--spacing-control-sm|control|control-lg` (28 / 32 / 36), `--spacing-glyph-sm|glyph` (16 / 20; an icon lives in the square it is drawn in), the radii `control 6 | card 8 | panel 12`, and three shadows (`shadow-raise` for a control resting on the page, `shadow-float` for a popover, `shadow-sheet` for a drawer and a dialog). **A surface with a shadow has no border, and a surface with a border has no shadow**, except popovers and glass: the edge keys are glass (a `backdrop-filter` blur and saturation over the `glass` role at `--glass-opacity`, §4-43), and a glass surface has a 1 px border and the float shadow, because it floats over any content: the border separates it from the words behind it, and the shadow says which layer it is on. The shell has four more sizes: the edge keys are 40×40 with radius 14, progressively enhanced with `corner-shape: superellipse(var(--corner-exponent))` (outside `@supports`, a plain `border-radius` of the same radius, §4-43), their icons in an 18 px square; the coin key is 32×32 with an 18 px glyph; the context ring's box is 40×40 with radius 18, a 1 px stroke and 5 px checkpoints; the composer's line is 1 px. Hit areas are at least 28×28 under a desktop pointer and 44×44 on touch; an icon button smaller than that widens its hit area with `::before`. Icons go through one `Glyph name=…` in `parts/glyph.svelte`: the names live in `parts/glyph.ts`, the drawing is the lucide icon set's (admitted under `client/Spec.lean` §7-9), and the stroke width is counted in screen pixels at any size, because views that each draw their own `<svg>` give one set of shapes several homes. City illustrations are pictures, not icons, and stay where they are.

### §4-37 One monospaced face: Geist Mono for Latin text by default; Chinese falls to the device's own font, and the font stack names no CJK face and ships none

The whole interface — running text, labels, figures and code — is set by default in one face, Geist Mono, and hierarchy comes from size, weight and the grey ramp alone; the appearance group can change running text to another face (`FACES` in `core/appearance.ts`). Chinese characters fall to a face this device already has, because that is what an engine does for any glyph that none of the named faces carries, and the platform's own choice is the only one available on terms this project can accept. **Each of two conditions settles this on its own.** Licence: the stack may name only faces the client may redistribute (`fonts/OFL.txt`, `docs/third-party.md` section 4), and the Chinese faces people actually have on Windows and macOS belong to those vendors. Size: a face worth naming is 17,773,244 B as its vendor ships it and 11,266,972 B after `gzip -9`, several times what `tools/xtask/budgets.toml` allows the whole frontend artefact (`frontend_artifact`). **The cost is small**: the fallback face's baseline and x-height differ from the shipped face's, which shows only where one line mixes Latin letters and Chinese characters; naming a CJK face would not remove that, and would only make the result depend on which fonts a machine happens to have. **Chinese still has its own size and leading** (`:root:lang(zh)` in `theme.css`): the note step does not drop 1 px, the line height is 1.6, and the letter spacing is zero — the three say that at one type size Chinese characters are much denser than Latin letters, whatever the face.

### §4-43 Motion, glass and corners are a few tokens in `theme.css`; no curve or duration is written outside it

**Two curves, copied by value from Open Props (`src/props.easing.css` of open-props 1.7.23, MIT), with no package installed (client D20)**: `--ease-arrive` is its `--ease-out-4`, `cubic-bezier(0, 0, .1, 1)`; where the engine knows `linear()` (`@supports (transition-timing-function: linear(0, 1))`) it becomes its `--ease-spring-1`, a spring that overshoots by only 1.7 %, copied verbatim; `--ease-leave` is its `--ease-in-4`, `cubic-bezier(.9, 0, 1, 1)`. Entering and arriving decelerate, and leaving accelerates (refrain Q1): for a transition that runs both ways, the base state writes `ease-leave` and the arrived state writes `ease-arrive`, because CSS takes the timing function from the target state. Tailwind's own `ease-in`, `ease-out` and `ease-in-out` are cleared by `--ease-*: initial` in `@theme`, and a transition that names no curve takes `--default-transition-timing-function`, which points at arrive.

**Three durations, by how far a thing moves** (refrain P6): `short`, 150 ms, for a change of state in place (hover, press, a row fading in); `panel`, 250 ms, for a surface that appears or turns over where it stands (tooltip, popover, modal, the coin key, the composer's line); `page`, 350 ms, for movement across the page (the world layer coming and going, the composer sinking to the foot, the context ring's arc). Tailwind's namespace for durations is `--transition-duration-*`, so the tokens are spelled `--transition-duration-short|panel|page` and the classes `duration-short|panel|page`; `--default-transition-duration` points at `short`. The ambient animation of the city illustration (figures bobbing, windows and thoughts blinking, flags waving) moves nothing across the page and takes none of the three; its periods are written only in `theme.css`.

**The entrances are named classes in `theme.css`, each built from the tokens**: `rise`, `drop`, `slide` and `fade` as before; `shift`, a `short` fade with a 4 px rise, for one settings group replacing another in the panel; `side-in`, a `page` move from 32 px to the right with a fade, for the right side arriving; and `side-in-then`, the same move started half a `page` later, for the level inside it (the right side's editor and terminal under its tab strip), so the inner level begins when the outer one is halfway. The side classes fill backwards only, so no transform stays on an arrived panel, and inside a one-column `.sheet`, which travels in whole, they are off. In Chromium, Firefox and Safari these are plain CSS animations with a `calc()` delay, supported by all three.

**`data-motion="off"` sets the three durations and the default to zero and stops animations**: on the root element it governs the page, on a subtree it governs that subtree (the `#/gallery` fixtures use it that way); the machine's `prefers-reduced-motion: reduce` does the same unless `data-motion` says `on`. The machine side: `xtask motion` (`tools/xtask/Spec.lean` §8-51) refuses `cubic-bezier(`, `linear(` and `steps(` outside `theme.css`, and Tailwind's `duration-<number>`, `duration-[…]` and `ease-[…]`, so the tokens are the only home of a transition's curve and duration.

**Glass is one role, one opacity and one utility**: `--color-glass` is one hop to `g2` (§7A), `--glass-opacity` is its opacity (75 %), and the utility `glass` mixes the two into a background, with `backdrop-filter: blur(24px) saturate(170%)`, a 1 px border and the float shadow (§4-34). **That opacity is the number `xtask color` judges**: laid over the brightest surface the page can draw (`raised-hover`), glass must still let `--color-text` reach the APCA tier it claims, judged once in each lighting; lowered until it no longer does, the gate turns red, so the number is also a floor. Glass is only for the small surfaces of the edge layer: the edge keys, and the mailbox and the settings sheet if they take glass, through this same utility; the conversation's panels use alpha, and the world layer takes no glass (refrain P12). In four cases glass turns solid and no information is lost: the appearance group's `glass: off`; `prefers-reduced-transparency: reduce` (only Chromium reports it, which is why the switch exists as well); an engine without `backdrop-filter` (these three draw `--color-glass` fully opaque and unblurred); and `forced-colors: active` (a `Canvas` ground and a `CanvasText` border).

**Corners have one exponent**: `--corner-exponent` is the parameter of CSS `superellipse()`, 1.6, as on the template's edge keys. Inside `@supports (corner-shape: superellipse(2))` every `rounded-*` utility writes `corner-shape: superellipse(var(--corner-exponent))`; outside it, a `border-radius` of the same radius. The SVG side (`cornerPower` in `city/shape.ts`) reads the same token and converts it to the Lamé curve's exponent 2^1.6 ≈ 3.03 for `squircle`; when it cannot read it, the exponent is 2, a plain circular arc, as an engine without `corner-shape` draws. One exponent replaces five homes of one shape: each of the four radius steps once carried an exponent of its own (2, 1.5, 1.5, 1), and the city drawing wrote 4.

**The blend tier's opacity**: `--blend-opacity` is the world layer's opacity in the blend tier (§7H), 60 % by default, and lives only in `theme.css`; a slider in the appearance group changes it between 30 % and 90 % in steps of 5 (refrain U2). Until the person moves it, the root element carries no such attribute, and the slider shows the value the page draws now. The slider is a native `<input type="range">` whose keys are the APG Slider's (`client/Spec.lean` §7-11); each value it settles on takes effect at once, and the card's foot says it is saved (`client/Spec.lean` §4-36).

**Under forced colours**: glass as above; the composer's line becomes a `CanvasText` gradient; the context ring's two checkpoints are drawn solid in `CanvasText`, their order told by where they sit on the ring, because forced colours replace both green and red.

### §7A Surface roles: what a surface is for has one home

**The eleven grey steps are the authority on values, the roles are the authority on use, and the two layers do not overlap.** `--color-g0…g10` and the hard assertion of eleven steps at `tools/xtask/src/color.rs:164` stay exactly as they are; the roles are a layer above them.

Without roles, every hand-written `bg-g2` or `border-g3` in `client/src/views/` is a home of the fact "what surface counts as a raised control" — the views had two hundred and thirty-six of them —, none of them is the authority, and when two disagree nobody can see it.

#### §7A-1 A role is one hop, not a value

```css
--color-raised: var(--color-g2);
```

**No literal.** An `oklch()` copied beside its step becomes at once a second home of that value, retuned by hand whenever the step moves; a hexadecimal alias is worse, because `tools/xtask/src/color/tables.rs` keeps only declarations whose value parses as `oklch()`, so the alias is **silently ignored rather than refused**. One hop also lets one declaration serve both lightings: the light block restates every step, and a role that points at a step follows it without being declared there again.

#### §7A-2 A closed vocabulary

The role names live in `ROLES` in `tools/xtask/src/color/roles.rs`, 22 of them: four surfaces (`page` / `chrome` / `raised` / `raised-hover`), one fill that lets what is behind it show through (`glass`, §4-43), three fills that are not navigation (`speech` / `track` / `disabled`), one fill for marks (`mark`), three edges (`edge` / `edge-panel` / `edge-input`), one ink for text on a solid colour (`on-accent`), and nine of the city illustration's own (`drawn-*`).

**Adding a row is a design decision.** A role earns a name only when a person can say in one sentence, without naming a step, what question it answers. In the draft, `inert` and `resting` were one step apart and no reader could say which of them a given dot was; they are one `mark` now.

#### §7A-3 The gate's four rules (`cargo xtask color`)

1. Every `--color-*` that is neither a step nor a text token, and whose value is not `oklch()`, is a name in `ROLES` and **one hop** to a declared step;
2. Every name in `ROLES` is **declared exactly once** in the stylesheet;
3. Every role has **at least one reader** in `client/src`; a role without a reader is a name with a value nobody reads, and it is deleted rather than kept;
4. No file other than `theme.css` **spells a step utility** (`bg-g2`, `border-g3`, `fill-g9` …).

**So the gate has one rule more, not one fewer**: a hexadecimal alias neither parses as `oklch()` nor is one hop, and both rules stop it.

#### §7A-4 Figure and ground: the page is one surface, and the right side is the only raised panel

`tools/xtask/src/color.rs` holds the contract that g0 is the page, and both ends of the ramp are hard assertions. Content and shell both stand on `page`: regions are separated by the grid and by 1 px rules, not by darker or lighter surfaces, because a box that announces itself competes with the work it frames. **The right side's frame is the only surface raised to `chrome`**: its tab strip and the terminal. The editor stays on `page`, so the deepest area on the screen is still the thing being read and written. The edge keys are glass: the `glass` role at `--glass-opacity` lets what is behind them show through (§4-34, §4-43).

The distance that matters is still `page` to `raised`: a control has to look pressable without a border.

### §7B ACCENT says "moving or chosen", ALERT says "needs the person, or something is missing"

**The accent answers one question only: something here is moving, or this is the chosen one.** Its seats: the status dot of a running run and its pulse, the 2 px left bar of a chosen row, the focus ring (`color-mix` cut to sixty per cent), the tick under the layer key that marks the current tier, checkpoints on the timeline, the added lines of a diff, a successful exit code, and the chosen session's lane in the commits column. **The alert answers the other question only: this needs the person, or something is missing here**: the status dot of a run waiting for you, the mailbox count, a tab with unsaved changes, the removed lines of a diff. **Green and red appear only as the context ring's two checkpoints** (§7J): the two dots must be told apart at a glance on a white ring, and accent and alert already mean something, so they are the only two other hues in the interface.

Every other "current" or "chosen" is said by one step of surface difference: the segmented control's thumb is `raised-hover`, and a chosen row or tab is a faint layer over the page.

The rule is still one sentence that can be checked: **the element with the highest contrast on the whole screen should be "stop"**. The coin key's stop face is a solid glyph on the page colour, and in both lightings it is the largest difference in brightness on the screen, because it is the thing a person has to hit in one try in a hurry.

### §7D One home per screen: a fact is drawn once on a screen

**On one screen, one place draws a fact.** No line of readings stands permanently at the foot of the window: each reading is where its reader already looks, and each tier has a seat table.

| Fact | zen, blend | panorama |
|---|---|---|
| Model, effort (before the session starts) | the two choices at the right end of the composer's settings row | the same two choices on the settings row of the conversation under the chosen session; on one column, where the conversation is a band, `/model` and `/effort` (the band draws no settings row) |
| Model, effort (after the session starts) | the first message head | the chosen session's gauge, the "model" cell |
| Room, gate, sandbox | the three chips at the left end of the settings row | the address line under the chosen session's title, and the gauge's "bounds" cell; the conversation under the chosen session also draws its settings row with the three chips |
| Context | the context ring | the context ring; the gauge's "context" cell writes the exact figures and draws no bar |
| This session's cost, the whole city's cost | not drawn | the gauge's "cost" cell |
| Time | message heads to the second; tool lines the duration in milliseconds, with the finishing ISO instant in their tooltip | the timeline: its head writes the date and the time zone once, each row `HH:MM:SS.mmmZ` |
| A commit's identity (oid, B3, parents) | not drawn | the commits column |
| Connection | the disconnection banner (only while disconnected) and the tab's icon (`core/mark.ts`) | the same |
| The city process's readings (processor and memory) | the performance page (`#/monitor`); a one-line summary beside the settings tree's "performance" entry | the same |

**The ring and the "context" cell are the one place a fact is drawn twice**: the ring draws the proportion and the two checkpoints, the cell writes the figures, and both read the same answer; the ring is where the eye already is, the cell is where an exact figure is read, and without either one the person would have to change tier to see the other half. **zen and blend draw no cost** (the person's ruling): those two tiers keep only the work in hand, and what it cost is read in panorama. **Risk readings still need no searching**: when the gate cannot be read, the gate chip carries the `asks` mark of `client/Spec.lean` §7C, and in panorama the same mark lands on the "bounds" cell. An open sandbox carries no mark: it is the setting the User chose for the building and nothing they must act on, so its words alone state it (`talk/bounds.svelte`). Once the session has started, model and effort are no longer controls on the composer, because a control that cannot be pressed is worse than a word, and the first message head is the one record of that freeze.

**Current state**: the "model" cell and the first message head read the effort from the run's opening (`Opening.effort`, which `run_started` carries from the first line, `crates/accounting/Spec.lean` §8-33), so a session states its effort before its first checkpoint; both write it in one phrase (`talk/frozen.ts`). The bounds are drawn twice in panorama: the conversation under the chosen session draws the full settings row since the workbench gave it the whole thread (`client/Spec.lean` §7K), and the gauge's "bounds" cell still writes them. The summary of process readings (`watchSummary` in `watching.ts`) is read by the settings tree's "performance" entry while the settings sheet is open (`client/Spec.lean` §7L).

### §7E The edge keys

Three keys stand one above the other at the foot of column 1: layers, mailbox and settings, with lucide's `layers`, `inbox` and `settings` icons. **Nothing stands above the three keys**: cost lives in the chosen session's gauge (§7D), and the connection shows only as a banner while it is lost.

- **Layers** changes the tier (§7H): a press made over the settings panel or another page first returns to the conversation, in the same view transition, and the focus lands in the composer (`client/Spec.lean` D48); each press cycles zen, blend, panorama and back to zen, and three 4×2 ticks under the key mark the current tier in accent; `\` is the same action's key. Holding the key, or `\`, for more than 300 ms enters blend for as long as it is held and returns to the tier before on release, so a look at the world layer does not change the tier the person chose.
- **Mailbox** pushes the mailbox in from the window's left edge, drawn under the three keys (`client/Spec.lean` §4-49, client D60), and Accel-B is its key; the badge counts only what needs the person, an ordinary unread refusal is a dot without a number, and while the link is not `live` a bar stands at the key's foot.
- **Settings** opens the settings sheet (`client/Spec.lean` §7L), and Accel-, is its key.

**Names and keys appear on demand, not permanently**: under pointer hover or keyboard focus a key's name and shortcut appear to its right, such as "Mailbox · 3 waiting for you Ctrl B"; holding the accelerator alone for more than 300 ms shows all three names at once, and they go on release, when the window loses focus, or when composition starts (hold to reveal). Names and keys are both read from `core/keys.ts`, with no second spelling. On touch the first tap performs the action, so no name needs hover to be read. The keys of all three are in `client/Spec.lean` §7-11.

### §7F The right side: editor above, terminal below

The right side is a working surface laid over the grid (`client/Spec.lean` §4-27). At its top is a 48 px tab strip with one tab for each open file and terminal, where a file with unsaved changes carries an alert dot; the collapse key is at the strip's right end. Below it the editor stands above the terminal, with a divider between them that can be dragged (its keys are the divider's in `client/Spec.lean` §7-11). One line above the editor writes the path and the two commits being compared; the added and removed lines of a diff take the accent's and the alert's pale grounds, and the cursor's line carries a 2 px accent left bar. The terminal's two head lines are the command, and then the duration, the exit code, the finishing ISO instant and the number of lines cut, with an "original" entry at the right end; output does not wrap and scrolls sideways.

**The tab strip** has one tab for each open item, and a terminal's tab carries the terminal glyph. The four readings `source / preview / diff / versions` belong to a document, and RefRain offers them in its own head, because RefRain's entry takes only `{ building, path, version }` and nothing on the strip can hand it a reading; how a call is read is decided by the call (`client/Spec.lean` §4-45). When only one region has an item, that region takes the whole height and no divider is drawn. **Current state**: the alert dot on the tab of a file with unsaved changes waits until RefRain hands its draft state to the tab strip; the terminal's exit code reads `Call.exit_code` (`client/Spec.lean` §4-58).

### §7H The three layers and the three tiers

**Three layers**: the world layer at the bottom (sessions, commits and files), the conversation layer above it (thread, message heads, tool lines), and the edge layer floating over both (the edge keys, the mailbox, the settings sheet and the composer). **Words never lie over words**: a text panel of the world layer stands only beside the conversation column or in its place, never under the conversation's words, because text over text reads slowest at transparencies between 5 % and 50 % (Harrison, Ishii, Vicente, Buxton 1995).

**The three tiers** say how much of the world layer is drawn. The preference is `tier` in `core/prefs.ts`, one of `zen`, `blend` and `panorama` (client D17):

- **zen**: the world layer is not rendered. It is not mounted, so its questions are not asked either, and the screen holds only the conversation and the edge keys.
- **blend**: the world layer appears as whole panels on both sides of the conversation, sessions on the left and the place column on the right (`client/Spec.lean` §7K), each with a 1 px outline and no blur: the outline separates a panel from the words beside it, and a blur would smear a surface whose opacity is already lowered (refrain roadmap P3, P4). The panels are lowered to `--blend-opacity` (sixty per cent by default, adjustable in the appearance group, §4-43). The place column takes neither pointer nor keyboard (`inert`), because a panel that cannot be clicked but can be tabbed into would carry the focus somewhere hard to see; the sessions panel takes both, and returns to full opacity under the pointer or the focus, because a click on a session there is how the person moves the conversation to it (`client/Spec.lean` §7K, `client D44`). A column label is plain text in this tier, not a menu. Nothing lies under the conversation's words. When the right side opens, the world layer folds away (`client/Spec.lean` §4-27).
- **panorama**: the world layer is the workspace (`client/Spec.lean` §7K), and the conversation stands under the chosen session, the two rows cut 1 : √2 with the conversation the taller, drawing the whole thread and the composer. On one column the world layer is a sheet and the conversation a band under it (§7I) that can still send and steer. The world layer follows the workspace: beside the mayor's room the place column draws the city, and beside a room in a building it draws that building's commit graph and files.

**Every launch opens in zen**: the page is opened to talk, so it starts with the conversation alone, whatever tier the last visit ended in. The tier is held by the tab only: it is not written to the browser's storage or sent to the city, and the city's answer does not change it (`client/Spec.lean` D47); the appearance group's tier card changes the same tier as the layers key. **Changing the tier does not rebuild the composer**: in all three tiers the composer is one element, and a change of tier changes only the column it stands in and its height, so the input, the selection, an input method's composition and the focus all stay, because a tier change that interrupted typing would cost the sentence the person is writing. The world layer comes and goes with a `duration-page` opacity transition (coming takes `--ease-arrive`, §4-43), and the composer's move animates only `transform`; with motion off, everything goes straight to its final state.

### §7I The composer: words above, one line, settings below

The composer is laid out like a page: the words on top, a 1 px line under them, and the settings under the line. It has no box, no ground and no shadow.

**The line follows the words.** While the box is empty, only a short stretch of the line leaves the left edge, fading out within 160 px, like a terminal that shows only its prompt; while typing, the solid line runs to the end of the words and then fades over 160 px, so the first words written read like progress; after sending, it draws back. When the box is both empty and unfocused, the line drops to half opacity. It is a `duration-panel` transition (`--ease-arrive`, §4-43) on `@property --typed` (`<length>`); the width of the words is measured with canvas `measureText` in the box's font, the longest line when there are several, capped at the box's width.

**The coin key**: send and stop are two faces of one key (client D18). One decision says which face is up: a run in front (`core/in_front.ts`) and an empty box show the stop face; words in the box show the send face, and while a run is running that send is a steer, whose landing the key's name says as `client/Spec.lean` §4-13 describes; with neither, the send face is faded, cannot be pressed, and its tooltip says why. Turning over is a `duration-panel` transition of `rotateY(180deg)`, which becomes a cross-fade when motion is off. The key has no border of its own: its ground is the page colour and it is only an 18 px glyph; hover lays 8 % of the ink colour over it, and a press shrinks it to 94 %. The microphone appears only when this city has a transcription endpoint (`client/Spec.lean` §4-16), drawn to the left of the ring.

**The settings row**: three chips at the left end, room, gate and sandbox. At the right end, three choices, model, effort and mode, appear only before the session starts; after it starts they are frozen facts in the first message head (§7D). **"Who is listening" is in the room chip**: when the room chip opens, its menu first reads who is listening — the city, the building, the residents and this run, one per line, each with how large the passage was that the room's latest run was told for it, which files it was read from, and how much the budget cut (`talk/listening.svelte`, reading `Query::Prefix`, the same answer the run page's prompt lens reads) — and then lists the rooms to go to (client D36). The full text is still on the run page. **The run policy is one control after the mode**, also only before the session starts: its face is the write limit, followed by the admission requirement or the landing when either is not the first value, so a trial run is never hidden behind a closed menu; its menu has three columns, write limit, admission requirement and landing, hangs from the whole settings row at the row's width, stays open while the person picks in several columns, and closes on Escape or a second press of the control (`talk/policy.ts`, `client/Spec.lean` §4-60, client D38). The four values are one `RunPolicy` that every dispatch sends.

**An empty room**: in zen the composer stands on the page's vertical centre line, with one line above it naming the recipient and the room's address (`client/Spec.lean` §4-10); after the first send the same element sinks to the foot with `duration-page` and `--ease-arrive`. The placeholder is "Write to {recipient}…".

**The conversation band on one column is the same composer**: the line above it is the last thing said, laid out as a message in the thread (one line of message head, the body cut to one line), and the line, the ring and the coin key are unchanged. The settings row is not drawn, because the sheet's gauge already writes the room, the gate and the sandbox. On the panorama workbench the conversation is not a band: it draws the whole thread and the full composer (§7H). A sentence that the link did not take, kept in the box, is still said as usual (`RowDraws` in `talk/settings_row.svelte`).

### §7J The context ring

A 1 px ring round the coin key, in a 40 px box with radius 18. It answers a question a person wants answered even in zen: how much of this session's context window is used.

**Drawing**: in the dark lighting an empty window is a whole solid white ring (`solid`), and the used part is swallowed by the page colour clockwise from twelve o'clock; in the light lighting, black and white swap. Only the remaining arc is drawn (`pathLength="100"` with `stroke-dasharray` and `stroke-dashoffset`): an arc of page colour laid over a white ring would leave a ragged seam where the two meet. Two 5 px checkpoints sit on the ring, wrapped in 1.5 px of page colour: green is the first reminder and red the second (handoff) reminder, each at its percentage. On hover or focus one line appears: "82.4k / 200k · 41 % used · ● reminder 30 % · ● handoff 65 %".

**What it reads**: the used amount is `Turn.used.input` of this session's latest turn (the input tokens the provider reported), and the window is `context_tokens` in the endpoint table for the model that answers this session, the same pair the city computes its context reminder from (glossary: context reminder). Both percentages are read from this room's `ConfigAnswer`: the first level is `first`, whose authority is `kernel::consts_policy::CTX_REMINDER_FIRST_PERCENT`, and the second is `second.percent`. The page copies neither (`client/Spec.lean` §4-58). **Which session**: the latest session in the room the composer speaks to; while the room has no session the ring is whole and the tooltip writes only the window's size; when the endpoint reports no `context_tokens`, no ring is drawn and only the coin key remains.

An older city that does not answer `first` gets no green dot on the ring, and the tooltip leaves out the reminder.

## What goes into the commit

The screen, its words in `lang.json`, its fixtures, and a new reading in `tools/xtask/budgets.toml`, with its reason, when the size moved past its slack. When the change alters what a part owes a keyboard or a screen reader, the interaction contract in `client/Spec.lean` §9 changes in the same commit, because that half is held to SPEC-first. When the change moves something an entry of *The design, entry by entry* states — a token, a column line, a seat in a tier — that entry changes in the same commit, so the page and its description never disagree. When a screen offers a choice whose list rests on outside facts — which editors read a file-and-line link, which fonts are installed — the SPEC entry that cites where each fact came from changes with it.
