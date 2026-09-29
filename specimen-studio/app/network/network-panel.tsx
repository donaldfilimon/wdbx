import { lazy, useEffect, useId, useState } from 'react';
import { Minus, Play, Plus, Shuffle } from 'lucide-react';
import { Badge } from '@/components/ui/badge';
import { Input } from '@/components/ui/input';
import {
  Table,
  TableBody,
  TableCaption,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { EmptyState, Panel, WButton } from '@/components/wdbx';
import {
  networkEdit,
  networkTrace,
  type Network,
  type NetworkCommand,
} from '@/lib/specimen/kernel';
import {
  describeRewiring,
  layerRows,
  traceStats,
} from '@/lib/specimen/network-view';
import {
  loadNetwork,
  persistNetwork,
  type StoredNetwork,
} from '@/lib/specimen/storage';
import type { Specimen } from '@/lib/specimen/types';
import { Viewport } from './viewport';

const LayersScene = lazy(() =>
  import('./scene').then((m) => ({ default: m.LayersScene })),
);
const TopologyScene = lazy(() =>
  import('./scene').then((m) => ({ default: m.TopologyScene })),
);

const field =
  'grid gap-1 text-xs font-semibold text-ink [&_input]:min-h-11 [&_input]:font-mono';
const whole = (v: string) => {
  const n = Number(v);
  return v.trim() !== '' && Number.isInteger(n) && n >= 0 ? n : null;
};

/** Layer builder: the network as a table of editable layers plus a 3D view. */
export function LayersView({
  network,
  onCommand,
  busy,
}: {
  network: Network;
  onCommand: (command: NetworkCommand, done: string) => void;
  busy: boolean;
}) {
  const id = useId();
  const rows = layerRows(network);
  const [widths, setWidths] = useState<Record<number, string>>({});
  const [fans, setFans] = useState<Record<number, string>>({});
  const [add, setAdd] = useState({ at: '0', outputs: '48', fanIn: '8' });
  const [seed, setSeed] = useState('104729');
  const [prompt, setPrompt] = useState('red circle');
  const [trace, setTrace] = useState<number[][] | undefined>();
  const [traceError, setTraceError] = useState('');
  const stats = trace ? traceStats(trace) : [];
  const seedValue = whole(seed) ?? 1;
  return (
    <div className="grid gap-4">
      <Viewport
        label="Network layers in 3D: an input column of 128 features, then one column of neurons per layer, with sampled connections"
        tableHint="The Layers table lists every layer."
      >
        <LayersScene network={network} trace={trace} />
      </Viewport>
      <Panel
        title="Layers"
        description="128 input features in; the last layer is the 32-output composition decoder."
      >
        <Table>
          <TableCaption className="sr-only">Network layers</TableCaption>
          <TableHeader>
            <TableRow>
              <TableHead scope="col">Layer</TableHead>
              <TableHead scope="col" className="text-right">
                Inputs
              </TableHead>
              <TableHead scope="col">Neurons</TableHead>
              <TableHead scope="col">Inputs per neuron</TableHead>
              <TableHead scope="col" className="text-right">
                Weights
              </TableHead>
              <TableHead scope="col">
                <span className="sr-only">Actions</span>
              </TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {rows.map((r) => {
              const width = widths[r.index] ?? String(r.outputs);
              const fan = fans[r.index] ?? String(r.fanIn);
              return (
                <TableRow
                  key={`${r.index}-${r.inputs}-${r.outputs}-${r.weights}`}
                >
                  <TableCell>
                    {r.index + 1}{' '}
                    {r.decoder && <Badge variant="outline">Decoder</Badge>}
                  </TableCell>
                  <TableCell className="text-right tabular-nums">
                    {r.inputs}
                  </TableCell>
                  <TableCell>
                    <form
                      className="flex items-center gap-2"
                      onSubmit={(e) => {
                        e.preventDefault();
                        const outputs = whole(width);
                        if (outputs !== null)
                          onCommand(
                            {
                              command: 'resizeLayer',
                              index: r.index,
                              outputs,
                              seed: seedValue,
                            },
                            `Layer ${r.index + 1} now has ${outputs} neurons.`,
                          );
                      }}
                    >
                      <Input
                        aria-label={`Layer ${r.index + 1} neurons`}
                        type="number"
                        min={1}
                        max={1024}
                        value={width}
                        disabled={r.decoder || busy}
                        onChange={(e) =>
                          setWidths({ ...widths, [r.index]: e.target.value })
                        }
                        className="min-h-11 w-24 font-mono"
                      />
                      {!r.decoder && (
                        <WButton
                          type="submit"
                          variant="outline"
                          disabled={busy || whole(width) === null}
                        >
                          Resize
                        </WButton>
                      )}
                    </form>
                  </TableCell>
                  <TableCell>
                    <form
                      className="flex items-center gap-2"
                      onSubmit={(e) => {
                        e.preventDefault();
                        const fanIn = whole(fan);
                        if (fanIn !== null)
                          onCommand(
                            {
                              command: 'setConnectivity',
                              index: r.index,
                              fanIn,
                              seed: seedValue,
                            },
                            `Layer ${r.index + 1} rewired.`,
                          );
                      }}
                    >
                      <Input
                        aria-label={`Layer ${r.index + 1} inputs per neuron`}
                        type="number"
                        min={1}
                        max={r.inputs}
                        value={fan}
                        disabled={busy}
                        onChange={(e) =>
                          setFans({ ...fans, [r.index]: e.target.value })
                        }
                        className="min-h-11 w-24 font-mono"
                      />
                      <WButton
                        type="submit"
                        variant="outline"
                        disabled={busy || whole(fan) === null}
                      >
                        Rewire
                      </WButton>
                    </form>
                  </TableCell>
                  <TableCell className="text-right tabular-nums">
                    {r.weights.toLocaleString()}
                  </TableCell>
                  <TableCell>
                    {!r.decoder && (
                      <WButton
                        variant="ghost"
                        aria-label={`Remove layer ${r.index + 1}`}
                        disabled={busy}
                        onClick={() =>
                          onCommand(
                            { command: 'removeLayer', index: r.index },
                            `Layer ${r.index + 1} removed.`,
                          )
                        }
                      >
                        <Minus aria-hidden="true" size={15} />
                      </WButton>
                    )}
                  </TableCell>
                </TableRow>
              );
            })}
          </TableBody>
        </Table>
      </Panel>
      <div className="grid gap-4 lg:grid-cols-2">
        <Panel
          title="Add a hidden layer"
          description="Inserted before the chosen layer; at most 8 layers."
        >
          <form
            className="flex flex-wrap items-end gap-3 p-4"
            onSubmit={(e) => {
              e.preventDefault();
              const at = whole(add.at);
              const outputs = whole(add.outputs);
              const fanIn = whole(add.fanIn);
              if (at === null || outputs === null || fanIn === null) return;
              onCommand(
                { command: 'addLayer', at, outputs, fanIn, seed: seedValue },
                `Added a ${outputs}-neuron layer.`,
              );
            }}
          >
            <label className={field}>
              Before layer
              <select
                value={add.at}
                onChange={(e) => setAdd({ ...add, at: e.target.value })}
                className="min-h-11 rounded-lg border border-line bg-surface-2 px-2 font-mono"
              >
                {rows.map((r) => (
                  <option key={r.index} value={r.index}>
                    {r.index + 1}
                  </option>
                ))}
              </select>
            </label>
            <label className={field} htmlFor={`${id}-outputs`}>
              Neurons
              <Input
                id={`${id}-outputs`}
                type="number"
                min={1}
                max={1024}
                value={add.outputs}
                onChange={(e) => setAdd({ ...add, outputs: e.target.value })}
                className="w-24"
              />
            </label>
            <label className={field} htmlFor={`${id}-fan`}>
              Inputs per neuron
              <Input
                id={`${id}-fan`}
                type="number"
                min={1}
                value={add.fanIn}
                onChange={(e) => setAdd({ ...add, fanIn: e.target.value })}
                className="w-24"
              />
            </label>
            <WButton type="submit" variant="primary" disabled={busy}>
              <Plus aria-hidden="true" size={15} />
              Add layer
            </WButton>
          </form>
        </Panel>
        <Panel title="Weights" description="Resampling keeps every connection.">
          <form
            className="flex flex-wrap items-end gap-3 p-4"
            onSubmit={(e) => {
              e.preventDefault();
              onCommand(
                { command: 'initWeights', seed: seedValue },
                `Weights resampled with seed ${seedValue}.`,
              );
            }}
          >
            <label className={field} htmlFor={`${id}-seed`}>
              Seed
              <Input
                id={`${id}-seed`}
                type="number"
                min={1}
                value={seed}
                onChange={(e) => setSeed(e.target.value)}
                className="w-32"
              />
            </label>
            <WButton type="submit" variant="outline" disabled={busy}>
              <Shuffle aria-hidden="true" size={15} />
              Resample weights
            </WButton>
          </form>
        </Panel>
      </div>
      <Panel
        title="Trace"
        description="Runs a prompt through every layer on the CPU with a text-only encoding: no resources, ATP state, visual features or specimen seed, so values differ from a real cycle."
      >
        <form
          className="flex flex-wrap items-end gap-3 p-4"
          onSubmit={(e) => {
            e.preventDefault();
            try {
              setTrace(networkTrace(network, prompt, seedValue));
              setTraceError('');
            } catch (err) {
              setTraceError(err instanceof Error ? err.message : String(err));
            }
          }}
        >
          <label
            className={`${field} min-w-60 flex-1`}
            htmlFor={`${id}-prompt`}
          >
            Prompt
            <Input
              id={`${id}-prompt`}
              value={prompt}
              maxLength={8000}
              onChange={(e) => setPrompt(e.target.value)}
            />
          </label>
          <WButton type="submit" variant="primary">
            <Play aria-hidden="true" size={15} />
            Trace
          </WButton>
        </form>
        {traceError && (
          <p role="alert" className="m-0 px-4 pb-3 text-sm text-danger">
            {traceError}
          </p>
        )}
        {stats.length > 0 && (
          <Table>
            <TableCaption className="sr-only">
              Trace: output per layer
            </TableCaption>
            <TableHeader>
              <TableRow>
                <TableHead scope="col">Layer</TableHead>
                <TableHead scope="col" className="text-right">
                  Active
                </TableHead>
                <TableHead scope="col" className="text-right">
                  Min
                </TableHead>
                <TableHead scope="col" className="text-right">
                  Mean
                </TableHead>
                <TableHead scope="col" className="text-right">
                  Max
                </TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {stats.map((s) => (
                <TableRow key={s.layer}>
                  <TableCell>{s.layer + 1}</TableCell>
                  <TableCell className="text-right tabular-nums">
                    {s.active} / {trace![s.layer].length}
                  </TableCell>
                  <TableCell className="text-right font-mono tabular-nums">
                    {s.min.toFixed(3)}
                  </TableCell>
                  <TableCell className="text-right font-mono tabular-nums">
                    {s.mean.toFixed(3)}
                  </TableCell>
                  <TableCell className="text-right font-mono tabular-nums">
                    {s.max.toFixed(3)}
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        )}
      </Panel>
    </div>
  );
}

/** Specimen topology in 3D with paired node and attachment tables. */
export function TopologyView({ specimen }: { specimen: Specimen }) {
  const [selected, setSelected] = useState<string | undefined>();
  const byRef = new Map(specimen.nodes.map((n) => [n.ref, n]));
  const node = selected ? byRef.get(selected) : undefined;
  if (!specimen.nodes.length)
    return (
      <EmptyState
        title="No nodes"
        text="Add nodes in the Node Library to see the topology."
      />
    );
  return (
    <div className="grid gap-4">
      <Viewport
        label="Specimen topology in 3D: node shape shows type, size shows strength, colour shows tone; attachments are lines, solid when hard"
        tableHint="The Nodes and Attachments tables list the same data."
      >
        <TopologyScene
          nodes={specimen.nodes}
          attachments={specimen.attachments}
          maxStrength={specimen.settings.maxStrength}
          selected={selected}
          onSelect={setSelected}
        />
      </Viewport>
      {node && (
        <output className="rounded-lg border border-teal bg-surface-2 p-3 text-sm">
          <strong>{node.name}</strong>: type {node.type}, strength{' '}
          {node.strength}, tone {node.tone}, {node.entries.length} entr
          {node.entries.length === 1 ? 'y' : 'ies'}
        </output>
      )}
      <div className="grid gap-4 lg:grid-cols-2">
        <Panel title="Nodes" description="Select a node to highlight it in 3D.">
          <Table>
            <TableCaption className="sr-only">Nodes</TableCaption>
            <TableHeader>
              <TableRow>
                <TableHead scope="col">Node</TableHead>
                <TableHead scope="col">Type</TableHead>
                <TableHead scope="col" className="text-right">
                  Strength
                </TableHead>
                <TableHead scope="col">Tone</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {specimen.nodes.map((n) => (
                <TableRow
                  key={n.ref}
                  data-state={n.ref === selected ? 'selected' : undefined}
                >
                  <TableCell>
                    <WButton
                      variant="ghost"
                      aria-pressed={n.ref === selected}
                      onClick={() =>
                        setSelected(n.ref === selected ? undefined : n.ref)
                      }
                    >
                      {n.name}
                    </WButton>
                  </TableCell>
                  <TableCell>{n.type}</TableCell>
                  <TableCell className="text-right tabular-nums">
                    {n.strength}
                  </TableCell>
                  <TableCell>{n.tone}</TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </Panel>
        <Panel
          title="Attachments"
          description="Affinity sets how strongly a pair is wired."
        >
          {specimen.attachments.length ? (
            <Table>
              <TableCaption className="sr-only">Attachments</TableCaption>
              <TableHeader>
                <TableRow>
                  <TableHead scope="col">From</TableHead>
                  <TableHead scope="col">To</TableHead>
                  <TableHead scope="col" className="text-right">
                    Affinity
                  </TableHead>
                  <TableHead scope="col">Kind</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {specimen.attachments.map((a) => (
                  <TableRow key={a.id}>
                    <TableCell>{byRef.get(a.from)?.name ?? a.from}</TableCell>
                    <TableCell>{byRef.get(a.to)?.name ?? a.to}</TableCell>
                    <TableCell className="text-right tabular-nums">
                      {a.affinity}
                    </TableCell>
                    <TableCell>
                      {a.hard ? 'hard' : 'soft'}
                      {a.bidirectional ? ', both ways' : ''}
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          ) : (
            <EmptyState
              title="No attachments"
              text="Attach nodes in the Node Library."
            />
          )}
        </Panel>
      </div>
    </div>
  );
}

/** The Network view: layer builder and topology, both in 3D with tables. */
export function NetworkPanel({
  specimen,
  desktop,
}: {
  specimen: Specimen;
  desktop: boolean;
}) {
  const [stored, setStored] = useState<StoredNetwork | null>(null);
  // Bumped on every successful edit: remounts the builder so drafts and the
  // last trace never describe a network that no longer exists.
  const [version, setVersion] = useState(0);
  const [reads, setReads] = useState(0);
  const [error, setError] = useState('');
  const [status, setStatus] = useState('');
  const [busy, setBusy] = useState(false);
  const network = stored?.network ?? null;
  useEffect(() => {
    let live = true;
    loadNetwork().then(
      (loaded) => {
        if (!live) return;
        setStored(loaded);
        setVersion((v) => v + 1);
        setError('');
      },
      (e: unknown) =>
        live && setError(e instanceof Error ? e.message : String(e)),
    );
    return () => {
      live = false;
    };
  }, [reads]);
  const command = (cmd: NetworkCommand, done: string) => {
    if (!stored) return;
    let next: Network;
    try {
      next = networkEdit(stored.network, cmd);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
      return;
    }
    const targets =
      cmd.command === 'addLayer'
        ? [cmd.at]
        : cmd.command === 'removeLayer'
          ? [cmd.index - 1]
          : cmd.command === 'initWeights'
            ? []
            : [cmd.index];
    const sideEffect = targets.length
      ? describeRewiring(stored.network, next, targets)
      : '';
    setBusy(true);
    persistNetwork(next, stored.revision).then(
      (saved) => {
        setStored(saved);
        setVersion((v) => v + 1);
        setError('');
        setStatus(sideEffect ? `${done} ${sideEffect}` : done);
        setBusy(false);
      },
      (e: unknown) => {
        setError(e instanceof Error ? e.message : String(e));
        setBusy(false);
      },
    );
  };
  return (
    <div className="grid gap-4">
      <p className="m-0 text-sm text-ink-soft">
        {desktop
          ? 'Desktop: this is the network cycles use for composition, saved in the WDBX journal.'
          : 'Browser: this network is saved in this browser for designing and tracing. Browser cycles still compose with the built-in default network; the desktop app composes with its saved one.'}
      </p>
      <output className="text-sm text-ink-soft">{status}</output>
      {error && (
        <div
          role="alert"
          className="flex flex-wrap items-center gap-3 text-sm text-danger"
        >
          {error}
          <WButton variant="outline" onClick={() => setReads((n) => n + 1)}>
            Reload network
          </WButton>
        </div>
      )}
      <Tabs defaultValue="layers" className="flex-col">
        <TabsList className="h-auto">
          <TabsTrigger
            value="layers"
            className="min-h-11 px-4 text-ink-soft data-active:text-ink"
          >
            Layers
          </TabsTrigger>
          <TabsTrigger
            value="topology"
            className="min-h-11 px-4 text-ink-soft data-active:text-ink"
          >
            Topology
          </TabsTrigger>
        </TabsList>
        <TabsContent value="layers">
          {network ? (
            <LayersView
              key={version}
              network={network}
              onCommand={command}
              busy={busy}
            />
          ) : (
            !error && (
              <p className="m-0 text-sm text-muted-foreground">
                Loading the network…
              </p>
            )
          )}
        </TabsContent>
        <TabsContent value="topology">
          <TopologyView specimen={specimen} />
        </TabsContent>
      </Tabs>
    </div>
  );
}
