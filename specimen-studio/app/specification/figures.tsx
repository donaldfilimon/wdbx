import { useMemo } from 'react';
import {
  parseEntityOutline,
  type SpecChapter,
  type SpecSegment,
} from '@/lib/specimen/spec-figures';
import type { SpecLive } from '../panels/specification-panel';
import { CodeFigure, SpecFigure } from './figure';
import { LifecycleDiagram } from './lifecycle-diagram';
import { MaintenanceDiagram } from './maintenance-diagram';
import { ModelDiagram } from './model-diagram';
import { PersistenceDiagram } from './persistence-diagram';
import { PipelineDiagram } from './pipeline-diagram';
import { RaftsDiagram } from './rafts-diagram';

const MODEL_FIGURES = ['ch5-1', 'ch5-4', 'ch5-5'];

type Figure = Extract<SpecSegment, { kind: 'figure' }>;

/** Entity name -> the ch5 figure that defines it, across all model figures. */
function modelIndex(chapters: SpecChapter[]) {
  const index = new Map<string, string>();
  for (const c of chapters)
    for (const s of c.segments)
      if (s.kind === 'figure' && MODEL_FIGURES.includes(s.id))
        for (const e of parseEntityOutline(s.code)) index.set(e.name, s.id);
  return index;
}

/** One specification figure: an interactive diagram where one exists. */
export function SpecFigureView({
  figure,
  chapters,
  onChapter,
  live,
}: {
  figure: Figure;
  chapters: SpecChapter[];
  onChapter: (n: number) => void;
  live: SpecLive;
}) {
  const known = useMemo(() => modelIndex(chapters), [chapters]);
  const { id, code } = figure;
  const s = live.specimen;
  switch (id) {
    case 'ch2-1':
      return (
        <SpecFigure
          id={id}
          title="Processing pipeline"
          caption="Each stage links to the chapter that specifies it. The voter phase ends at the phase boundary; the orchestrator then fans out to its inputs and back into composition."
          source={code}
        >
          <PipelineDiagram onChapter={onChapter} />
        </SpecFigure>
      );
    case 'ch5-1':
    case 'ch5-4':
    case 'ch5-5':
      return (
        <SpecFigure
          id={id}
          title={
            id === 'ch5-1'
              ? 'Specimen and node model'
              : id === 'ch5-4'
                ? 'Resources, sigils, attachments and votes'
                : 'Output, learning and mutation records'
          }
          caption="Field types that name another entity link to its card. Live counts come from the open specimen."
          source={code}
        >
          <ModelDiagram
            figure={id}
            code={code}
            known={known}
            counts={
              id === 'ch5-1'
                ? {
                    nodes: s.nodes.length,
                    side_systems: s.resources.length,
                    attachments: s.attachments.length,
                    conversation_history: s.history.length,
                    pinned_history: s.history.filter((c) => c.pinned).length,
                    mutation_history: s.mutations.length,
                    learning_proposals: s.proposals.length,
                    entries: s.nodes.reduce((n, x) => n + x.entries.length, 0),
                  }
                : undefined
            }
          />
        </SpecFigure>
      );
    case 'ch11-1':
      return (
        <SpecFigure
          id={id}
          title="Cycle lifecycle"
          caption="The kernel's trace phases over run_cycle. Phases light up as the running or last cycle reaches them."
          source={code}
        >
          <LifecycleDiagram trace={live.trace} busy={live.busy} />
        </SpecFigure>
      );
    case 'ch19-1':
      return (
        <SpecFigure
          id={id}
          title="Idle maintenance"
          caption="Each idle maintenance pass flips a coin between a bounded vote mutation and PHAGY cleanup. Counts come from the open specimen."
          source={code}
        >
          <MaintenanceDiagram specimen={s} />
        </SpecFigure>
      );
    case 'ch20-2':
      return (
        <SpecFigure
          id={id}
          title="Index Rafts planner"
          caption="The kernel's partition of a scan of N candidates with this specimen's settings: checkpoint chunks, each split into disjoint rafts."
          source={code}
        >
          <RaftsDiagram settings={s.settings} />
        </SpecFigure>
      );
    case 'ch24-1':
      return (
        <SpecFigure
          id={id}
          title="Save and restore"
          caption="How this studio restores a specimen and where it keeps it. The specification's wdbx.specimen.v1 layout is in the figure text."
          source={code}
        >
          <PersistenceDiagram desktop={live.desktop} />
        </SpecFigure>
      );
    default:
      return <CodeFigure id={id} lang={figure.lang} code={code} />;
  }
}
