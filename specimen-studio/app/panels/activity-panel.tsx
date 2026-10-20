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
import { Badge } from '@/components/ui/badge';
import { type Specimen } from '@/lib/specimen/types';
import { AgentDesk } from './agent-desk';

export function ActivityView({
  state,
  onMaintenance,
  onLearn,
  onReview,
  onDismiss,
  onReconsider,
  onDismissAll,
  onReconsiderAll,
  agentAsk,
  onApprove,
  onDecline,
  busy,
}: {
  state: Specimen;
  onMaintenance: (m: 'phagy' | 'mutation') => void;
  onLearn: (pattern: string) => void;
  onReview: () => void;
  onDismiss?: (id: string) => void;
  onReconsider?: (id: string) => void;
  onDismissAll?: () => void;
  onReconsiderAll?: () => void;
  agentAsk?: string | null;
  onApprove?: () => void;
  onDecline?: () => void;
  busy: boolean;
}) {
  const proposals = state.proposals;
  return (
    <div className="grid gap-4">
      <AgentDesk
        state={state}
        agentAsk={agentAsk ?? null}
        busy={busy}
        onApprove={onApprove}
        onDecline={onDecline}
      />
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
      {proposals.length > 0 && (
        <Panel title="Learning proposals">
          <div className="px-4 py-1 text-xs text-muted-foreground">
            {proposals.filter((p) => p.status === 'pending').length} pending ·{' '}
            {proposals.filter((p) => p.status === 'learned').length} learned ·{' '}
            {proposals.filter((p) => p.status === 'dismissed').length} dismissed
          </div>
          <div className="flex gap-2 px-4 pb-2">
            {proposals.some((p) => p.status === 'pending') && onDismissAll && (
              <WButton
                variant="outline"
                onClick={() => onDismissAll()}
                disabled={busy}
              >
                Dismiss all pending
              </WButton>
            )}
            {proposals.some((p) => p.status === 'dismissed') &&
              onReconsiderAll && (
                <WButton
                  variant="outline"
                  onClick={() => onReconsiderAll()}
                  disabled={busy}
                >
                  Reconsider all dismissed
                </WButton>
              )}
          </div>
          <ul className="m-0 list-none divide-y divide-line p-0">
            {proposals.map((p) => (
              <li
                key={p.id}
                className="flex flex-wrap items-center gap-3 px-4 py-3"
              >
                <div className="min-w-0 flex-1">
                  <strong className="text-sm text-ink">{p.pattern}</strong>
                  <Badge
                    variant={
                      p.status === 'pending'
                        ? 'default'
                        : p.status === 'dismissed'
                          ? 'outline'
                          : 'secondary'
                    }
                    className="ml-2 text-[10px]"
                  >
                    {p.status}
                  </Badge>
                  <p className="m-0 text-sm text-ink-soft">
                    {p.evidence} unmatched prompts.{' '}
                    {p.status === 'pending'
                      ? 'Supply response to learn.'
                      : p.status === 'dismissed'
                        ? 'Dismissed from suggestions.'
                        : 'Learned.'}
                  </p>
                  {(p.pros?.length || p.cons?.length) && (
                    <div className="mt-1 text-[10px] text-muted-foreground">
                      {p.pros?.length ? (
                        <span>Pros: {p.pros.join(', ')} </span>
                      ) : null}
                      {p.cons?.length ? (
                        <span>Cons: {p.cons.join(', ')}</span>
                      ) : null}
                    </div>
                  )}
                  {p.observer && (
                    <div className="mt-0.5 text-[10px] text-muted-foreground">
                      Observer: {p.observer}
                    </div>
                  )}
                </div>
                {p.status === 'pending' && (
                  <>
                    <WButton
                      variant="primary"
                      onClick={() => onLearn(p.pattern)}
                      disabled={busy}
                    >
                      Review & learn
                    </WButton>
                    <WButton
                      variant="outline"
                      onClick={() => onDismiss?.(p.id)}
                      disabled={busy}
                    >
                      Dismiss
                    </WButton>
                  </>
                )}
                {p.status === 'dismissed' && onReconsider && (
                  <WButton
                    variant="outline"
                    onClick={() => onReconsider(p.id)}
                    disabled={busy}
                  >
                    Reconsider
                  </WButton>
                )}
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
