import { VIEWS, type View } from '../state/navigation';

export interface ShellCommand {
  id: string;
  label: string;
  group: 'Go to' | 'Open to the side' | 'Specimen' | 'View';
  shortcut?: string;
  run: () => void;
}

export const VIEW_TITLES: Record<View, string> = {
  studio: 'Studio',
  nodes: 'Node library',
  memory: 'Memory',
  activity: 'Activity',
  engine: 'Live engine',
  store: 'Store explorer',
  lab: 'Vision & models',
  specification: 'Specification',
  settings: 'Settings',
};

/** Everything the command palette offers, in display order. */
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
    label: `Go to ${VIEW_TITLES[v]}`,
    group: 'Go to',
    run: () => ctx.go(v),
  }));
  for (const v of VIEWS) {
    if (v === ctx.view || v === 'settings') continue;
    cmds.push({
      id: `side:${v}`,
      label: `${VIEW_TITLES[v]} beside ${VIEW_TITLES[ctx.view]}`,
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
    {
      id: 'save',
      label: 'Save specimen file',
      group: 'Specimen',
      run: ctx.save,
    },
    {
      id: 'load',
      label: 'Load specimen file',
      group: 'Specimen',
      run: ctx.load,
    },
    {
      id: 'reset',
      label: 'Reset to starter specimen',
      group: 'Specimen',
      run: ctx.reset,
    },
    {
      id: 'theme',
      label: 'Toggle light and dark theme',
      group: 'View',
      run: ctx.toggleTheme,
    },
    {
      id: 'sidebar',
      label: 'Toggle sidebar',
      group: 'View',
      shortcut: '⌘B',
      run: ctx.toggleSidebar,
    },
    {
      id: 'dock',
      label: 'Toggle activity dock',
      group: 'View',
      shortcut: '`',
      run: ctx.toggleDock,
    },
  );
  return cmds;
}
