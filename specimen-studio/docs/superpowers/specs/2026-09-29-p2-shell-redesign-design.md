# P2: Specimen Studio shell redesign

Date: 2026-09-29. Status: written for review. Parent program:
`2026-09-28-specimen-studio-desktop-design.md` (phase P2, section 4).
Scope: `wdbx/specimen-studio/` browser and desktop editions, which share one
React tree.

## Intent

Donald asked to redesign Specimen Studio into a modern, Next.js-style app
using the open-source design system already in the repository, as the
foundation for Obsidian-style notes, the WDBX console and the 3D editors.

Decisions (Donald, 2026-09-28/29):

- Working model: **hybrid**. An app shell whose center holds one view, or two
  side by side; no full editor tabs.
- Visual direction: **B, dark graphite + teal**, dark first, with a light
  theme toggle (chosen in the visual companion, `shell-direction.html`).
- Full browser/desktop parity (program decision) applies to the shell.
- Ordering: P2 runs before P1. The shell must not assume the P1 command layer
  exists, and must let panels move from direct engine calls to commands later
  without another restructure.

Assumptions (correct if wrong):

- The specimen workflow itself (run, feedback, teach, memory, maintenance,
  save/load) does not change. P2 changes where things live and how they look.
- The white/teal look survives as the light theme, not as the default.
- The `?view=` and `?chapter=` URLs keep working; bookmarks do not break.

## What exists (facts from the code)

- `app/studio.tsx` (3,238 lines) holds one `Studio` component (lines
  144-1642) with 24 `useState`s, all handlers, the shell (sidebar, drawer,
  header, error banner, toast, one dialog switch) and seven views chosen by
  `view` state: studio, nodes, memory, activity, specification, settings, lab.
  Studio's JSX is inline (about 545 lines); the others are local functions;
  only `NativeLab` is its own module.
- Navigation lives only in `?view=…&chapter=…` (`nav()`, `pushState`,
  `popstate`). No global keyboard shortcuts exist.
- `app/studio.css` (3,982 lines) is two stacked layers, the second an
  "Instrument Console" system with about 330 hard-coded colors. There is no
  dark mode (`color-scheme: light`).
- `components/ui/` holds about 60 shadcn/Base UI components (sidebar,
  resizable, command, tabs, sheet, tooltip, kbd, scroll-area, dialog, toast).
  `studio.tsx` uses only `dialog`. Their tokens live in `app/globals.css`,
  which nothing imports, so they render unstyled today.
- `tests/ui-contract.test.js` checks source strings in `app/studio.tsx` and
  `app/studio.css`; the e2e suites select `.sidebar`, `.nav-backdrop` and
  `.toast`.

## Section 1: architecture

Split by responsibility; each file has one job.

- `app/state/use-studio.ts`: a hook that owns the `Specimen`, persistence
  (autosave, `beforeunload`), idle maintenance, `run`, `reviewContext`,
  `giveFeedback`, `save`, `load`, dialogs, notices, errors and undo: every
  state and handler now in `Studio` that is not layout. It returns one
  `StudioModel` object. Its handlers are the single place P1 later swaps for
  commands.
- `app/state/navigation.ts`: parses and writes the URL (`view`, `chapter`, and
  the new `split`), listens to `popstate`. Pure functions plus one hook.
- `app/shell/`: `app-shell.tsx` (layout), `app-sidebar.tsx` (today's
  hand-built sidebar and mobile drawer, restyled, with a desktop icon-rail
  collapse), `top-bar.tsx` (today's header), `command-palette.tsx` (built on
  `components/ui/command`), `console-dock.tsx`, `pane-layout.tsx` (built on
  `components/ui/resizable`), `theme.ts` (resolution and toggle).
- `app/theme.css`: the token source for both themes, imported by
  `app/studio.css`.
- `app/panels/`: one module per view: `studio-panel.tsx` (today's Studio view
  whole: telemetry, workflow rail, composer, conversation, network/trace,
  node inspector, phase strip), `nodes-panel.tsx`,
  `memory-panel.tsx`, `activity-panel.tsx`, `specification-panel.tsx`,
  `settings-panel.tsx`, `lab-panel.tsx` (wraps `NativeLab`). Each takes the
  `StudioModel` (or a narrow slice of it) as props and owns only its local
  UI state (filters, tabs, virtual scroll).
- `app/dialogs/`: `node-form.tsx`, `resource-form.tsx`,
  `attachment-form.tsx`, `dialog-host.tsx` (the switch now at
  `studio.tsx:1506-1639`).
- `app/studio.tsx` shrinks to composing `useStudio`, navigation and the
  shell. `app/page.tsx` and `desktop/main.tsx` keep importing it unchanged.

The Studio view stays one panel. The e2e suites drive conversation, network
and inspector together on `?view=studio` (`Network`, `Inspect Calculate`,
`.inspector`), and keeping them together keeps that contract. The side pane
puts a second view next to it instead (Nodes, Memory, Activity,
Specification, Settings, Lab). A shell-level inspector is deferred to P4,
where it gains backlinks.

## Section 2: layout and interaction

Desktop (≥ 1024 px):

- **Sidebar** (the existing `nav[aria-label="Main navigation"]`, grouped
  Build: Studio, Nodes, Memory; Observe: Activity, Lab; Reference:
  Specification; settings link and theme toggle at the bottom). ⌘/Ctrl-B
  collapses it to a 56 px icon rail with tooltips; the links stay the same
  elements, so each `?view=` link exists exactly once on the page.
- **Center**: one pane, or two via **Open to the side** (command palette, or
  ⌘/Ctrl-\ to open the last other view beside the current one). Resizable divider;
  either pane closes. The same view cannot be open twice. Layout persists in
  the URL as `?view=studio&split=network`.
- **Console dock** (bottom, toggled with backtick, closed by default so the
  composer stays in the first viewport): in P2 the live activity journal
  (the `events` stream); the WDBX console joins it in P3. The composer and
  slash commands stay in the Studio panel.
- **Top bar**: today's header (view title as the page's only `h1`, specimen
  name, Starter/Load/Save) plus the palette trigger ("Search or run a
  command", ⌘K). The secondary pane titles itself with an `h2`.
- **Command palette** (⌘/Ctrl-K, `cmdk`): go to any view, open a view to the
  side, close the side pane, save, load, reset to starter, toggle theme, toggle sidebar
  and dock, open a specification chapter, run the starter prompts. Every
  entry shows its shortcut.

Below 1024 px there is one pane (a split collapses to its primary view and
restores when the window widens). At 760 px and below the sidebar is the
existing drawer, unchanged in behavior: `.sidebar.is-open`,
`#workspace-navigation`, the Open/Close navigation buttons, the focus trap,
`inert` on the closed drawer and on `main` while it is open, and a
`.nav-backdrop` that leaves the DOM when closed. The dock is available there
as a full-width panel. Keyboard shortcuts never
fire while focus is in a text input, except ⌘K and Escape.

## Section 3: visual system

- One token source: `app/theme.css`, imported by `app/studio.css` (which both
  entry points already import). `app/globals.css` stays unimported, because
  its base layer would restyle every element. `theme.css` defines the studio
  tokens and the shadcn token names the shell components need. `:root` holds
  the light theme (today's white/teal:
  background `#fff`, primary `#087b6c`), `.dark` holds graphite/teal
  (background `#0e1214`, panes `#141b1e`, lines `#1f2a2e`, text `#d7e0e3`,
  accent `#2ec4b6`, warm accent `#f0b454`), the values shown in the companion.
  Contrast pairs meet WCAG AA (4.5:1 text, 3:1 UI).
- Theme choice: dark by default; a toggle stores `light` or `dark` in
  `localStorage` (a per-viewer convenience; unreadable storage falls back to
  dark without error). Applied as a class on `<html>` before first paint to
  avoid a flash.
- New shell components use Tailwind utilities and the shadcn components.
  Existing panel markup keeps its class names; `studio.css` is migrated so
  every color reads a token (`var(--…)`) instead of one of its roughly 330
  literals, and its two stacked `:root` layers move into `theme.css`. This
  migration is required, not cosmetic: dark is the default, and axe's
  contrast rule runs on whatever renders first. Panels are restyled to Tailwind
  incrementally in later phases, not in P2.
- Motion: panel open/close and the palette use short fades (≤ 150 ms);
  `prefers-reduced-motion` removes them, as the two existing rules already
  do for the studio.

## Section 4: error handling and state

- Error banner, notice toast with Undo and the confirmation dialogs keep
  their markup and behavior (`.toast`, `role="alert"` and the Base UI dialog
  are pinned by the e2e suites).
- Unknown `?view` or `?split` values fall back to `studio` with no split;
  `split` equal to `view` is dropped.
- Busy state (`aria-busy`) is shown on the running pane and in the top bar.
- The mobile drawer keeps its focus trap and Escape handling; dialogs keep
  focus return.

## Section 5: testing and gates

- `tests/ui-contract.test.js` is updated deliberately: its strings move to the
  files that now hold them (for example `console-telemetry` in
  `app/panels/studio-panel.tsx`), and it gains checks for the shell landmarks
  (sidebar collapse, palette, pane layout, dock) and that both theme blocks exist.
- New Bun tests: `navigation.ts` round-trips (`view`, `chapter`, `split`,
  invalid values, `split == view`), theme resolution (stored value, missing
  storage, throwing storage), and the palette's command list (every view
  reachable, no duplicate shortcut).
- `tests/browser.e2e.mjs` and `tests/accessibility.e2e.mjs` keep every
  pinned selector, label and audit-state name (`scripts/qualification-summary.py`
  pins the state and touch-target names). Both honor a new `STUDIO_THEME`
  (`dark` default, or `light`), seeded into storage before load, so each
  theme can be audited; output file names gain the theme only when it is
  set, so CI receipts keep their names. Names that must stay unique on the
  page stay unique: each `?view=` link, `?view=settings`, `.specimen-name`,
  `Stored on this device`, and buttons matching `Network`, `Cancel`,
  `Cycle trace`, `Run dossier`, `Add pattern`.
- A new e2e step opens a split (`?view=studio&split=nodes`) at 1440 px and
  audits it with axe.
- `package.json` `lint:studio` lists `app/studio.tsx`; it expands to
  `app/state`, `app/shell`, `app/panels` and `app/dialogs`.
- Gates: `bun run check` and `bun run test:native` (unchanged by P2, still
  required), plus `bun run test:browser` against a running preview for the
  layout and accessibility suites. README "Use the studio" and
  `RUNTIME-PROFILE.md` interface descriptions are updated to the new layout.

Out of scope: the P1 command layer, notes and backlinks, the 3D editors, the
terminal TUI, and restyling panel internals beyond token migration.
