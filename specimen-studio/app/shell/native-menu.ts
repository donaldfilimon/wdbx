const MENU_ACTIONS = [
  'save',
  'load',
  'reset',
  'palette',
  'dock',
  'sidebar',
  'theme',
] as const;

type MenuAction = (typeof MENU_ACTIONS)[number];

function isIdPayload(payload: unknown): payload is { id: unknown } {
  return !!payload && typeof payload === 'object' && 'id' in payload;
}

function menuAction(payload: unknown): MenuAction | undefined {
  const id =
    typeof payload === 'string'
      ? payload
      : isIdPayload(payload)
        ? payload.id
        : undefined;
  return MENU_ACTIONS.find((action) => action === id);
}

/** Desktop menu listener. No-ops outside Tauri, and if listen fails. */
export async function bindNativeMenu(handlers: {
  save: () => void;
  load: () => void;
  reset: () => void;
  palette: () => void;
  dock: () => void;
  sidebar: () => void;
  theme: () => void;
}): Promise<() => void> {
  const noop = () => {};
  try {
    if (typeof window === 'undefined' || !('__TAURI_INTERNALS__' in window))
      return noop;
    const { listen } = await import('@tauri-apps/api/event');
    return await listen<unknown>('native-menu', (event) => {
      const action = menuAction(event.payload);
      if (action) handlers[action]();
    });
  } catch {
    return noop;
  }
}
