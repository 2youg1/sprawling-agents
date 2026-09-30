# docs

The documents for people who use, run or change sprawling, grouped by who reads them first.

| Reader | Document | What it answers |
|---|---|---|
| Someone starting | [`getting-started.md`](getting-started.md), in Chinese [`getting-started.zh-CN.md`](getting-started.zh-CN.md) | From nothing installed to a city that does work, with every idea met on the way |
| Someone using a city every day | [`operating.md`](operating.md) | Steering and stopping work, answering residents, driving a city from a script, and the ways out of what goes wrong |
| Someone debugging | [`logging.md`](logging.md) | What goes in the diagnostic log and what goes in the Ledger, and why the two stay apart |
| Anyone | [`glossary.md`](glossary.md) | What each word means; the `lexicon` gate holds the code and the documents to these words |
| Someone changing the code | [`CONTRIBUTING.md`](CONTRIBUTING.md) | The rules, the gate that holds each one, and how to get a green run locally |
| Someone changing a screen | [`frontend-method.md`](frontend-method.md) | How a screen under `client/` is built, settled and accepted |
| Anyone checking what is owed | [`third-party.md`](third-party.md) | Whose work this stands on, and the licence obligations |

Three parts of this directory are the product itself rather than documents about it, so a change to them is a change to what a person runs. [`City.md`](City.md) and the files under [`templates/`](templates/) are compiled into the binary: the city writes `City.md` when it is raised and the templates into its buildings, as [`templates/README.md`](templates/README.md) lists. [`dist/QUICKSTART.md`](dist/QUICKSTART.md) is copied into every release archive.
