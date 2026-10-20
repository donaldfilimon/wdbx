import { Badge } from '@/components/ui/badge';
import { Panel, WButton } from '@/components/wdbx';
import { AGENT_MAY, AGENT_MAY_NOT } from '@/lib/specimen/agent-policy';
import type { Specimen } from '@/lib/specimen/types';

/** The specimen agent desk. Approval stays visible; sibling stores stay untouched. */
export function AgentDesk({
  state,
  agentAsk,
  busy,
  onApprove,
  onDecline,
}: {
  state: Specimen;
  agentAsk: string | null;
  busy: boolean;
  onApprove?: () => void;
  onDecline?: () => void;
}) {
  const count = (status: 'pending' | 'learned' | 'dismissed') =>
    state.proposals.filter((proposal) => proposal.status === status).length;
  return (
    <Panel
      title="Agent desk"
      description="Studio approves specimen tools. ABI complete --learn and Abbey learn stay in their own programs."
    >
      <div className="grid gap-3 p-4 md:grid-cols-3">
        <section aria-label="Agent may">
          <h3 className="m-0 text-sm text-ink">May do</h3>
          <ul className="m-0 mt-2 list-none p-0 text-sm text-ink-soft">
            {AGENT_MAY.map((tool) => (
              <li key={tool}>
                <Badge variant="outline">{tool}</Badge>
              </li>
            ))}
          </ul>
        </section>
        <section aria-label="Needs you">
          <h3 className="m-0 text-sm text-ink">Needs you</h3>
          <p className="m-0 mt-2 text-sm text-ink-soft">
            {count('pending')} proposals waiting. {count('learned')} learned.{' '}
            {count('dismissed')} dismissed. A run from an agent waits here
            before it appends a cycle.
          </p>
          {agentAsk ? (
            <div className="mt-3 grid gap-2">
              <p className="m-0 text-sm text-ink">{agentAsk}</p>
              <div className="flex flex-wrap gap-2">
                <WButton disabled={busy} onClick={onApprove}>
                  Approve agent run
                </WButton>
                <WButton variant="outline" disabled={busy} onClick={onDecline}>
                  Decline agent run
                </WButton>
              </div>
            </div>
          ) : (
            <p className="m-0 mt-2 text-sm text-muted-foreground">
              No agent request is waiting.
            </p>
          )}
        </section>
        <section aria-label="Agent may not">
          <h3 className="m-0 text-sm text-ink">Never from an agent</h3>
          <ul className="m-0 mt-2 list-none p-0 text-sm text-ink-soft">
            {AGENT_MAY_NOT.map((item) => (
              <li key={item}>{item}</li>
            ))}
          </ul>
        </section>
      </div>
    </Panel>
  );
}
