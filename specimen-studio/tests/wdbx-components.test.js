import { expect, test } from 'bun:test';
import { createElement as h } from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import {
  CheckboxField,
  CodeBlock,
  control,
  Disclosure,
  Field,
  FieldRow,
  FormError,
  EmptyState,
  EventTimeline,
  Panel,
  Stat,
  StatGroup,
  Toolbar,
  WButton,
} from '../components/wdbx';

const html = (el) => renderToStaticMarkup(el);

test('Panel is a labelled section with a heading', () => {
  const out = html(
    h(Panel, { title: 'Learning proposals', actions: h('button', null, 'x') }),
  );
  expect(out).toMatch(/^<section[^>]*aria-labelledby="([^"]+)"/);
  const id = out.match(/aria-labelledby="([^"]+)"/)[1];
  expect(out).toContain(`<h2 id="${id}"`);
  expect(out).toContain('Learning proposals');
  expect(out).toContain('<button>x</button>');
});

test('Panel without a title is an unlabelled section', () => {
  const out = html(h(Panel, { className: 'activity-panel' }, 'body'));
  expect(out).not.toContain('aria-labelledby');
  expect(out).toContain('activity-panel');
});

test('WButton variants keep a 44px target and focus ring', () => {
  const out = html(h(WButton, { variant: 'primary' }, 'Run'));
  expect(out).toContain('<button');
  expect(out).toContain('type="button"');
  expect(out).toMatch(/min-h-11/);
  expect(out).toMatch(/focus-visible:outline/);
  const outline = html(h(WButton, { variant: 'outline', disabled: true }, 'x'));
  expect(outline).toContain('disabled=""');
});

test('Toolbar is a labelled toolbar', () => {
  const out = html(h(Toolbar, { label: 'Activity actions' }, 'a'));
  expect(out).toContain('role="toolbar"');
  expect(out).toContain('aria-label="Activity actions"');
});

test('Stat pairs a label with its value', () => {
  const out = html(
    h(StatGroup, { label: 'Overview' }, h(Stat, { label: 'Nodes', value: 8 })),
  );
  expect(out).toContain('<dl');
  expect(out).toContain('<dt');
  expect(out).toContain('Nodes');
  expect(out).toContain('<dd');
  expect(out).toContain('8');
});

test('EventTimeline renders an ordered list with machine-readable times', () => {
  const out = html(
    h(EventTimeline, {
      events: [
        {
          id: 'e1',
          type: 'feedback',
          title: 'Positive feedback',
          detail: '1 of 1',
          createdAt: '2026-01-01T00:00:00.000Z',
        },
      ],
    }),
  );
  expect(out).toContain('<ol');
  expect(out).toContain('<time dateTime="2026-01-01T00:00:00.000Z"');
  expect(out).toContain('Positive feedback');
});

test('EmptyState explains itself', () => {
  const out = html(h(EmptyState, { title: 'No events', text: 'Run a cycle.' }));
  expect(out).toContain('No events');
  expect(out).toContain('Run a cycle.');
});

test('CodeBlock is a focusable, labelled scroll region', () => {
  const out = html(h(CodeBlock, { label: 'Vote evidence' }, '{}'));
  expect(out).toContain('<pre');
  expect(out).toContain('tabindex="0"');
  expect(out).toContain('aria-label="Vote evidence"');
});

test('Field labels its control and keeps the 44px control height', () => {
  const out = html(
    h(Field, { label: 'Node name', htmlFor: 'n' }, h('input', { id: 'n', className: control })),
  );
  expect(out).toContain('<label for="n"');
  expect(out).toContain('Node name');
  expect(out).toMatch(/<input id="n" class="[^"]*min-h-11/);
});

test('CheckboxField is a labelled checkbox with a 44px target', () => {
  const out = html(h(CheckboxField, { name: 'hard', label: 'Hard attachment' }));
  expect(out).toMatch(/<label class="[^"]*min-h-11[^"]*"><input type="checkbox"[^>]*name="hard"/);
  expect(out).toContain('Hard attachment');
});

test('FormError is an alert; Disclosure has a 44px summary; FieldRow is a grid', () => {
  expect(html(h(FormError, null, 'Choose two nodes.'))).toMatch(/^<p role="alert"/);
  expect(html(h(Disclosure, { summary: 'More' }, 'x'))).toMatch(/<summary class="[^"]*min-h-11/);
  expect(html(h(FieldRow, null, 'a'))).toMatch(/class="grid/);
});
