import { expect, test } from 'bun:test';
import { AGENT_MAY, AGENT_MAY_NOT } from '../lib/specimen/agent-policy';

test('studio agent policy keeps writes behind a person and off sibling stores', () => {
  expect(AGENT_MAY).toContain('run_specimen_prompt');
  expect(AGENT_MAY_NOT).toContain('write an ABI or Abbey store');
  expect(AGENT_MAY_NOT).toContain('train or update weights');
  expect(AGENT_MAY.some((tool) => tool.includes('dismiss'))).toBe(false);
});
