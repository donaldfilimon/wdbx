import { useEffect, useState } from 'react';
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
    <details className="border-t border-line px-4 py-2 text-sm">
      <summary className="cursor-pointer text-muted-foreground">
        Show data
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
        className="wdbx-meter h-2 w-full"
      />
    </div>
  );
}

/** Live views of the kernel: the running trace, votes, history and ATP. */
export function EnginePanel({
  trace,
  busy,
  cycle,
  history,
  nodes,
  atp,
  threshold,
}: {
  trace: TraceStep[];
  busy: boolean;
  cycle: Cycle | undefined;
  history: Cycle[];
  nodes: Specimen['nodes'];
  atp: Specimen['atp'];
  threshold: number;
}) {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    const t = setInterval(() => setNow(Date.now()), 5000);
    return () => clearInterval(t);
  }, []);
  const phases = phaseTimeline(trace, busy);
  const votes = voteBreakdown(cycle, nodes);
  const series = cycleSeries(history, 50);
  const live = atpNow(atp, now);
  const matched = series.filter((p) => p.matched).length;
  const durations = series.map((p) => p.duration);
  return (
    <div className="grid gap-4">
      <StatGroup label="Engine summary">
        <Stat label="Cycles" value={history.length} />
        <Stat
          label="Matched (last 50)"
          value={series.length ? `${Math.round((matched / series.length) * 100)}%` : '—'}
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
          title={busy ? 'Current cycle' : 'Last cycle'}
          description={cycle && !busy ? cycle.input : undefined}
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
                      <Loader2
                        size={14}
                        className="motion-safe:animate-spin"
                      />
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
          description={`Confidence per contributing node; threshold ${threshold}`}
        >
          {votes.length ? (
            <>
              <ChartContainer config={voteChart} className="h-56 w-full p-2">
                <BarChart data={votes} layout="vertical" margin={{ left: 16 }}>
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
              <DataTable
                caption="Votes by node"
                head={['Node', 'Confidence', 'Strength']}
                rows={votes.map((v) => [v.node, v.confidence.toFixed(1), v.strength])}
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
              <ChartContainer config={historyChart} className="h-56 w-full p-2">
                <LineChart data={series}>
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
        <Panel
          title="ATP"
          description="Affective state as the kernel sees it now (decays over 180 s)"
        >
          <div className="grid gap-4 p-4">
            <Meter label="Valence" value={live.valence} min={-1} max={1} />
            <Meter label="Intensity" value={live.intensity} min={0} max={1} />
          </div>
        </Panel>
      </div>
    </div>
  );
}
