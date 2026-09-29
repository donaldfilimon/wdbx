/**
 * What each edition's store holds, for the store explorer. Desktop reads the
 * WDBX v2 journal through the read-only `storeInfo` op; the browser reports
 * its single IndexedDB record and the origin's storage estimate.
 */
import { callNative } from './native';
import { readStoredWorkspace } from './storage';
import type { Specimen } from './types';

export interface DiskUsage {
  files: number;
  bytes: number;
}

/** Mirrors `specimen_core::persistence::StoreInfo`. */
export interface DesktopStoreInfo {
  root: string;
  writerId: string;
  heads: Record<string, number>;
  committedTransactions: number;
  kvCount: number;
  vectorCount: number;
  spatialCount: number;
  auditCount: number;
  auditHeads: string[];
  auditDag: { ok: boolean; error: string | null };
  snapshotKey: {
    key: string;
    versionId: string;
    writerId: string;
    sequence: number;
    conflicts: number;
    bytes: number;
  } | null;
  schema: string;
  revision: number;
  records: Record<string, number>;
  tombstones: { name?: string; ref?: string; deletedAt?: string }[];
  disk: { store: DiskUsage; assets: DiskUsage };
}

export const desktopStoreInfo = () =>
  callNative<DesktopStoreInfo>({ op: 'storeInfo' });

type Counted = Pick<
  Specimen,
  | 'nodes'
  | 'resources'
  | 'attachments'
  | 'history'
  | 'events'
  | 'mutations'
  | 'proposals'
>;

export function recordCounts(s: Counted): Record<string, number> {
  return {
    nodes: s.nodes.length,
    entries: s.nodes.reduce((n, node) => n + (node.entries?.length ?? 0), 0),
    resources: s.resources.length,
    attachments: s.attachments.length,
    history: s.history.length,
    events: s.events.length,
    mutations: s.mutations.length,
    proposals: s.proposals.length,
  };
}

export interface BrowserStoreReport {
  /** UTF-8 size of the stored record's JSON; 0 when nothing is saved. */
  bytes: number;
  updatedAt: string | null;
  records: Record<string, number> | null;
  usage: number | null;
  quota: number | null;
  /** Whether the origin's storage is persistent; null when unknown. */
  persisted: boolean | null;
}

export function browserStoreReport(
  record: (Counted & { updatedAt?: string }) | null,
  estimate: { usage?: number; quota?: number } | null,
  persisted: boolean | null,
): BrowserStoreReport {
  return {
    bytes: record ? new TextEncoder().encode(JSON.stringify(record)).length : 0,
    updatedAt: record?.updatedAt ?? null,
    records: record ? recordCounts(record) : null,
    usage: estimate?.usage ?? null,
    quota: estimate?.quota ?? null,
    persisted,
  };
}

/** Reads the browser's stored record and storage estimate. */
export async function browserStoreInfo(): Promise<BrowserStoreReport> {
  const storage =
    typeof navigator !== 'undefined' ? navigator.storage : undefined;
  const [record, estimate, persisted] = await Promise.all([
    readStoredWorkspace(),
    storage?.estimate ? storage.estimate().catch(() => null) : null,
    storage?.persisted ? storage.persisted().catch(() => null) : null,
  ]);
  return browserStoreReport(record, estimate, persisted);
}

/** Asks the browser to keep this origin's storage; resolves to the outcome. */
export async function requestPersistentStorage(): Promise<boolean> {
  return (await navigator.storage?.persist?.()) ?? false;
}

export function formatBytes(bytes: number | null | undefined): string {
  if (bytes === null || bytes === undefined || !Number.isFinite(bytes))
    return '—';
  if (bytes < 1024) return `${bytes} B`;
  const units = ['KiB', 'MiB', 'GiB', 'TiB'];
  let value = bytes / 1024;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit++;
  }
  return `${value.toFixed(1)} ${units[unit]}`;
}
