# sprawling — start here

You have unpacked a folder. Nothing was installed and no service was registered. While this folder can be written to, nothing outside it is written, and deleting the folder removes everything; when it cannot, the city goes under your home directory instead, as *Where your data is* below says.

## 1 Start it

- **Windows** — double-click `sprawling.exe`.
- **macOS and Linux** — run `./sprawling` from a terminal.

An operating-system warning is not proof of a safe or unsafe download. Verify the selected release's checksum and build provenance, and inspect its code-signing status before deciding whether to run it. The [installation guide](https://github.com/2youg1/sprawling-agents/blob/main/docs/getting-started.md#select-and-verify-a-version) explains these separate checks.

Started with no command, it asks one question before it creates anything, and shows you the folder it is about to create. Answer it and the window becomes the city's CLI, where you can already talk to the Mayor. Type `/web` to open the page at <http://127.0.0.1:8787> in your browser; the window then shows only the address and a pairing code. **That window keeps the city running**: closing the browser stops nothing, and the address opens the page again. Type `/quit` to close the city; closing the window closes it too, after its runs stop at their next safe point. Ctrl+C copies selected text and never closes the city.

The browser `/web` opens is paired with the city without typing anything. In another browser, the page asks for the pairing code the window shows; each code takes one try, and the window then shows a new one.

To make `sprawling` a word your shell resolves from anywhere, run `sprawling install` once. It copies this binary into your own program directory and puts that directory on your PATH; `sprawling install --uninstall` reverses both. Nothing there needs administrator rights.

## 2 Give it a model to think with

This program schedules agents, records what they do, and shows it to you. **It does not think by itself**, so before it can do anything you need one of:

- an API key from a provider that speaks the OpenAI dialect or the Anthropic dialect,
- an agent that speaks ACP, such as a vendor's coding agent, which signs in with your own subscription, or
- a local server that speaks the OpenAI dialect.

## 3 Three steps in the page

1. **connect a provider**, on the welcome page, opens **settings** → **accounts and providers**. Fill in the provider's `base_url`, which face it answers in (`wire_api`), and the key, then **list models** and **attach**. The key goes straight into your operating system's credential service; the page only ever shows a `secret:realm/name` reference afterwards. For a subscription, use **ACP agents**: pick an agent found on this computer or in the bundled catalog, or paste its command, read the exact command line on its consent card, and add it. **add and use here** also hands the current room to that agent, which ends the room's current session record. The city does not sign in for you: if the agent needs a login, sign in with the agent's own program first, outside the city.
2. Under **which model thinks**, choose the model for **main · thinks**. **digest · reads**, which reads long documents on `main`'s behalf, follows it until you choose another.
3. **the Mayor**, the page the city opens on, is the conversation with the city's planner. Write what you want done and press Enter. The Mayor plans the work, raises the buildings it needs, and hands each one its part.

Then **city** shows the buildings working, a count on the mailbox key says how many questions wait for you, and **cost** is what it spent. A line that begins with `/` is a command, on the page and in the CLI alike: `/help` lists them, and `/quit` closes the city.

## Where your data is

In `city/`, beside this file, unless this folder cannot be written to; then the first screen names the folder it chose under your home directory instead. One folder holds the whole history, and it can be moved, copied or deleted as a unit. Each session keeps its own folder inside the building it works in.

## Is there a newer one?

Nothing here updates itself, and update checks run only when you ask. `sprawling version` says which release this is and the day it was cut; `sprawling status --check` asks whether a newer one is published, and **settings** → **which release this is** has the same check behind a button. Verify that the reported channel matches your installation, then follow the [update and rollback guide](https://github.com/2youg1/sprawling-agents/blob/main/docs/getting-started.md#updating). Stop the city and verify a restorable backup before replacing anything.

## Everything else

Run `sprawling help` for the full command list. The complete walkthrough, the vocabulary, and the design are at <https://github.com/2youg1/sprawling-agents>.
