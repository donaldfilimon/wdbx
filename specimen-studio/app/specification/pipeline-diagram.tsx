import { ArrowDown } from 'lucide-react';
import {
  ORCHESTRATOR_INPUTS,
  PIPELINE,
  type PipelineStage,
} from '@/lib/specimen/spec-figures';
import { ChapterLink } from './figure';

const stageClass =
  'flex min-h-11 w-full items-center justify-between gap-3 rounded-lg border border-line bg-surface-2 px-3 py-2 text-left text-sm text-ink no-underline hover:border-teal hover:bg-teal-soft focus-visible:outline-2 focus-visible:outline-focus';

function Stage({
  stage,
  onChapter,
}: {
  stage: PipelineStage;
  onChapter: (n: number) => void;
}) {
  return (
    <ChapterLink
      chapter={stage.chapter}
      onChapter={onChapter}
      className={stageClass}
    >
      <span>{stage.label}</span>
      <span className="shrink-0 font-mono text-xs text-muted-foreground">
        ch. {stage.chapter}
      </span>
    </ChapterLink>
  );
}

const Arrow = () => (
  <ArrowDown
    aria-hidden="true"
    size={16}
    className="mx-auto my-0.5 text-muted-foreground"
  />
);

/** Chapter 2: the processing pipeline, each stage linked to its chapter. */
export function PipelineDiagram({
  onChapter,
}: {
  onChapter: (n: number) => void;
}) {
  const before = PIPELINE.filter((s) => !s.after);
  const [orchestrator, ...after] = PIPELINE.filter((s) => s.after);
  return (
    <div className="grid gap-0.5">
      <ol aria-label="Voter phase" className="m-0 grid list-none gap-0.5 p-0">
        {before.map((stage, i) => (
          <li key={stage.label}>
            {i > 0 && <Arrow />}
            <Stage stage={stage} onChapter={onChapter} />
          </li>
        ))}
      </ol>
      <p className="my-2 border-y-2 border-dashed border-amber py-1 text-center font-mono text-xs text-amber">
        phase boundary
      </p>
      <ol
        aria-label="Composition phase"
        className="m-0 grid list-none gap-0.5 p-0"
      >
        <li>
          <Stage stage={orchestrator} onChapter={onChapter} />
          <ul
            aria-label="Orchestrator inputs"
            className="my-2 grid list-none grid-cols-2 gap-2 p-0 sm:grid-cols-4"
          >
            {ORCHESTRATOR_INPUTS.map((input) => (
              <li key={input.label}>
                <Stage stage={input} onChapter={onChapter} />
              </li>
            ))}
          </ul>
        </li>
        {after.map((stage) => (
          <li key={stage.label}>
            <Arrow />
            <Stage stage={stage} onChapter={onChapter} />
          </li>
        ))}
      </ol>
      <div className="mt-3 rounded-lg border border-dashed border-line p-3 text-xs text-ink-soft">
        <strong className="text-ink">Idle:</strong> Type B correlation,
        attachment statistics, mutation-tag / PHAGY maintenance, approved growth{' '}
        <ChapterLink
          chapter={19}
          onChapter={onChapter}
          className="font-semibold text-teal"
        >
          (ch. 19)
        </ChapterLink>
      </div>
    </div>
  );
}
