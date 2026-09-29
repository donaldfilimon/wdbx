# B1: WDBX component system

Date: 2026-09-29. Program: `~/.claude/plans/continue-with-all-superpowers-brainstorm-serene-parnas.md` (B1).

## Decisions

- **Owned primitives in `components/wdbx/`**, built on the tokens in
  `app/theme.css` and exposed to Tailwind as colors through `@theme inline`
  in `app/studio.css` (`bg-surface`, `text-ink`, `border-line`,
  `text-muted-foreground`, `bg-teal`, `text-on-teal`, …): `WButton` (cva
  variants primary / outline / utility / ghost, icon size, 44 px minimum
  target, visible focus), `Panel` (a labelled `section` when titled),
  `Toolbar`/`ToolbarSpacer` (`role="toolbar"`), `Stat`/`StatGroup` (`dl`),
  `EventTimeline` (`ol` with `time[dateTime]`), `EmptyState`, `CodeBlock`
  (focusable, labelled scroll region). `tests/wdbx-components.test.js` pins
  their semantics with `react-dom/server`.
- **Panels move one at a time** and keep every label, id, class and single
  occurrence the e2e suites pin. A `studio.css` rule is deleted only when no
  source file references its class (checked per class before pruning);
  rules are removed as byte-range spans so the rest of the file is untouched.
- **Ruling:** Specification and Studio are restyled inside B4 (interactive
  spec diagrams) and B3 (live engine views), which rebuild those panels
  anyway; restyling them twice would be waste. Cost if wrong: those two
  panels keep `studio.css` styling until then.

## Done in B1

Activity, Settings (with the kernel's native field bounds), and the Nodes
and Memory toolbars and attachment list. `studio.css` 4,178 → 3,963 lines.
