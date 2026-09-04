/// <reference types="vite/client" />
import '@fontsource/geist/400.css';
import '@fontsource/geist/500.css';
import '@fontsource/geist/600.css';
import '@fontsource/geist-mono/400.css';
import { createRoot } from 'react-dom/client';
import Studio from '../app/studio';
import '../app/studio.css';
async function mount() {
  if (import.meta.env.VITE_NATIVE_E2E === '1') await import('@wdio/tauri-plugin');
  createRoot(document.getElementById('root')!).render(<Studio />);
}
void mount();
