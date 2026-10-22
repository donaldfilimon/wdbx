import { expect, test } from 'bun:test';
import {
  AGENT_MAY,
  AGENT_MAY_NOT,
  recordAgentAudit,
} from '../lib/specimen/agent-policy';

test('studio agent policy keeps writes behind a person and off sibling stores', () => {
  expect(AGENT_MAY).toContain('run_specimen_prompt');
  expect(AGENT_MAY_NOT).toContain('write an ABI or Abbey store');
  expect(AGENT_MAY_NOT).toContain('train or update weights');
  expect(AGENT_MAY.some((tool) => tool.includes('dismiss'))).toBe(false);
});

test('agent audit keeps the newest call and drops the oldest', () => {
  const log = recordAgentAudit(
    [{ id: 1, tool: 'read_specimen', outcome: 'read', detail: 'old' }],
    { tool: 'workspace_status', outcome: 'read', detail: 'permission report' },
  );
  expect(log[0].tool).toBe('workspace_status');
  expect(log[0].id).toBe(2);
  expect(
    recordAgentAudit(
      Array.from({ length: 12 }, (_, i) => ({
        id: i + 1,
        tool: 'read_specimen',
        outcome: 'read',
        detail: String(i),
      })),
      { tool: 'read_specimen', outcome: 'read', detail: 'newest' },
    ),
  ).toHaveLength(12);
});
