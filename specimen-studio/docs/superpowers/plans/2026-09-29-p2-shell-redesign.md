# P2: Shell Redesign Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Turn the single 3,238-line `app/studio.tsx` into a themed hybrid shell (collapsible sidebar, optional side pane, console dock, command palette) with a dark graphite + teal default theme, without breaking any pinned UI contract.

**Architecture:** State and handlers move into one `useStudio` hook; each view becomes a panel module; a small shell composes them. Colors move to tokens in a new `app/theme.css` with `:root` (light) and `.dark` (default) blocks. URL state gains `split`.

**Tech Stack:** React 19.2.8, TypeScript, Vinext/Vite, Bun 1.4 tests, Playwright e2e, `cmdk` 1.1.1 (`components/ui/command`), `react-resizable-panels` 4.5.8 (`components/ui/resizable`), Base UI dialog.

**Spec:** `docs/superpowers/specs/2026-09-29-p2-shell-redesign-design.md` (parent: `2026-09-28-specimen-studio-desktop-design.md`, phase P2).

## Global Constraints

- All commands run from `specimen-studio/`. TypeScript strict, two-space indent, single quotes, semicolons; format changed files with `bun run format <paths>`; no explicit `any`.
- Pinned selectors stay exactly as they are (see Review Focus 1). Each `nav[aria-label="Main navigation"] a[href="?view=<id>"]`, `a[href="?view=settings"]`, `.specimen-name`, `.chapter-count`, `.answer-text` (single-segment), `.toast`, `.trace-outcome`, `input[type=file]`, `#prompt`, `#workspace-navigation` exists at most once on the page.
- No visible button name may contain `Network`, `Cancel`, `Cycle trace`, `Run dossier`, `Add pattern`, `Open navigation` or `Dismiss error` except the existing ones (the accessibility test matches those non-exactly).
- `Stored on this device` stays in exactly one visible element.
- `app/page.tsx` and `desktop/main.tsx` keep importing `app/studio` (default export) and `app/studio.css`. `app/globals.css` stays unimported.
- `lib/specimen/*` and `lib/webmcp.ts` are not changed in P2.
- Theme is `dark` by default; the stored choice lives under localStorage key `wdbx-studio-theme`; unreadable or throwing storage means `dark`.
- Gates: `bun run check` and `bun run test:native` exit 0 after every task; `bun run test:browser` against a preview passes at Tasks 4, 5, 6 and 7.
- Commit messages end with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

## Review Focus

1. A moved element loses a pinned name or class (e.g. `Inspect Calculate`, `.inspector`, `Topology zoom`, `Scrollable specimen topology`), or a new shell element duplicates one. Expected: every e2e selector still resolves to exactly one element. Pinned by running `bun run test:browser` in Tasks 4-7.
2. Dark theme renders a panel color that fails WCAG AA contrast (a literal missed by migration, or a token pair too close). Expected: axe clean in both themes. Pinned by the no-literal test (Task 3) and the themed e2e runs (Task 7).
3. `?split=` with an unknown view, the same view as `view`, or `settings`. Expected: dropped silently, primary view renders. Pinned in Task 1.
4. Keyboard shortcuts firing while typing in the composer or a form (backtick in a prompt, ⌘K excepted). Expected: typing inserts text. Pinned in Task 6 (`matchShortcut` tests with an editable target).
5. Theme flash or crash when storage throws (private windows, blocked site data). Expected: dark, no exception. Pinned in Task 2.

---

## File Structure

| Path | Action | Responsibility |
|---|---|---|
| `app/state/navigation.ts` | create | Parse/serialize `view`, `chapter`, `split`; view metadata |
| `app/state/use-studio.ts` | create | All specimen state and handlers now in `Studio` (not layout) |
| `app/shell/theme.ts` | create | Resolve, apply and persist the theme |
| `app/shell/shortcuts.ts` | create | Pure shortcut matcher |
| `app/shell/commands.ts` | create | Pure command-list builder for the palette |
| `app/shell/app-sidebar.tsx`, `top-bar.tsx`, `pane-layout.tsx`, `console-dock.tsx`, `command-palette.tsx` | create | Shell pieces |
| `app/panels/*.tsx` | create | One module per view |
| `app/dialogs/*.tsx` | create | Node, resource, attachment forms; dialog host |
| `app/studio.tsx` | rewrite | Composes hook, navigation and shell (small) |
| `app/theme.css` | create | Tokens for both themes |
| `app/studio.css` | modify | Colors via tokens only; imports `theme.css` |
| `app/layout.tsx`, `desktop/index.html` | modify | Pre-paint theme script |
| `tests/navigation.test.js`, `tests/theme.test.js`, `tests/shell.test.js`, `tests/styles.test.js` | create | Unit tests |
| `tests/ui-contract.test.js` | modify | Read strings across `app/**/*.tsx` |
| `tests/browser.e2e.mjs`, `tests/accessibility.e2e.mjs` | modify | `STUDIO_THEME`, split audit |
| `package.json` | modify | `lint:studio` covers new directories |
| `README.md`, `RUNTIME-PROFILE.md`, `AGENTS.md`, `VALIDATION.md` | modify | Describe the new layout; record the run |

---

### Task 1: Navigation state with `split`

**Files:** Create `app/state/navigation.ts`, `tests/navigation.test.js`.

**Interfaces:**
- Produces:

```ts
export type View = 'studio' | 'nodes' | 'memory' | 'activity' | 'specification' | 'settings' | 'lab';
export const VIEWS: readonly View[]; // the seven, in nav order: studio, nodes, memory, activity, lab, specification, settings
export interface Route { view: View; chapter?: number; split?: View }
export function parseRoute(search: string): Route;
export function routeSearch(route: Route): string; // '?view=…[&chapter=…][&split=…]'
```

- [ ] **Step 1: Write the failing tests** in `tests/navigation.test.js`

```js
import { test, expect } from 'bun:test';
import { parseRoute, routeSearch } from '../app/state/navigation';

test('defaults to studio with no split', () => {
  expect(parseRoute('')).toEqual({ view: 'studio' });
});

test('keeps existing view and chapter URLs working', () => {
  expect(parseRoute('?view=specification&chapter=9')).toEqual({
    view: 'specification',
    chapter: 9,
  });
  expect(parseRoute('?view=settings')).toEqual({ view: 'settings' });
});

test('rejects unknown views and out-of-range chapters', () => {
  expect(parseRoute('?view=nope&chapter=27')).toEqual({ view: 'studio' });
  expect(parseRoute('?view=specification&chapter=0')).toEqual({
    view: 'specification',
  });
});

test('accepts a split view and drops invalid ones', () => {
  expect(parseRoute('?view=studio&split=nodes')).toEqual({
    view: 'studio',
    split: 'nodes',
  });
  expect(parseRoute('?view=studio&split=studio')).toEqual({ view: 'studio' });
  expect(parseRoute('?view=studio&split=bogus')).toEqual({ view: 'studio' });
  expect(parseRoute('?view=studio&split=settings')).toEqual({ view: 'studio' });
});

test('serializes in a stable order that round-trips', () => {
  const route = { view: 'studio', split: 'specification', chapter: 3 };
  expect(routeSearch(route)).toBe('?view=studio&chapter=3&split=specification');
  expect(parseRoute(routeSearch(route))).toEqual(route);
  expect(routeSearch({ view: 'nodes' })).toBe('?view=nodes');
});
```

- [ ] **Step 2: Run and see it fail.** `bun test tests/navigation.test.js`. Expected: FAIL, cannot resolve `../app/state/navigation`.

- [ ] **Step 3: Implement** `app/state/navigation.ts`

```ts
export type View =
  | 'studio'
  | 'nodes'
  | 'memory'
  | 'activity'
  | 'specification'
  | 'settings'
  | 'lab';

export const VIEWS: readonly View[] = [
  'studio',
  'nodes',
  'memory',
  'activity',
  'lab',
  'specification',
  'settings',
];

export interface Route {
  view: View;
  chapter?: number;
  split?: View;
}

const CHAPTERS = 26;
const isView = (v: string | null): v is View =>
  v !== null && (VIEWS as readonly string[]).includes(v);

export function parseRoute(search: string): Route {
  const p = new URLSearchParams(search);
  const view = isView(p.get('view')) ? (p.get('view') as View) : 'studio';
  const route: Route = { view };
  const chapter = Number(p.get('chapter'));
  if (Number.isInteger(chapter) && chapter >= 1 && chapter <= CHAPTERS) {
    route.chapter = chapter;
  }
  const split = p.get('split');
  if (isView(split) && split !== view && split !== 'settings') {
    route.split = split;
  }
  return route;
}

export function routeSearch(route: Route): string {
  const p = new URLSearchParams();
  p.set('view', route.view);
  if (route.chapter) p.set('chapter', String(route.chapter));
  if (route.split) p.set('split', route.split);
  return `?${p}`;
}
```

Settings cannot be a split target because the e2e contract requires `a[href="?view=settings"]` to be unique and the settings form has one save flow; keeping it primary-only avoids two forms.

- [ ] **Step 4: Run.** `bun test tests/navigation.test.js`. Expected: 5 pass.
- [ ] **Step 5: Commit.** `git add app/state/navigation.ts tests/navigation.test.js && git commit -m "feat(specimen-studio): route model with split pane"`

### Task 2: Theme tokens and theme resolution

**Files:** Create `app/theme.css`, `app/shell/theme.ts`, `tests/theme.test.js`. Modify `app/studio.css` (first line), `app/layout.tsx`, `desktop/index.html`.

**Interfaces:**
- Produces:

```ts
export type Theme = 'dark' | 'light';
export const THEME_KEY = 'wdbx-studio-theme';
export function resolveTheme(storage: Pick<Storage, 'getItem'> | undefined): Theme;
export function storeTheme(storage: Pick<Storage, 'setItem'> | undefined, theme: Theme): void; // never throws
export function applyTheme(root: { classList: DOMTokenList }, theme: Theme): void; // toggles class 'dark'
export const THEME_BOOT_SCRIPT: string; // inline pre-paint script
```

- `app/theme.css` defines, in both `:root` and `.dark`: `--bg`, `--surface`, `--surface-2`, `--raised`, `--line`, `--line-strong`, `--ink`, `--ink-soft`, `--muted`, `--teal`, `--teal-strong`, `--teal-soft`, `--on-teal`, `--amber`, `--amber-soft`, `--danger`, `--danger-soft`, `--violet`, `--violet-soft`, `--info`, `--info-soft`, `--shadow`, `--sidebar-bg`, `--sidebar-ink`, `--sidebar-muted`, `--sidebar-active`, `--focus`; plus the shadcn names the shell components read (`--background`, `--foreground`, `--popover`, `--popover-foreground`, `--border`, `--input`, `--ring`, `--muted-foreground`, `--accent`, `--accent-foreground`, `--primary`, `--primary-foreground`) mapped onto them.

- [ ] **Step 1: Write the failing tests** `tests/theme.test.js`

```js
import { test, expect } from 'bun:test';
import { readFile } from 'node:fs/promises';
import {
  THEME_KEY,
  applyTheme,
  resolveTheme,
  storeTheme,
} from '../app/shell/theme';

const mem = (v) => ({ getItem: (k) => (k === THEME_KEY ? v : null) });

test('dark is the default', () => {
  expect(resolveTheme(undefined)).toBe('dark');
  expect(resolveTheme(mem(null))).toBe('dark');
  expect(resolveTheme(mem('purple'))).toBe('dark');
});

test('a stored light choice wins', () => {
  expect(resolveTheme(mem('light'))).toBe('light');
});

test('throwing storage falls back without throwing', () => {
  const broken = {
    getItem() {
      throw new Error('blocked');
    },
    setItem() {
      throw new Error('blocked');
    },
  };
  expect(resolveTheme(broken)).toBe('dark');
  expect(() => storeTheme(broken, 'light')).not.toThrow();
});

test('applyTheme toggles the dark class', () => {
  const set = new Set();
  const root = {
    classList: {
      toggle: (c, on) => (on ? set.add(c) : set.delete(c)),
    },
  };
  applyTheme(root, 'dark');
  expect(set.has('dark')).toBe(true);
  applyTheme(root, 'light');
  expect(set.has('dark')).toBe(false);
});

const TOKENS = ['--bg', '--surface', '--surface-2', '--raised', '--line',
  '--line-strong', '--ink', '--ink-soft', '--muted', '--teal', '--teal-strong',
  '--teal-soft', '--on-teal', '--amber', '--amber-soft', '--danger',
  '--danger-soft', '--violet', '--violet-soft', '--info', '--info-soft',
  '--shadow', '--sidebar-bg', '--sidebar-ink', '--sidebar-muted',
  '--sidebar-active', '--focus', '--background', '--foreground', '--popover',
  '--popover-foreground', '--border', '--input', '--ring',
  '--muted-foreground', '--accent', '--accent-foreground', '--primary',
  '--primary-foreground'];

test('both theme blocks define every token', async () => {
  const css = await readFile(new URL('../app/theme.css', import.meta.url), 'utf8');
  const block = (sel) => css.slice(css.indexOf(`${sel} {`), css.indexOf('}', css.indexOf(`${sel} {`)));
  for (const sel of [':root', '.dark']) {
    for (const t of TOKENS) expect(block(sel)).toContain(`${t}:`);
  }
});
```

- [ ] **Step 2: Run and see it fail.** `bun test tests/theme.test.js`. Expected: FAIL, module not found.

- [ ] **Step 3: Implement** `app/shell/theme.ts`

```ts
export type Theme = 'dark' | 'light';
export const THEME_KEY = 'wdbx-studio-theme';

export function resolveTheme(
  storage: Pick<Storage, 'getItem'> | undefined,
): Theme {
  try {
    return storage?.getItem(THEME_KEY) === 'light' ? 'light' : 'dark';
  } catch {
    return 'dark';
  }
}

export function storeTheme(
  storage: Pick<Storage, 'setItem'> | undefined,
  theme: Theme,
): void {
  try {
    storage?.setItem(THEME_KEY, theme);
  } catch {
    // Storage can be blocked; the choice then lasts for this page only.
  }
}

export function applyTheme(
  root: { classList: Pick<DOMTokenList, 'toggle'> },
  theme: Theme,
): void {
  root.classList.toggle('dark', theme === 'dark');
}

export const THEME_BOOT_SCRIPT = `(()=>{let t='dark';try{if(localStorage.getItem('${THEME_KEY}')==='light')t='light'}catch{}if(t==='dark')document.documentElement.classList.add('dark')})()`;
```

Write `app/theme.css`. Light values are today's identity (from the Instrument Console `:root` at `app/studio.css:2538-2556`); dark values are the companion's graphite palette:

```css
:root {
  color-scheme: light;
  --bg: #f3f5f1;
  --surface: #fbfcf9;
  --surface-2: #f4f6f3;
  --raised: #ffffff;
  --line: #d8e0da;
  --line-strong: #b9c6bf;
  --ink: #102b27;
  --ink-soft: #2b4540;
  --muted: #5a6d67;
  --teal: #087b6c;
  --teal-strong: #05695d;
  --teal-soft: #e3f1ec;
  --on-teal: #ffffff;
  --amber: #8a5a00;
  --amber-soft: #fff4dc;
  --danger: #b3261e;
  --danger-soft: #fff1ee;
  --violet: #6a3fa0;
  --violet-soft: #f2ecfa;
  --info: #1d5fa8;
  --info-soft: #eaf2fb;
  --shadow: 0 1px 2px rgb(16 43 39 / 8%), 0 8px 24px rgb(16 43 39 / 6%);
  --sidebar-bg: #06372f;
  --sidebar-ink: #e6f2ee;
  --sidebar-muted: #a9c7bd;
  --sidebar-active: #0d4a40;
  --focus: #087b6c;
  --background: var(--bg);
  --foreground: var(--ink);
  --popover: var(--raised);
  --popover-foreground: var(--ink);
  --border: var(--line);
  --input: var(--line);
  --ring: var(--focus);
  --muted-foreground: var(--muted);
  --accent: var(--teal-soft);
  --accent-foreground: var(--ink);
  --primary: var(--teal);
  --primary-foreground: var(--on-teal);
}

.dark {
  color-scheme: dark;
  --bg: #0e1214;
  --surface: #12181b;
  --surface-2: #101618;
  --raised: #141b1e;
  --line: #1f2a2e;
  --line-strong: #2c3a40;
  --ink: #d7e0e3;
  --ink-soft: #b8c6cb;
  --muted: #8aa0a7;
  --teal: #2ec4b6;
  --teal-strong: #5fd6ca;
  --teal-soft: #0f2a2a;
  --on-teal: #06201d;
  --amber: #f0b454;
  --amber-soft: #2a2010;
  --danger: #ff8a7a;
  --danger-soft: #2c1512;
  --violet: #c4a8ff;
  --violet-soft: #221a33;
  --info: #8cc2ff;
  --info-soft: #132235;
  --shadow: 0 1px 2px rgb(0 0 0 / 40%), 0 8px 24px rgb(0 0 0 / 35%);
  --sidebar-bg: #0a0d0e;
  --sidebar-ink: #d7e0e3;
  --sidebar-muted: #8aa0a7;
  --sidebar-active: #172024;
  --focus: #5fd6ca;
  --background: var(--bg);
  --foreground: var(--ink);
  --popover: var(--raised);
  --popover-foreground: var(--ink);
  --border: var(--line);
  --input: var(--line-strong);
  --ring: var(--focus);
  --muted-foreground: var(--muted);
  --accent: var(--teal-soft);
  --accent-foreground: var(--ink);
  --primary: var(--teal);
  --primary-foreground: var(--on-teal);
}
```

Add `@import './theme.css';` after the two `@import` lines at the top of `app/studio.css`. In `app/layout.tsx` add `suppressHydrationWarning` to `<html>` and, as the first child of `<body>`, `<script dangerouslySetInnerHTML={{ __html: THEME_BOOT_SCRIPT }} />` (import from `./shell/theme`). In `desktop/index.html` add the same script inline in `<head>` (copy the string's value).

- [ ] **Step 4: Run.** `bun test tests/theme.test.js && bun run check`. Expected: 5 pass; check exit 0.
- [ ] **Step 5: Commit.** `feat(specimen-studio): theme tokens and dark-by-default resolution`

### Task 3: Migrate `studio.css` colors to tokens

**Files:** Modify `app/studio.css`. Create `tests/styles.test.js`.

- [ ] **Step 1: Write the failing test** `tests/styles.test.js`

```js
import { test, expect } from 'bun:test';
import { readFile } from 'node:fs/promises';

const css = await readFile(new URL('../app/studio.css', import.meta.url), 'utf8');

test('studio.css uses theme tokens instead of color literals', () => {
  const literals = css.match(/#[0-9a-fA-F]{3,8}\b|\b(?:rgb|rgba|hsl|hsla|oklch)\(/g) ?? [];
  expect(literals).toEqual([]);
});

test('studio.css defines no :root color layer of its own', () => {
  expect(css).not.toMatch(/:root\s*\{[^}]*--(?:ink|teal|line|pale)\s*:/);
});
```

- [ ] **Step 2: Run and see it fail.** `bun test tests/styles.test.js`. Expected: FAIL listing about 341 literals.

- [ ] **Step 3: Migrate with a one-off classifier** (run from the scratchpad, not committed). It maps every literal to the nearest token by role:

```python
import re, colorsys, pathlib
p = pathlib.Path('app/studio.css'); s = p.read_text()
def rgb(h):
    h = h.lstrip('#'); h = ''.join(c*2 for c in h) if len(h) in (3, 4) else h
    return tuple(int(h[i:i+2], 16) / 255 for i in (0, 2, 4))
def token(r, g, b, prop):
    h, l, sat = colorsys.rgb_to_hls(r, g, b); hue = h * 360
    border = 'border' in prop or 'outline' in prop
    if sat < 0.18 or l > 0.93:
        if l > 0.985: return 'var(--raised)'
        if l > 0.955: return 'var(--line)' if border else 'var(--surface)'
        if l > 0.90: return 'var(--line)' if border else 'var(--surface-2)'
        if l > 0.75: return 'var(--line-strong)' if border else 'var(--line)'
        if l > 0.45: return 'var(--muted)'
        if l > 0.25: return 'var(--ink-soft)'
        return 'var(--ink)'
    if 150 <= hue < 200:
        if l > 0.85: return 'var(--teal-soft)'
        if l < 0.22: return 'var(--sidebar-bg)'
        return 'var(--teal-strong)' if l < 0.28 else 'var(--teal)'
    if 20 <= hue < 60: return 'var(--amber-soft)' if l > 0.8 else 'var(--amber)'
    if hue < 20 or hue >= 330: return 'var(--danger-soft)' if l > 0.8 else 'var(--danger)'
    if 250 <= hue < 330: return 'var(--violet-soft)' if l > 0.8 else 'var(--violet)'
    return 'var(--info-soft)' if l > 0.8 else 'var(--info)'
out = []
for line in s.split('\n'):
    prop = line.split(':')[0].strip()
    line = re.sub(r'#[0-9a-fA-F]{3,8}\b', lambda m: token(*rgb(m.group()), prop), line)
    out.append(line)
p.write_text('\n'.join(out))
```

Then handle the remaining `rgb(...)`/`rgba(...)`/`oklch(...)` uses (about 11) by hand. Shadows become `var(--shadow)`; translucent overlays (e.g. the drawer backdrop) become `color-mix(in srgb, var(--ink) 40%, transparent)`. Delete the two `:root` blocks' color variables (`app/studio.css` ~18-29 and ~2538-2556), keeping the non-color ones (`--sidebar` widths, fonts) in a single remaining `:root` block. Replace `var(--pale)` with `var(--bg)`, `var(--console-*)` with the matching token (`--console-surface` → `--surface`, `--console-raised` → `--raised`, `--console-dark` → `--sidebar-bg`, `--console-dark-2` → `--sidebar-active`, `--console-soft` → `--surface-2`, `--console-accent-soft` → `--teal-soft`, `--console-shadow` → `--shadow`), and `color-scheme: light` with nothing (theme.css sets it). Sidebar text colors become `--sidebar-ink`/`--sidebar-muted`.

Review the diff by eye for three roles the classifier cannot know: text on teal buttons must be `var(--on-teal)`, focus rings `var(--focus)`, and error banners `var(--danger)`/`var(--danger-soft)`.

- [ ] **Step 4: Run.** `bun test tests/styles.test.js tests/ui-contract.test.js && bun run check`. Expected: pass; check exit 0 (`ui-contract` still finds `.console-telemetry`, `.workflow-step`, `.topology-stage`, `repeat(6, minmax(0, 1fr))`).
- [ ] **Step 5: Visual check.** `bun dev`, open `http://localhost:3000` in the browser pane in both themes (toggle by running `localStorage.setItem('wdbx-studio-theme','light'); location.reload()`), screenshot Studio, Nodes, Specification. Fix any unreadable pairing by changing the token used at that rule.
- [ ] **Step 6: Commit.** `refactor(specimen-studio): route every studio color through theme tokens`

### Task 4: Extract `useStudio`

**Files:** Create `app/state/use-studio.ts`. Modify `app/studio.tsx`.

**Interfaces:**
- Produces `export function useStudio(): StudioModel` and `export type StudioModel = ReturnType<typeof useStudio>`. The returned object contains, by these exact names, every state value, setter, ref and handler the views and shell read today: `state, commit, act, announce, openDialog, dialog, setDialog, editing, setEditing, pendingLoad, setPendingLoad, saveStatus, storageEnabled, setStorageEnabled, selected, setSelected, selectedNode, query, setQuery, prompt, setPrompt, promptRef, busy, trace, activeTrace, runOutcome, runInput, networkMode, setNetworkMode, topologyZoom, setTopologyZoom, notice, setNotice, error, setError, showProvenance, setShowProvenance, cycle, ratedContributors, feedbackUnavailable, maxConfidence, run, reviewContext, giveFeedback, save, load, fileRef, abortRef, undo`.
- Navigation (`view`, `chapter`, `nav`) stays in `studio.tsx` in this task; `useStudio(navigate)` takes `navigate: (v: View, chapter?: number) => void` because `run`, `useStudioTools` and the workflow rail call `nav`.

- [ ] **Step 1: Baseline.** Start a preview (`bun run build && bun start --ip 127.0.0.1 --port 4176 &`), then `STUDIO_URL=http://127.0.0.1:4176 bun run test:browser`. Expected: pass before any change (this is the regression oracle for Tasks 4-6). If Playwright browsers are missing, `bunx playwright install chromium` first; record the command in the ledger.
- [ ] **Step 2: Move.** Cut `app/studio.tsx` lines 145-638 (state, refs, derived values, callbacks, effects, `useStudioTools`) into `useStudio(navigate)` in `app/state/use-studio.ts`, replacing every `nav(` inside with `navigate(`, and return the object above. Keep the drawer/mobile effects (292-341) and the `view`/`chapter`/`mobileNav`/`mobileViewport` state in `studio.tsx`; they are layout. Move `formText`, `fmt`, `clock` to `app/state/format.ts` and import them where used.
- [ ] **Step 3: Wire.** In `Studio`, `const m = useStudio(nav);` and prefix every former local with `m.` (the compiler lists each). `nav` is defined before `useStudio` and uses only layout state.
- [ ] **Step 4: Verify.** `bun run check && bun run test:native`, rebuild the preview, rerun `bun run test:browser`. Expected: all green, identical to Step 1.
- [ ] **Step 5: Commit.** `refactor(specimen-studio): move specimen state and handlers into useStudio`

### Task 5: Panels and dialogs as modules

**Files:** Create `app/panels/{studio,nodes,memory,activity,specification,settings,lab}-panel.tsx`, `app/panels/topology.tsx` (Topology, NodeInspector, Trace, VisualOutput), `app/panels/common.tsx` (SearchField, Empty), `app/dialogs/{node-form,resource-form,attachment-form,dialog-host}.tsx`. Modify `app/studio.tsx`, `tests/ui-contract.test.js`, `package.json`.

**Interfaces:**
- Each panel: `export function <Name>Panel({ m, navigate }: { m: StudioModel; navigate: (v: View, chapter?: number) => void }): JSX.Element` (Specification also takes `chapter`). Local UI state (filters, tabs, virtual scroll) moves with its view unchanged.
- `DialogHost({ m })` renders today's `studio.tsx:1506-1639` switch.

- [ ] **Step 1: Repoint the contract test first** so it reads all app source. Replace the top of `tests/ui-contract.test.js`:

```js
import { readFile, readdir } from 'node:fs/promises';
import { test, expect } from 'bun:test';

const appDir = new URL('../app/', import.meta.url);
const files = (await readdir(appDir, { recursive: true })).filter((f) =>
  f.endsWith('.tsx'),
);
const studio = (
  await Promise.all(files.map((f) => readFile(new URL(f, appDir), 'utf8')))
).join('\n');
```

and add:

```js
test('studio panel keeps its contract strings in the studio panel module', async () => {
  const panel = await readFile(
    new URL('../app/panels/studio-panel.tsx', import.meta.url),
    'utf8',
  );
  expect(panel).toContain('className="console-telemetry"');
  expect(panel).toContain('aria-label="Specimen workflow"');
});
```

Run `bun test tests/ui-contract.test.js`. Expected: the new test FAILs (file missing); the old ones pass.

- [ ] **Step 2: Move.** Studio view JSX (`studio.tsx:830-1375`) into `StudioPanel`; each local view function (`NodesView` 1987, `MemoryView` 2229, `ActivityView` 2428, `Specification` 2527, `SettingsView` 2618) into its panel module with its props adapted to `m`; `Topology` 1644, `NodeInspector` 1785, `Trace` 1875, `VisualOutput` 1928 into `topology.tsx`; `SearchField` 1955 and `Empty` 2868 into `common.tsx`; the three forms (2886, 3043, 3158) into `app/dialogs/`. Carry each `oxlint-disable` comment with the code it covers (lines 2, 1696, 1700, 1888).
- [ ] **Step 3: Widen lint.** `package.json`: `"lint:studio": "oxlint app lib/specimen lib/webmcp.ts"`.
- [ ] **Step 4: Verify.** `bun run check && bun run test:native`; rebuild preview; `bun run test:browser`. Expected: all green.
- [ ] **Step 5: Commit.** `refactor(specimen-studio): one module per panel and dialog`

### Task 6: Shell (sidebar collapse, top bar, split pane, dock, palette, shortcuts)

**Files:** Create `app/shell/{shortcuts,commands}.ts`, `app/shell/{app-sidebar,top-bar,pane-layout,console-dock,command-palette}.tsx`, `tests/shell.test.js`. Rewrite `app/studio.tsx`. Modify `app/studio.css` (shell rules, tokens only).

**Interfaces:**

```ts
// shortcuts.ts
export type ShortcutAction = 'palette' | 'sidebar' | 'dock' | 'split' | 'close';
export function matchShortcut(e: { key: string; metaKey: boolean; ctrlKey: boolean; altKey: boolean; target: EventTarget | null }): ShortcutAction | null;
// commands.ts
export interface ShellCommand { id: string; label: string; group: 'Go to' | 'Open to the side' | 'Specimen' | 'View'; shortcut?: string; run: () => void }
export function buildCommands(ctx: {
  view: View; split?: View;
  go: (v: View) => void; openSide: (v: View) => void; closeSide: () => void;
  save: () => void; load: () => void; reset: () => void;
  toggleTheme: () => void; toggleSidebar: () => void; toggleDock: () => void;
}): ShellCommand[];
```

Rules: ⌘/Ctrl-K → `palette` always (even in inputs); ⌘/Ctrl-B → `sidebar`; ⌘/Ctrl-\ → `split`; backtick (no modifiers) → `dock`; Escape → `close`. Everything except `palette` and `close` returns `null` when `target` is an `input`, `textarea`, `select` or `[contenteditable]`.

- [ ] **Step 1: Write the failing tests** `tests/shell.test.js`

```js
import { test, expect } from 'bun:test';
import { matchShortcut } from '../app/shell/shortcuts';
import { buildCommands } from '../app/shell/commands';

const key = (k, mods = {}, tag = 'BODY') => ({
  key: k, metaKey: false, ctrlKey: false, altKey: false,
  target: { tagName: tag, isContentEditable: false }, ...mods,
});

test('palette opens from anywhere, including inputs', () => {
  expect(matchShortcut(key('k', { metaKey: true }))).toBe('palette');
  expect(matchShortcut(key('k', { ctrlKey: true }, 'INPUT'))).toBe('palette');
});

test('layout shortcuts never fire while typing', () => {
  expect(matchShortcut(key('`'))).toBe('dock');
  expect(matchShortcut(key('`', {}, 'INPUT'))).toBeNull();
  expect(matchShortcut(key('`', {}, 'TEXTAREA'))).toBeNull();
  expect(matchShortcut(key('b', { metaKey: true }))).toBe('sidebar');
  expect(matchShortcut(key('b', { metaKey: true }, 'INPUT'))).toBeNull();
  expect(matchShortcut(key('\\', { ctrlKey: true }))).toBe('split');
  expect(matchShortcut(key('a'))).toBeNull();
});

const noop = () => {};
const ctx = (over = {}) => ({
  view: 'studio', go: noop, openSide: noop, closeSide: noop, save: noop,
  load: noop, reset: noop, toggleTheme: noop, toggleSidebar: noop,
  toggleDock: noop, ...over,
});

test('every view is reachable and side targets exclude the current view and settings', () => {
  const cmds = buildCommands(ctx());
  const go = cmds.filter((c) => c.group === 'Go to').map((c) => c.id);
  expect(go).toEqual(['go:studio', 'go:nodes', 'go:memory', 'go:activity',
    'go:lab', 'go:specification', 'go:settings']);
  const side = cmds.filter((c) => c.group === 'Open to the side').map((c) => c.id);
  expect(side).not.toContain('side:studio');
  expect(side).not.toContain('side:settings');
  expect(side).toContain('side:nodes');
});

test('close side pane appears only with a split, and shortcuts are unique', () => {
  expect(buildCommands(ctx()).some((c) => c.id === 'close-side')).toBe(false);
  const cmds = buildCommands(ctx({ split: 'nodes' }));
  expect(cmds.some((c) => c.id === 'close-side')).toBe(true);
  const keys = cmds.map((c) => c.shortcut).filter(Boolean);
  expect(new Set(keys).size).toBe(keys.length);
});

test('no command label collides with a pinned e2e button name', () => {
  const pinned = /network|cancel|cycle trace|run dossier|add pattern|open navigation|dismiss error/i;
  for (const c of buildCommands(ctx({ split: 'nodes' }))) {
    expect(c.label).not.toMatch(pinned);
  }
});
```

- [ ] **Step 2: Run and see it fail.** `bun test tests/shell.test.js`. Expected: FAIL, modules missing.

- [ ] **Step 3: Implement the pure modules.**

```ts
// app/shell/shortcuts.ts
export type ShortcutAction = 'palette' | 'sidebar' | 'dock' | 'split' | 'close';

function editing(target: EventTarget | null): boolean {
  const el = target as { tagName?: string; isContentEditable?: boolean } | null;
  return (
    !!el &&
    (el.isContentEditable === true ||
      ['INPUT', 'TEXTAREA', 'SELECT'].includes(el.tagName ?? ''))
  );
}

export function matchShortcut(e: {
  key: string;
  metaKey: boolean;
  ctrlKey: boolean;
  altKey: boolean;
  target: EventTarget | null;
}): ShortcutAction | null {
  const mod = e.metaKey || e.ctrlKey;
  if (mod && !e.altKey && e.key.toLowerCase() === 'k') return 'palette';
  if (e.key === 'Escape') return 'close';
  if (editing(e.target)) return null;
  if (mod && !e.altKey && e.key.toLowerCase() === 'b') return 'sidebar';
  if (mod && !e.altKey && e.key === '\\') return 'split';
  if (!mod && !e.altKey && e.key === '`') return 'dock';
  return null;
}
```

```ts
// app/shell/commands.ts
import { VIEWS, type View } from '../state/navigation';

export interface ShellCommand {
  id: string;
  label: string;
  group: 'Go to' | 'Open to the side' | 'Specimen' | 'View';
  shortcut?: string;
  run: () => void;
}

const TITLES: Record<View, string> = {
  studio: 'Studio',
  nodes: 'Node library',
  memory: 'Memory',
  activity: 'Activity',
  lab: 'Vision & models',
  specification: 'Specification',
  settings: 'Settings',
};

export function buildCommands(ctx: {
  view: View;
  split?: View;
  go: (v: View) => void;
  openSide: (v: View) => void;
  closeSide: () => void;
  save: () => void;
  load: () => void;
  reset: () => void;
  toggleTheme: () => void;
  toggleSidebar: () => void;
  toggleDock: () => void;
}): ShellCommand[] {
  const cmds: ShellCommand[] = VIEWS.map((v) => ({
    id: `go:${v}`,
    label: `Go to ${TITLES[v]}`,
    group: 'Go to',
    run: () => ctx.go(v),
  }));
  for (const v of VIEWS) {
    if (v === ctx.view || v === 'settings') continue;
    cmds.push({
      id: `side:${v}`,
      label: `${TITLES[v]} beside ${TITLES[ctx.view]}`,
      group: 'Open to the side',
      run: () => ctx.openSide(v),
    });
  }
  if (ctx.split) {
    cmds.push({
      id: 'close-side',
      label: 'Close side pane',
      group: 'View',
      run: ctx.closeSide,
    });
  }
  cmds.push(
    { id: 'save', label: 'Save specimen file', group: 'Specimen', run: ctx.save },
    { id: 'load', label: 'Load specimen file', group: 'Specimen', run: ctx.load },
    { id: 'reset', label: 'Reset to starter specimen', group: 'Specimen', run: ctx.reset },
    { id: 'theme', label: 'Toggle light and dark theme', group: 'View', run: ctx.toggleTheme },
    { id: 'sidebar', label: 'Toggle sidebar', group: 'View', shortcut: '⌘B', run: ctx.toggleSidebar },
    { id: 'dock', label: 'Toggle activity dock', group: 'View', shortcut: '`', run: ctx.toggleDock },
  );
  return cmds;
}
```

Palette labels avoid `Load specimen`/`Save specimen` exact names (those are pinned header buttons); "Save specimen file" is a different exact name, and the palette's items are not rendered while it is closed.

- [ ] **Step 4: Run.** `bun test tests/shell.test.js`. Expected: 5 pass.

- [ ] **Step 5: Build the shell components.**
  - `app-sidebar.tsx`: today's `<aside>` (`studio.tsx:644-731`, with drawer backdrop) as a component taking `view`, `nav`, `mobileNav`, `setMobileNav`, `collapsed`, `sidebarRef`, `mobileCloseRef`, counts, `theme`, `toggleTheme`. Add a `collapsed` class on desktop that hides text labels (icons stay, each link gets `title` equal to its label) and a theme toggle button (`aria-label` "Switch to light theme"/"Switch to dark theme") beside the settings link. Keep `id="workspace-navigation"`, `nav[aria-label="Main navigation"]`, the one `a[href="?view=settings"]`, `inert`/`aria-hidden` logic.
  - `top-bar.tsx`: today's header (`studio.tsx:737-809`) plus a trigger button `aria-label="Search or run a command"` showing `⌘K`.
  - `pane-layout.tsx`: when `split` is set and `window.matchMedia('(min-width: 1024px)')` matches, `ResizablePanelGroup` (horizontal) with the primary panel, a `ResizableHandle withHandle`, and a secondary `section` (`aria-label` "Side pane: <title>", `h2` title, a close button `aria-label` "Close side pane"). Otherwise the primary panel only.
  - `console-dock.tsx`: collapsed by default; when open, a `section aria-label="Activity dock"` listing the latest 50 `m.state.events` (`title`, `detail`, time via `clock`) in a `role="log"` list.
  - `command-palette.tsx`: `CommandDialog` from `components/ui/command` with `CommandInput placeholder="Type a command or view"`, grouped `CommandItem`s from `buildCommands`, `CommandShortcut` where set.
  - `app/studio.tsx`: `useStudio(nav)`, route state from `parseRoute(location.search)` (replacing the local restore), `nav` writes `routeSearch`, and a single `keydown` listener dispatching `matchShortcut`. `split` collapses when the viewport narrows below 1024 px and returns when it widens (it stays in the URL).
  - `studio.css`: shell rules (collapsed sidebar width 56 px, top-bar trigger, split handle, dock) using tokens only; touch targets ≥ 44 px on every new button.

- [ ] **Step 6: Verify.** `bun run check && bun run test:native`; rebuild the preview; `bun run test:browser`; then open `?view=studio&split=nodes` at 1440 px in the browser pane and check both panes render and resize.
- [ ] **Step 7: Commit.** `feat(specimen-studio): hybrid shell with split pane, dock and command palette`

### Task 7: Themed e2e, split audit, docs

**Files:** Modify `tests/browser.e2e.mjs`, `tests/accessibility.e2e.mjs`, `README.md`, `RUNTIME-PROFILE.md`, `AGENTS.md`, `VALIDATION.md`.

- [ ] **Step 1: Theme input.** In both e2e scripts add, after the browser context is created:

```js
const theme = process.env.STUDIO_THEME;
if (theme) {
  await context.addInitScript((t) => {
    try {
      localStorage.setItem('wdbx-studio-theme', t);
    } catch {}
  }, theme);
}
const suffix = theme ? `-${theme}` : '';
```

and append `suffix` to every `work/…` output file name (JSON receipts and screenshots).

- [ ] **Step 2: Split audit.** In `accessibility.e2e.mjs`, inside the width loop only when `width === 1440`, after the specification audit: `await page.goto(`${base}?view=studio&split=nodes`)`, expect `getByRole('region', { name: 'Side pane: Node library' })` visible, then `await audit(page, width, 'split-studio-nodes')`. Add `'split-studio-nodes'` to the 1440 expectations in `scripts/qualification-summary.py` (line ~236-265) and its fixture in `tests/qualification-summary.test.js` together, or, if the collector rejects unknown states, record it outside `audits`; read the collector first and choose the smaller change.
- [ ] **Step 3: Run both themes.** `STUDIO_URL=http://127.0.0.1:4176 bun run test:browser` and `STUDIO_THEME=light STUDIO_URL=http://127.0.0.1:4176 bun run test:browser`. Expected: both pass; axe reports no serious or critical violations. Fix contrast failures by adjusting token values in `app/theme.css`, never by disabling rules.
- [ ] **Step 4: Docs.** README "Use the studio": sidebar collapse (⌘B), command palette (⌘K), side pane (⌘\\ or palette), activity dock (backtick), theme toggle; dark default. RUNTIME-PROFILE: interface paragraph. AGENTS.md *Project Structure*: `app/state`, `app/shell`, `app/panels`, `app/dialogs`, `app/theme.css`, the no-literal rule. VALIDATION.md: a dated section with the commands and results of Step 3 and the gates.
- [ ] **Step 5: Gates.** `bun run check`, `bun run test:native`, `bun run check:native`, `bun run test:browser` (both themes). Expected: all exit 0.
- [ ] **Step 6: Commit.** `test(specimen-studio): themed e2e runs and split-pane audit`
