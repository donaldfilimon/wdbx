import { useEffect, useState } from 'react';
import { RefreshCw, ShieldCheck, ShieldAlert } from 'lucide-react';
import { Badge } from '@/components/ui/badge';
import {
  Table,
  TableBody,
  TableCaption,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table';
import {
  CodeBlock,
  EmptyState,
  Panel,
  Stat,
  StatGroup,
  Toolbar,
  ToolbarSpacer,
  WButton,
} from '@/components/wdbx';
import {
  browserStoreInfo,
  desktopStoreInfo,
  formatBytes,
  requestPersistentStorage,
  type BrowserStoreReport,
  type DesktopStoreInfo,
} from '@/lib/specimen/store-info';

const short = (id: string) => `${id.slice(0, 8)}…`;
const skippedText = (n: number) =>
  `${n} ${n === 1 ? 'entry' : 'entries'} could not be read while measuring and ${n === 1 ? 'is' : 'are'} not counted.`;
const when = (iso: unknown) =>
  typeof iso === 'string' && !Number.isNaN(Date.parse(iso))
    ? new Date(iso).toLocaleString()
    : '—';

function RecordsTable({ records }: { records: Record<string, number> }) {
  return (
    <Table>
      <TableCaption className="sr-only">
        Stored records by collection
      </TableCaption>
      <TableHeader>
        <TableRow>
          <TableHead scope="col">Collection</TableHead>
          <TableHead scope="col" className="text-right">
            Records
          </TableHead>
        </TableRow>
      </TableHeader>
      <TableBody>
        {Object.entries(records).map(([name, count]) => (
          <TableRow key={name}>
            <TableCell className="font-mono text-xs">{name}</TableCell>
            <TableCell className="text-right tabular-nums">{count}</TableCell>
          </TableRow>
        ))}
      </TableBody>
    </Table>
  );
}

/** The desktop WDBX v2 journal: frontier, verification, key, records, disk. */
export function DesktopStoreView({ info }: { info: DesktopStoreInfo }) {
  const key = info.snapshotKey;
  return (
    <div className="grid gap-4">
      <StatGroup label="Store summary">
        <Stat label="Revision" value={info.revision} />
        <Stat label="Transactions" value={info.committedTransactions} />
        <Stat label="Writer sessions" value={Object.keys(info.heads).length} />
        <Stat label="Keys" value={info.kvCount} />
        <Stat
          label="On disk"
          value={formatBytes(info.disk.store.bytes + info.disk.assets.bytes)}
        />
      </StatGroup>
      <div className="grid gap-4 lg:grid-cols-2">
        <Panel
          title="Snapshot record"
          description="The studio keeps its workspace as one versioned key."
        >
          {key ? (
            <dl className="m-0 grid grid-cols-[auto_1fr] gap-x-4 gap-y-1.5 p-4 text-sm">
              <dt className="text-muted-foreground">Key</dt>
              <dd className="m-0 font-mono text-xs">{key.key}</dd>
              <dt className="text-muted-foreground">Version</dt>
              <dd className="m-0 font-mono text-xs break-all">
                {key.versionId}
              </dd>
              <dt className="text-muted-foreground">Writer</dt>
              <dd className="m-0 font-mono text-xs">
                {short(key.writerId)} · sequence {key.sequence}
              </dd>
              <dt className="text-muted-foreground">Size</dt>
              <dd className="m-0">{formatBytes(key.bytes)}</dd>
              <dt className="text-muted-foreground">Conflicts</dt>
              <dd className="m-0">
                {key.conflicts ? (
                  <Badge variant="destructive">
                    {key.conflicts} unresolved concurrent version
                    {key.conflicts === 1 ? '' : 's'}
                  </Badge>
                ) : (
                  <Badge variant="outline">None</Badge>
                )}
              </dd>
            </dl>
          ) : (
            <EmptyState
              title="Nothing saved yet"
              text="The first edit writes the workspace to the journal."
            />
          )}
        </Panel>
        <Panel
          title="Audit DAG"
          description={`${info.auditCount} blocks, ${info.auditHeads.length} heads`}
        >
          <div className="grid gap-2 p-4 text-sm">
            {info.auditDag.ok ? (
              <p className="m-0 flex items-center gap-2 text-ink">
                <ShieldCheck
                  aria-hidden="true"
                  size={16}
                  className="text-teal"
                />
                {info.auditCount
                  ? 'Verified: every parent exists and the edges form a DAG. Block hashes are not recomputed here.'
                  : 'Empty: the studio writes no audit blocks, so there is nothing to verify.'}
              </p>
            ) : (
              <p
                role="alert"
                className="m-0 flex items-center gap-2 text-danger"
              >
                <ShieldAlert aria-hidden="true" size={16} />
                Verification failed: {info.auditDag.error}
              </p>
            )}
          </div>
        </Panel>
        <Panel title="Records" description={`Schema ${info.schema}`}>
          <RecordsTable records={info.records} />
        </Panel>
        <Panel
          title="Causal frontier"
          description="Committed sequence per writer journal; the desktop app starts a new writer session each launch"
        >
          <Table>
            <TableCaption className="sr-only">Writer heads</TableCaption>
            <TableHeader>
              <TableRow>
                <TableHead scope="col">Writer</TableHead>
                <TableHead scope="col" className="text-right">
                  Sequence
                </TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {Object.entries(info.heads).map(([writer, sequence]) => (
                <TableRow key={writer}>
                  <TableCell className="font-mono text-xs">
                    {short(writer)}{' '}
                    {writer === info.writerId && (
                      <Badge variant="outline">This session</Badge>
                    )}
                  </TableCell>
                  <TableCell className="text-right tabular-nums">
                    {sequence}
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </Panel>
        <Panel
          title="Deleted nodes"
          description="Deletions are kept as tombstones, newest first"
        >
          {info.tombstones.length ? (
            <Table>
              <TableCaption className="sr-only">Tombstones</TableCaption>
              <TableHeader>
                <TableRow>
                  <TableHead scope="col">Node</TableHead>
                  <TableHead scope="col">Deleted</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {info.tombstones.map((t, i) => (
                  <TableRow key={`${t.ref ?? ''}-${i}`}>
                    <TableCell>{t.name ?? t.ref ?? '—'}</TableCell>
                    <TableCell className="text-xs">
                      {when(t.deletedAt)}
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          ) : (
            <EmptyState
              title="No deletions"
              text="Deleted nodes appear here."
            />
          )}
        </Panel>
        <Panel
          title="Disk"
          description="Measured sizes of the journal and asset folders"
        >
          <div className="grid gap-3 p-4 text-sm">
            <p className="m-0">
              Journal and segments: {info.disk.store.files} files,{' '}
              {formatBytes(info.disk.store.bytes)}
            </p>
            <p className="m-0">
              Assets: {info.disk.assets.files} files,{' '}
              {formatBytes(info.disk.assets.bytes)}
            </p>
            {info.disk.store.skipped + info.disk.assets.skipped > 0 && (
              <p className="m-0 text-xs text-muted-foreground">
                {skippedText(
                  info.disk.store.skipped + info.disk.assets.skipped,
                )}
              </p>
            )}
            <CodeBlock label="Store location">{info.root}</CodeBlock>
          </div>
        </Panel>
      </div>
    </div>
  );
}

/** The browser edition: one IndexedDB record and the origin's quota. */
export function BrowserStoreView({
  report,
  onPersist,
}: {
  report: BrowserStoreReport;
  onPersist?: () => void;
}) {
  return (
    <div className="grid gap-4">
      <StatGroup label="Browser storage summary">
        <Stat label="Saved workspace" value={formatBytes(report.bytes)} />
        <Stat label="Last saved" value={when(report.updatedAt)} />
        <Stat label="Origin usage" value={formatBytes(report.usage)} />
        <Stat label="Quota" value={formatBytes(report.quota)} />
      </StatGroup>
      <div className="grid gap-4 lg:grid-cols-2">
        <Panel
          title="Persistence"
          description="Browsers may clear non-persistent storage under pressure."
        >
          <div className="grid gap-3 p-4 text-sm">
            <p className="m-0">
              {report.persisted === null
                ? 'This browser does not report whether storage is persistent.'
                : report.persisted
                  ? 'Storage is persistent: the browser will not clear it on its own.'
                  : 'Storage is best-effort: the browser may clear it when space runs low.'}
            </p>
            {report.persisted === false && onPersist && (
              <WButton variant="outline" onClick={onPersist}>
                Keep storage persistent
              </WButton>
            )}
            <p className="m-0 text-xs text-muted-foreground">
              The browser keeps only the latest save, with no version history.
              Download a specimen file to keep versions, or use the desktop app
              for the WDBX journal.
            </p>
          </div>
        </Panel>
        <Panel
          title="Records"
          description="IndexedDB wdbx-specimen-studio / workspace"
        >
          {report.records ? (
            <RecordsTable records={report.records} />
          ) : (
            <EmptyState
              title="Nothing saved yet"
              text="Your first change is saved to this browser."
            />
          )}
        </Panel>
      </div>
    </div>
  );
}

type Loaded =
  | { kind: 'desktop'; info: DesktopStoreInfo }
  | { kind: 'browser'; report: BrowserStoreReport };

/** Store explorer: loads this edition's store report on open and on refresh. */
export function StorePanel({ desktop }: { desktop: boolean }) {
  const [loaded, setLoaded] = useState<Loaded | null>(null);
  const [error, setError] = useState('');
  const [message, setMessage] = useState('');
  // Bumping `reads` re-reads the store; results land in promise callbacks.
  const [reads, setReads] = useState(0);
  const reload = () => setReads((n) => n + 1);
  useEffect(() => {
    let live = true;
    const read: Promise<Loaded> = desktop
      ? desktopStoreInfo().then((info) => ({ kind: 'desktop', info }))
      : browserStoreInfo().then((report) => ({ kind: 'browser', report }));
    read.then(
      (result) => {
        if (!live) return;
        setLoaded(result);
        setError('');
      },
      (e: unknown) => {
        if (live) setError(e instanceof Error ? e.message : String(e));
      },
    );
    return () => {
      live = false;
    };
  }, [desktop, reads]);
  const persist = async () => {
    const granted = await requestPersistentStorage().catch(() => false);
    setMessage(
      granted
        ? 'The browser will keep this storage.'
        : 'The browser declined; keep a downloaded specimen file as a backup.',
    );
    reload();
  };
  return (
    <div className="grid gap-4">
      <Toolbar label="Store actions">
        <Badge variant="outline">
          {desktop ? 'Desktop · WDBX v2 journal' : 'Browser · IndexedDB'}
        </Badge>
        <ToolbarSpacer />
        <WButton variant="outline" onClick={reload}>
          <RefreshCw aria-hidden="true" size={14} />
          Refresh
        </WButton>
      </Toolbar>
      <output className="text-sm text-ink-soft">{message}</output>
      {error && (
        <p role="alert" className="m-0 text-sm text-danger">
          The store could not be read: {error}
        </p>
      )}
      {!loaded && !error && (
        <p className="m-0 text-sm text-muted-foreground">Reading the store…</p>
      )}
      {loaded?.kind === 'desktop' && <DesktopStoreView info={loaded.info} />}
      {loaded?.kind === 'browser' && (
        <BrowserStoreView
          report={loaded.report}
          onPersist={() => void persist()}
        />
      )}
    </div>
  );
}
