/* oxlint-disable next/no-html-link-for-pages -- Shared browser/desktop view uses host-neutral local navigation. */
import { Activity, GitBranch, Search, Shield } from 'lucide-react';
import { type Specimen } from '@/lib/specimen/types';

import { clock } from '../state/format';

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
  return (
    <>
      <div className="workspace-tools">
        <button className="button outline" disabled={busy} onClick={onReview}>
          <Search size={16} />
          Review context
        </button>
        <span className="muted">
          A record of cycles, learning, and maintenance
        </span>
        <span className="toolbar-spacer" />
        <button
          className="button outline"
          disabled={busy}
          onClick={() => onMaintenance('mutation')}
        >
          <GitBranch size={16} />
          Run mutation scan
        </button>
        <button
          className="button outline"
          disabled={busy}
          onClick={() => onMaintenance('phagy')}
        >
          <Shield size={16} />
          Run PHAGY
        </button>
      </div>
      {state.proposals.some((p) => p.status === 'pending') && (
        <section className="panel proposals">
          <div className="panel-heading">
            <h2>Learning proposals</h2>
          </div>
          {state.proposals
            .filter((p) => p.status === 'pending')
            .map((p) => (
              <div className="proposal" key={p.id}>
                <div>
                  <strong>{p.pattern}</strong>
                  <p>
                    {p.evidence} unmatched prompts suggest a missing pattern.
                    Supply its response to approve learning.
                  </p>
                </div>
                <button
                  className="button primary"
                  onClick={() => onLearn(p.pattern)}
                >
                  Review & learn
                </button>
              </div>
            ))}
        </section>
      )}
      <section className="panel activity-panel">
        {state.events.slice(0, 100).map((event) => (
          <div className="activity-row" key={event.id}>
            <span
              className={`event-icon ${event.type === 'feedback' ? 'amber' : ''}`}
            >
              <Activity size={17} />
            </span>
            <div>
              <strong>{event.title}</strong>
              <p>{event.detail}</p>
            </div>
            <time dateTime={event.createdAt}>{clock(event.createdAt)}</time>
          </div>
        ))}
      </section>
      <section className="runtime-note">
        <h2>About this runtime</h2>
        <p>
          This browser studio runs local pattern matching, procedural sigils,
          votes, feedback, memory, attachments, and bounded maintenance. Its
          synthesis is fixed and does not train a language model. The complete
          design remains available in Specification. Read the{' '}
          <a href="/RUNTIME-PROFILE.md" download>
            browser runtime profile
          </a>{' '}
          for exact behavior and limits.
        </p>
      </section>
    </>
  );
}
