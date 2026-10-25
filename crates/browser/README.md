# sprawling-browser

What a resident may see of a web page and do to it, decided for a browser the city drives over WebDriver BiDi. A run looks at a page as a snapshot, clicks, fills a field, presses keys, takes a screenshot, compares two images, looks again after changing code, and finds the same signed-in account the next time through the building's own browser profile.

The package holds the decisions and the value types and none of the I/O. It starts no browser process, holds no socket and downloads no driver; it has no `async` and no file handle. In `crates/sprawling`, `browser_bidi` starts or attaches to the browser, holds the socket and implements this package's `BrowserPort`, and `browser_tool` turns the actions into the `browser` and `usersbrowser` tools. Whether a building may use a browser at all is decided by `city::policy`, and page text is tainted by `kernel::taint` like any other tool result.

The `survey` module reads a drawn page and reports where its boxes went wrong. The `render` gate in `tools/xtask` uses it on this repository's gallery, and the `browser` tool uses it on a page a resident opened.

## Who uses it

Two workspace packages depend on it: `sprawling`, the assembly root, and `xtask`, whose `render` gate reads `survey`. The packages this one may name are its line in the `depmap` block of `ARCHITECTURE.md` section 3.

## The specification

`crates/browser/Spec.lean` is the entry, and its parts live under `crates/browser/spec/`: stale pages, key nesting, the development loop's end, snapshot labels and per-building profiles are proved there. §3 of the entry lists what is assumed about the protocol and what is still open. `ARCHITECTURE.md` section 11, *Specifications in Lean*, says how a SPEC is laid out.

## Building and checking

```bash
cargo nextest run -p sprawling-browser   # the module tests, which need no browser
lake build crates.browser.Spec           # this package's Lean specification
```

`crates/browser/Spec.lean` §2 and §16 say which test checks which unit. `just test` and `just models` run the same checks for the whole workspace.
