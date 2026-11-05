# Designing a screen

The view layer settles a screen against the shipped stylesheet and a fixture before it merges; this file holds the design judgement that leads there. Read it with [frontend-method.md](frontend-method.md), which holds the method and the approved design.

Design from the real flow, not the feature list:

- Walk the task as the person would think it, in their order, each step leading into the next.
- Put the primary task one direct step from its context, unless approval or safety requires confirmation.
- Verify the supported range from a narrow touch screen to a wide desktop, with finger, pointer and keyboard. `just shots` draws only 1440 and 1920, so check a phone-width viewport yourself.
- Cut the ambiguous: one control, one meaning. A control whose effect must be guessed is redesigned, not captioned.
- Use layout, state and icons to carry meaning; retain explanatory copy wherever uncertainty remains, and keep an icon-only control's accessible name in `lang.json`, as `xtask render` requires.
- When available, scan a screen with [Impeccable](https://github.com/pbakaus/impeccable) as an advisory source check; it does not replace rendered checks, and the existing gates and `frontend-method.md` stay authoritative. Installing it, enabling hooks or writing its artifacts requires explicit approval.
