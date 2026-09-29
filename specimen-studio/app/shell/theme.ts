export type Theme = 'dark' | 'light';
export const THEME_KEY = 'wdbx-studio-theme';

/** Dark unless the viewer chose light; blocked storage also means dark. */
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

/** Runs before first paint so the stored theme never flashes. */
export const THEME_BOOT_SCRIPT = `(()=>{let t='dark';try{if(localStorage.getItem('${THEME_KEY}')==='light')t='light'}catch{}if(t==='dark')document.documentElement.classList.add('dark')})()`;
