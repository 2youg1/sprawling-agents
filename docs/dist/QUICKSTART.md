# sprawling — start here

You have unpacked a folder. Nothing was installed and no service was registered. While this folder can be written to, nothing outside it is written, and deleting the folder removes everything; when it cannot, the city goes under your home directory instead, as *Where your data is* below says.

## 1 Start it

- **Windows** — double-click `sprawling.exe`.
- **macOS and Linux** — run `./sprawling` from a terminal.

The first time on Windows you will see **"Windows protected your PC"**, because this program carries no code-signing certificate. Choose **More info**, then **Run anyway**.

Started with no command, it asks one question before it creates anything, and shows you the folder it is about to create. Answer it and a console window opens and stays open. **That window is the city**: closing it, or pressing `Ctrl-C` in it, stops the city. Your browser opens at <http://127.0.0.1:8787>; if it does not, open that address yourself.

To make `sprawling` a word your shell resolves from anywhere, run `sprawling install` once. It copies this binary into your own program directory and puts that directory on your PATH; `sprawling install --uninstall` reverses both. Nothing there needs administrator rights.

## 2 Give it a model to think with

This program schedules agents, records what they do, and shows it to you. **It does not think by itself**, so before it can do anything you need one of:

- an API key from a provider that speaks the OpenAI dialect or the Anthropic dialect,
- an Anthropic subscription you can sign in to, or
- a local server that speaks the OpenAI dialect.

## 3 Three steps in the page

1. **connect a provider**, on the welcome page, opens **settings** → **accounts and providers**. Fill in the provider's `base_url`, which face it answers in (`wire_api`), and the key, then **list models** and **attach**. The key goes straight into your operating system's credential service; the page only ever shows a `secret:realm/name` reference afterwards. With a subscription, choose **with a subscription** instead.
2. Under **which model thinks**, choose the model for **main · thinks**. **digest · reads**, which reads long documents on `main`'s behalf, follows it until you choose another.
3. **the Mayor**, the page the city opens on, is the conversation with the city's planner. Write what you want done and press Enter. The Mayor plans the work, raises the buildings it needs, and hands each one its part.

Then **city** shows the buildings working, a count on the mailbox key says how many questions wait for you, and **cost** is what it spent. A line that begins with `/` is a command: `/help` lists them.

## Where your data is

In `city/`, beside this file, unless this folder cannot be written to; then the first screen names the folder it chose under your home directory instead. One folder holds the whole history, and it can be moved, copied or deleted as a unit. Each session keeps its own folder inside the building it works in.

## Is there a newer one?

Nothing here updates itself, and nothing asks npm anything until you do. `sprawling version` says which release this is and the day it was cut; `sprawling status --check` asks whether a newer one is published, and **settings** → **which release this is** has the same check behind a button. Either way you are told what to run, and nothing is replaced for you.

## Everything else

Run `sprawling help` for the full command list. The complete walkthrough, the vocabulary, and the design are at <https://github.com/2youg1/sprawling-agents>.
