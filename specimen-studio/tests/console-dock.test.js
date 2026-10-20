import { expect, test } from 'bun:test';
import { createElement as h } from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import { ConsoleDock } from '../app/shell/console-dock';

test('the activity dock renders a command line and keeps the event log', () => {
  const out = renderToStaticMarkup(
    h(ConsoleDock, {
      events: [
        {
          id: 'e1',
          type: 'node',
          title: 'Pattern learned',
          detail: 'Color',
          createdAt: '2026-01-01T00:00:00.000Z',
        },
      ],
      replies: [{ id: 1, text: '/help — List commands.' }],
      onCommand: () => {},
    }),
  );
  expect(out).toContain('aria-label="Activity dock"');
  expect(out).toContain('Pattern learned');
  expect(out).toContain('>Command line</label>');
  expect(out).toContain('Run command');
  expect(out).toContain('/help — List commands.');
  expect(out).toContain('name="command"');
});
