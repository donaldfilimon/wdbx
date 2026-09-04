import { useEffect, useRef } from 'react';
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
          'Run one pattern retrieval and composition cycle, adding its result to visible conversation history. Does not accept slash commands.',
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
