/**
 * What a studio agent may do. ABI's SEA loop and Abbey's learn pipeline are
 * separate products; this desk does not write their stores.
 */

export const AGENT_MAY = [
  'read_specimen',
  'workspace_status',
  'run_specimen_prompt',
  'open_specification_chapter',
] as const;

export const AGENT_MAY_NOT = [
  'dismiss or reconsider proposals',
  'teach a pattern',
  'install or generate with a model',
  'train or update weights',
  'write an ABI or Abbey store',
] as const;

export type AgentTool = (typeof AGENT_MAY)[number];
