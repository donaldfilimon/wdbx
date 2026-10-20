import { useEffect, useRef } from 'react';
import { AGENT_MAY, AGENT_MAY_NOT } from './specimen/agent-policy';
import type { Cycle, Specimen } from './specimen/types';
type Tool = {
  name: string;
  title: string;
  description: string;
  inputSchema: object;
  annotations: { readOnlyHint: boolean; untrustedContentHint: boolean };
  execute: (input: unknown) => unknown;
};
type Context = {
  registerTool: (
    tool: Tool,
    options: { signal: AbortSignal },
  ) => void | Promise<void>;
};
export function useStudioTools(actions: {
  read: () => Specimen;
  run: (input: string) => Promise<Cycle>;
  openChapter: (chapter: number) => void;
}) {
  const latest = useRef(actions);
  useEffect(() => {
    latest.current = actions;
  }, [actions]);
  useEffect(() => {
    const ctx = (document as Document & { modelContext?: Context })
      .modelContext;
    if (!ctx?.registerTool) return;
    const lifecycle = new AbortController();
    const object = (v: unknown): Record<string, unknown> => {
      if (!v || typeof v !== 'object' || Array.isArray(v))
        throw new Error('Expected an object.');
      return v as Record<string, unknown>;
    };
    const tools: Tool[] = [
      {
        name: 'read_specimen',
        title: 'Read active specimen',
        description:
          'Read the visible workspace summary and latest result without changing it.',
        inputSchema: {
          type: 'object',
          properties: {},
          additionalProperties: false,
        },
        annotations: { readOnlyHint: true, untrustedContentHint: true },
        execute(input) {
          if (Object.keys(object(input)).length)
            throw new Error('No arguments expected.');
          const s = latest.current.read();
          return {
            name: s.name,
            nodes: s.nodes.map((n) => ({
              ref: n.ref,
              name: n.name,
              patternId: n.patternId,
              strength: n.strength,
            })),
            memories: s.resources.length,
            latest: s.history.at(-1)?.segments.map((x) => x.text),
            pendingProposals: s.proposals.filter((p) => p.status === 'pending'),
          };
        },
      },
      {
        name: 'run_specimen_prompt',
        title: 'Run specimen prompt',
        description:
          'Write, after visible approval: run one cycle and append it to history. Declining runs nothing. Does not accept slash commands, teach, dismiss, install a model, or change weights.',
        inputSchema: {
          type: 'object',
          properties: {
            input: { type: 'string', minLength: 1, maxLength: 8000 },
          },
          required: ['input'],
          additionalProperties: false,
        },
        annotations: { readOnlyHint: false, untrustedContentHint: true },
        async execute(input) {
          const v = object(input);
          if (
            Object.keys(v).some((k) => k !== 'input') ||
            typeof v.input !== 'string' ||
            !v.input.trim() ||
            v.input.length > 8000 ||
            v.input.startsWith('/')
          )
            throw new Error(
              'Provide a nonempty prompt of at most 8,000 characters, without a slash command.',
            );
          const c = await latest.current.run(v.input);
          return {
            cycleId: c.id,
            status: c.status,
            text: c.segments.map((s) => s.text),
            contributors: c.votes.map((v) => v.nodeRef),
          };
        },
      },
      {
        name: 'open_specification_chapter',
        title: 'Open specification chapter',
        description:
          'Navigate the integrated reference reader to one of its 26 chapters.',
        inputSchema: {
          type: 'object',
          properties: { chapter: { type: 'integer', minimum: 1, maximum: 26 } },
          required: ['chapter'],
          additionalProperties: false,
        },
        annotations: { readOnlyHint: false, untrustedContentHint: false },
        execute(input) {
          const v = object(input);
          if (
            Object.keys(v).some((k) => k !== 'chapter') ||
            !Number.isInteger(v.chapter) ||
            Number(v.chapter) < 1 ||
            Number(v.chapter) > 26
          )
            throw new Error('Choose a chapter from 1 to 26.');
          latest.current.openChapter(Number(v.chapter));
          return { chapter: v.chapter };
        },
      },
      {
        name: 'workspace_status',
        title: 'Workspace control status',
        description:
          'Read-only permission report. Lists counts and the actions an agent must not take. Dismiss, teach, model install, and weight changes stay on visible controls. Does not write this specimen or any ABI or Abbey store.',
        inputSchema: {
          type: 'object',
          properties: {},
          additionalProperties: false,
        },
        annotations: { readOnlyHint: true, untrustedContentHint: true },
        execute(input) {
          if (Object.keys(object(input)).length)
            throw new Error('No arguments expected.');
          const s = latest.current.read();
          const count = (status: string) =>
            s.proposals.filter((p) => p.status === status).length;
          return {
            permission: 'read',
            writesRequireVisibleApproval: true,
            nodes: s.nodes.length,
            memories: s.resources.length,
            pendingProposals: count('pending'),
            learnedProposals: count('learned'),
            dismissedProposals: count('dismissed'),
            agentMay: AGENT_MAY,
            agentMayNot: AGENT_MAY_NOT,
          };
        },
      },
    ];
    for (const tool of tools) {
      try {
        void Promise.resolve(
          ctx.registerTool(tool, { signal: lifecycle.signal }),
        ).catch((e) => console.warn('WebMCP registration unavailable', e));
      } catch (e) {
        console.warn('WebMCP registration unavailable', e);
      }
    }
    return () => lifecycle.abort();
  }, []);
}
