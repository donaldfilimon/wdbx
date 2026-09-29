/* oxlint-disable next/no-html-link-for-pages -- Shared browser/desktop view uses host-neutral local navigation. */
import { GitBranch, Search, Shield } from 'lucide-react';
import {
  EmptyState,
  EventTimeline,
  Panel,
  Toolbar,
  ToolbarSpacer,
  WButton,
} from '@/components/wdbx';
import { type Specimen } from '@/lib/specimen/types';

export function ActivityView({
  state,
  onMaintenance,
  onLearn,
  onReview,
  busy,
}: {
  state: Specimen;
  onMaintenance: (m: 'phagy' | 'mutation') => void;
  onLearn: (pattern: string) => void;
  onReview: () => void;
  busy: boolean;
}) {
  const pending = state.proposals.filter((p) => p.status === 'pending');
  return (
    <div className="grid gap-4">
      <Toolbar label="Activity actions">
        <WButton disabled={busy} onClick={onReview}>
          <Search size={16} />
          Review context
        </WButton>
        <span className="text-sm text-muted-foreground">
          A record of cycles, learning, and maintenance
        </span>
        <ToolbarSpacer />
        <WButton disabled={busy} onClick={() => onMaintenance('mutation')}>
          <GitBranch size={16} />
          Run mutation scan
        </WButton>
        <WButton disabled={busy} onClick={() => onMaintenance('phagy')}>
          <Shield size={16} />
          Run PHAGY
        </WButton>
      </Toolbar>
      {pending.length > 0 && (
        <Panel title="Learning proposals">
          <ul className="m-0 list-none divide-y divide-line p-0">
            {pending.map((p) => (
              <li
                key={p.id}
                className="flex flex-wrap items-center gap-3 px-4 py-3"
              >
                <div className="min-w-0 flex-1">
                  <strong className="text-sm text-ink">{p.pattern}</strong>
                  <p className="m-0 text-sm text-ink-soft">
                    {p.evidence} unmatched prompts suggest a missing pattern.
                    Supply its response to approve learning.
                  </p>
                </div>
                <WButton variant="primary" onClick={() => onLearn(p.pattern)}>
                  Review & learn
                </WButton>
              </li>
            ))}
          </ul>
        </Panel>
      )}
      <Panel
        title="Activity"
        description={`${Math.min(state.events.length, 100)} most recent of ${state.events.length} events`}
      >
        {state.events.length ? (
          <EventTimeline events={state.events.slice(0, 100)} />
        ) : (
          <EmptyState
            title="No activity yet"
            text="Run a prompt or teach a pattern to start the record."
          />
        )}
      </Panel>
      <Panel title="About this runtime">
        <p className="m-0 px-4 py-3 text-sm text-ink-soft">
          Both editions run the same Rust specimen kernel; the browser loads it
          as WebAssembly and keeps data in this browser. The complete design
          remains available in Specification. Read the{' '}
          <a
            className="font-medium text-teal underline underline-offset-2"
            href="/RUNTIME-PROFILE.md"
            download
          >
            runtime profile
          </a>{' '}
          for exact behavior and limits.
        </p>
      </Panel>
    </div>
  );
}
