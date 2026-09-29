import {
  isDesktop,
  loadNative,
  loadNativeNetwork,
  persistNative,
  persistNativeNetwork,
} from './native';
import {
  kernelLoaded,
  networkDefault,
  networkValidate,
  validateSpecimen,
  type Network,
} from './kernel';
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
/**
 * The browser's stored record as saved, without validation or migration,
 * for the store explorer. Null when nothing is saved.
 */
export async function readStoredWorkspace(): Promise<Specimen | null> {
  const db = await open();
  return new Promise((resolve, reject) => {
    const req = db
      .transaction('workspace', 'readonly')
      .objectStore('workspace')
      .get('active');
    req.onsuccess = () => {
      db.close();
      resolve((req.result as Specimen | undefined) ?? null);
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
/**
 * The editable network: the desktop snapshot's, or the browser's own under
 * the `network` key. A missing or invalid browser network is the default.
 */
/** A network and, on desktop, the store revision it was read at. */
export interface StoredNetwork {
  network: Network;
  revision?: number;
}

export async function loadNetwork(): Promise<StoredNetwork> {
  if (isDesktop()) return (await loadNativeNetwork()) as StoredNetwork;
  // Validation and the default both come from the kernel.
  await kernelLoaded;
  const db = await open();
  const stored = await new Promise<unknown>((resolve, reject) => {
    const req = db
      .transaction('workspace', 'readonly')
      .objectStore('workspace')
      .get('network');
    req.onsuccess = () => {
      db.close();
      resolve(req.result);
    };
    req.onerror = () => {
      db.close();
      reject(req.error);
    };
  });
  if (!stored) return { network: networkDefault() };
  try {
    networkValidate(stored as Network);
    return { network: stored as Network };
  } catch {
    return { network: networkDefault() };
  }
}

export async function persistNetwork(
  network: Network,
  revision?: number,
): Promise<StoredNetwork> {
  if (isDesktop())
    return (await persistNativeNetwork(
      network,
      revision ?? 0,
    )) as StoredNetwork;
  const db = await open();
  await new Promise<void>((resolve, reject) => {
    const tx = db.transaction('workspace', 'readwrite');
    tx.objectStore('workspace').put(network, 'network');
    tx.oncomplete = () => {
      db.close();
      resolve();
    };
    // A quota failure aborts the transaction without an error event.
    tx.onerror = tx.onabort = () => {
      db.close();
      reject(new Error('The network could not be stored on this device.'));
    };
  });
  return { network };
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
