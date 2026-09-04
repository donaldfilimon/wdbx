import { isDesktop, loadNative, persistNative } from './native';
import { validateSpecimen } from './engine';
import type { Specimen } from './types';
const database = 'wdbx-specimen-studio';
function open(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    const req = indexedDB.open(database, 1);
    req.onupgradeneeded = () => req.result.createObjectStore('workspace');
    req.onsuccess = () => resolve(req.result);
    req.onerror = () =>
      reject(
        new Error(
          'Device storage is unavailable. Use Save specimen to keep your work.',
        ),
      );
  });
}
export async function loadWorkspace(): Promise<Specimen | null> {
  if (isDesktop()) return loadNative();
  const db = await open();
  return new Promise((resolve, reject) => {
    const tx = db.transaction('workspace', 'readonly'),
      req = tx.objectStore('workspace').get('active');
    req.onsuccess = () => {
      try {
        resolve(req.result ? validateSpecimen(req.result) : null);
      } catch (e) {
        reject(e);
      } finally {
        db.close();
      }
    };
    req.onerror = () => {
      db.close();
      reject(req.error);
    };
  });
}
export async function persistWorkspace(state: Specimen): Promise<void> {
  if (isDesktop()) return persistNative(state);
  const db = await open();
  return new Promise((resolve, reject) => {
    const tx = db.transaction('workspace', 'readwrite');
    tx.objectStore('workspace').put(state, 'active');
    tx.oncomplete = () => {
      db.close();
      resolve();
    };
    tx.onerror = () => {
      db.close();
      reject(
        new Error(
          'Your changes could not be stored on this device. Download a specimen file to preserve them.',
        ),
      );
    };
  });
}
export function downloadText(
  filename: string,
  text: string,
  type = 'application/json',
): void {
  const blob = new Blob([text], { type });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = filename;
  a.click();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}
