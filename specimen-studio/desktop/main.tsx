/// <reference types="vite/client" />
import '@fontsource/geist/400.css';
import '@fontsource/geist/500.css';
import '@fontsource/geist/600.css';
import '@fontsource/geist-mono/400.css';
import { createRoot } from 'react-dom/client';
import Studio from '../app/studio';
import { applyTheme, resolveTheme } from '../app/shell/theme';
import '../app/studio.css';
function safeStorage(): Storage | undefined {
  try {
    return window.localStorage;
  } catch {
    return undefined;
  }
}
// index.html ships class="dark" (the default); CSP forbids an inline boot script.
applyTheme(document.documentElement, resolveTheme(safeStorage()));
async function mount() {
  if (import.meta.env.VITE_NATIVE_E2E === '1') {
    await import('@wdio/tauri-plugin');
    const { callNative } = await import('../lib/specimen/native');
    (window as unknown as {__wdbxTestCall:typeof callNative}).__wdbxTestCall=callNative;
  }
  createRoot(document.getElementById('root')!).render(<Studio />);
}
void mount();
