import { useSyncExternalStore } from 'react';
import {
  Bar,
  BarChart,
  CartesianGrid,
  Line,
  LineChart,
  ReferenceLine,
  XAxis,
  YAxis,
} from 'recharts';
import { Check, Loader2 } from 'lucide-react';
import {
  ChartContainer,
  ChartTooltip,
  ChartTooltipContent,
  type ChartConfig,
} from '@/components/ui/chart';
import { EmptyState, Panel, Stat, StatGroup } from '@/components/wdbx';
import {
  atpNow,
  cycleSeries,
  phaseTimeline,
  voteBreakdown,
} from '@/lib/specimen/insights';
import type { Cycle, Specimen, TraceStep } from '@/lib/specimen/types';
import { cn } from '@/lib/utils';

const voteChart = {
  confidence: { label: 'Confidence', color: 'var(--teal)' },
} satisfies ChartConfig;
const historyChart = {
  confidence: { label: 'Best confidence', color: 'var(--teal)' },
  duration: { label: 'Duration (ms)', color: 'var(--amber)' },
} satisfies ChartConfig;

const table =
  'w-full border-collapse text-left text-xs [&_td]:border-t [&_td]:border-line [&_td]:px-3 [&_td]:py-1.5 [&_th]:px-3 [&_th]:py-1.5 [&_th]:font-semibold [&_th]:text-muted-foreground';

/** Chart data as a table: the accessible equivalent of every chart. */
function DataTable({
  caption,
  head,
  rows,
}: {
  caption: string;
  head: string[];
  rows: (string | number)[][];
}) {
  return (
    <details className="border-t border-line px-4 py-1 text-sm">
      <summary className="min-h-11 cursor-pointer content-center text-muted-foreground">
        Show data: {caption}
      </summary>
      <div className="mt-2 max-h-56 overflow-auto">
        <table className={table}>
          <caption className="sr-only">{caption}</caption>
          <thead>
            <tr>
              {head.map((c) => (
                <th key={c} scope="col">
                  {c}
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {rows.map((r, i) => (
              <tr key={i}>
                {r.map((c, j) => (
                  <td key={j}>{c}</td>
                ))}
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </details>
  );
}

/**
 * A labelled value with a visual bar. Signed ranges (min < 0) fill from the
 * centre, so negative reads as negative. The native <meter> carries the value.
 */
function Meter({
  label,
  value,
  min,
  max,
}: {
  label: string;
  value: number;
  min: number;
  max: number;
}) {
  const at = (v: number) =>
    Math.max(0, Math.min(100, ((v - min) / (max - min)) * 100));
  const origin = min < 0 ? at(0) : 0;
  const pos = at(value);
  const left = Math.min(origin, pos);
  const width = Math.abs(pos - origin);
  return (
    <div className="grid gap-1.5">
      <div className="flex justify-between text-xs">
        <span className="font-semibold text-ink">{label}</span>
        <span className="font-mono text-muted-foreground tabular-nums">
          {value.toFixed(3)}
        </span>
      </div>
      <meter
        aria-label={label}
        min={min}
        max={max}
        value={value}
        className="sr-only"
      />
      <div
        aria-hidden="true"
        className="relative h-2 overflow-hidden rounded-full border border-line bg-surface-2"
      >
        {min < 0 && (
          <span
            className="absolute inset-y-0 w-px bg-line-strong"
            style={{ left: `${origin}%` }}
          />
        )}
        <span
          data-signed={min < 0}
          className="absolute inset-y-0 rounded-full bg-teal"
          style={{
            left: `${+left.toFixed(2)}%`,
            width: `${+width.toFixed(2)}%`,
          }}
        />
      </div>
    </div>
  );
}

/** A 5-second clock that pauses while the tab is hidden; null on the server. */
let clockNow = 0;
function subscribeClock(onChange: () => void) {
  const tick = () => {
    if (document.visibilityState !== 'visible') return;
    clockNow = Date.now();
    onChange();
  };
  // React re-reads the snapshot after subscribing, so a reopened panel
  // shows the current time rather than the last tick.
  clockNow = Date.now();
  const timer = setInterval(tick, 5000);
  document.addEventListener('visibilitychange', tick);
  return () => {
    clearInterval(timer);
    document.removeEventListener('visibilitychange', tick);
  };
}
const readClock = () => (clockNow ||= Date.now());
const serverClock = () => null;

/**
 * The ATP state: the stored values modulate the next cycle's votes; decay
 * over 180 s is applied after Compose. Owns the clock so ticks re-render
 * only this panel.
 */
function AtpPanel({ atp }: { atp: Specimen['atp'] }) {
  const now = useSyncExternalStore(subscribeClock, readClock, serverClock);
  const decayed = now === null ? undefined : atpNow(atp, now);
  return (
    <Panel
      title="ATP"
      description="Stored affective state; it modulates the next cycle's votes"
    >
      <div className="grid gap-4 p-4">
        <Meter label="Valence" value={atp.valence} min={-1} max={1} />
        <Meter label="Intensity" value={atp.intensity} min={0} max={1} />
        <p className="m-0 text-xs text-ink-soft">
          Decayed now (the base the next update starts from, 180 s time
          constant):{' '}
          <span className="font-mono tabular-nums">
            {decayed
              ? `valence ${decayed.valence.toFixed(3)}, intensity ${decayed.intensity.toFixed(3)}`
              : '…'}
          </span>
        </p>
      </div>
    </Panel>
  );
}

const OUTCOME_TITLE = {
  idle: 'Last cycle',
  complete: 'Last cycle',
  running: 'Current cycle',
  cancelled: 'Cancelled cycle',
  failed: 'Failed cycle',
} as const;

/** Live views of the kernel: the running trace, votes, history and ATP. */
export function EnginePanel({
  trace,
  cycle,
  history,
  nodes,
  atp,
  threshold,
  outcome,
  runInput,
}: {
  trace: TraceStep[];
  cycle: Cycle | undefined;
  history: Cycle[];
  nodes: Specimen['nodes'];
  atp: Specimen['atp'];
  threshold: number;
  /** The latest run's outcome; `trace` belongs to it unless it is idle. */
  outcome: keyof typeof OUTCOME_TITLE;
  runInput: string;
}) {
  // Review and maintenance also set `busy`; only a running cycle has a live phase.
  const phases = phaseTimeline(trace, outcome === 'running');
  // `cycle` is the last completed cycle; a running, cancelled or failed run
  // has a trace of its own but no votes yet.
  const ownRun = outcome === 'idle' || outcome === 'complete';
  const previous =
    cycle && !ownRun ? ` From the previous cycle: ${cycle.input}` : '';
  const votes = voteBreakdown(cycle, nodes);
  const series = cycleSeries(history, 50);
  const matched = series.filter((p) => p.matched).length;
  const durations = series.map((p) => p.duration);
  return (
    <div className="grid gap-4">
      <StatGroup label="Engine summary">
        <Stat label="Cycles" value={history.length} />
        <Stat
          label="Matched (last 50)"
          value={
            series.length
              ? `${Math.round((matched / series.length) * 100)}%`
              : '—'
          }
        />
        <Stat
          label="Last confidence"
          value={votes.length ? votes[0].confidence.toFixed(0) : '—'}
        />
        <Stat
          label="Mean duration"
          value={
            durations.length
              ? `${(durations.reduce((a, b) => a + b, 0) / durations.length).toFixed(1)} ms`
              : '—'
          }
        />
      </StatGroup>
      <div className="grid gap-4 lg:grid-cols-2">
        <Panel
          title={OUTCOME_TITLE[outcome]}
          description={ownRun ? cycle?.input : runInput}
        >
          {phases.length ? (
            <ol
              aria-label="Live cycle trace"
              className="m-0 grid list-none gap-2 p-4"
            >
              {phases.map((p) => (
                <li
                  key={p.phase}
                  aria-current={p.state === 'active' ? 'step' : undefined}
                  className={cn(
                    'grid grid-cols-[1.5rem_1fr] gap-2 rounded-lg border border-line bg-raised px-3 py-2',
                    p.state === 'active' && 'border-teal',
                  )}
                >
                  <span
                    aria-hidden="true"
                    className="grid size-6 place-items-center rounded-full bg-teal-soft text-teal"
                  >
                    {p.state === 'active' ? (
                      <Loader2 size={14} className="motion-safe:animate-spin" />
                    ) : (
                      <Check size={14} />
                    )}
                  </span>
                  <div className="min-w-0">
                    <div className="flex items-baseline justify-between gap-2">
                      <strong className="text-sm text-ink">{p.phase}</strong>
                      {p.count !== undefined && (
                        <span className="font-mono text-xs text-muted-foreground tabular-nums">
                          {p.count}
                        </span>
                      )}
                    </div>
                    <p className="m-0 text-xs break-words text-ink-soft">
                      {p.detail}
                    </p>
                    {p.coverage !== undefined && (
                      <progress
                        aria-label={`${p.phase} coverage`}
                        max={100}
                        value={Math.round(p.coverage * 100)}
                        className="wdbx-meter mt-1.5 h-1.5 w-full"
                      />
                    )}
                  </div>
                </li>
              ))}
            </ol>
          ) : (
            <EmptyState
              title="No cycle yet"
              text="Run a prompt in Studio and its trace appears here as it runs."
            />
          )}
        </Panel>
        <Panel
          title="Votes"
          description={`Confidence per contributing node; threshold ${threshold}.${previous}`}
        >
          {votes.length ? (
            <>
              {/* oxlint-disable-next-line jsx-a11y/prefer-tag-over-role -- An <img> cannot host the SVG chart; the wrapper names the chart as one image beside its data table. */}
              <div role="img" aria-label="Vote confidence by node">
                <ChartContainer config={voteChart} className="h-56 w-full p-2">
                  <BarChart
                    data={votes}
                    layout="vertical"
                    margin={{ left: 16 }}
                    accessibilityLayer={false}
                  >
                    <CartesianGrid horizontal={false} />
                    <XAxis type="number" domain={[-100, 100]} />
                    <YAxis type="category" dataKey="node" width={90} />
                    <ReferenceLine x={threshold} stroke="var(--amber)" />
                    <ChartTooltip content={<ChartTooltipContent />} />
                    <Bar
                      dataKey="confidence"
                      fill="var(--color-confidence)"
                      radius={4}
                      isAnimationActive={false}
                    />
                  </BarChart>
                </ChartContainer>
              </div>
              <DataTable
                caption="Votes by node"
                head={['Node', 'Confidence', 'Strength']}
                rows={votes.map((v) => [
                  v.node,
                  v.confidence.toFixed(1),
                  v.strength,
                ])}
              />
            </>
          ) : (
            <EmptyState
              title="No votes"
              text="The last cycle had no qualifying votes."
            />
          )}
        </Panel>
        <Panel
          title="Cycle history"
          description={`Last ${series.length} cycles`}
        >
          {series.length ? (
            <>
              <div
                // oxlint-disable-next-line jsx-a11y/prefer-tag-over-role -- An <img> cannot host the SVG chart; the wrapper names the chart as one image beside its data table.
                role="img"
                aria-label="Confidence and duration of recent cycles"
              >
                <ChartContainer
                  config={historyChart}
                  className="h-56 w-full p-2"
                >
                  <LineChart data={series} accessibilityLayer={false}>
                    <CartesianGrid vertical={false} />
                    <XAxis dataKey="id" tick={false} />
                    <YAxis yAxisId="c" domain={[-100, 100]} />
                    <YAxis yAxisId="d" orientation="right" />
                    <ChartTooltip content={<ChartTooltipContent />} />
                    <Line
                      yAxisId="c"
                      dataKey="confidence"
                      stroke="var(--color-confidence)"
                      dot={false}
                      connectNulls={false}
                      isAnimationActive={false}
                    />
                    <Line
                      yAxisId="d"
                      dataKey="duration"
                      stroke="var(--color-duration)"
                      dot={false}
                      isAnimationActive={false}
                    />
                  </LineChart>
                </ChartContainer>
              </div>
              <DataTable
                caption="Recent cycles"
                head={['Input', 'Status', 'Best confidence', 'Duration (ms)']}
                rows={series
                  .slice()
                  .reverse()
                  .map((p) => [
                    p.input,
                    p.status,
                    p.confidence === null ? '—' : p.confidence.toFixed(1),
                    p.duration.toFixed(1),
                  ])}
              />
            </>
          ) : (
            <EmptyState
              title="No history"
              text="Cycles you run are charted here."
            />
          )}
        </Panel>
        <AtpPanel atp={atp} />
      </div>
    </div>
  );
}
