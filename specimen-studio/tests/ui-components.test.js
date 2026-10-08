import { expect, test } from 'bun:test';
import { createElement as h } from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import { BreadcrumbPage } from '../components/ui/breadcrumb';
import { InputOTPSeparator } from '../components/ui/input-otp';
import { Label } from '../components/ui/label';
import { PaginationLink } from '../components/ui/pagination';
import {
  ChartContainer,
  ChartTooltipContent,
  ChartLegendContent,
} from '../components/ui/chart';
import { useIsMobile } from '../hooks/use-mobile';

const html = (element) => renderToStaticMarkup(element);

test('current breadcrumb page is text with current-page semantics', () => {
  const output = html(h(BreadcrumbPage, null, 'Current'));
  expect(output).toContain('aria-current="page"');
  expect(output).not.toContain('role="link"');
  expect(output).not.toContain('aria-disabled');
});

test('OTP visual separator does not split the accessible input', () => {
  const output = html(h(InputOTPSeparator));
  expect(output).toContain('aria-hidden="true"');
  expect(output).not.toContain('role="separator"');
});

test('Label forwards explicit control association and children', () => {
  const output = html(h(Label, { htmlFor: 'notes' }, 'Notes'));
  expect(output).toContain('for="notes"');
  expect(output).toContain('Notes</label>');
});

test('PaginationLink forwards the page content and anchor target', () => {
  const output = html(
    h(PaginationLink, { href: '/?page=2', isActive: true }, '2'),
  );
  expect(output).toContain('href="/?page=2"');
  expect(output).toContain('aria-current="page"');
  expect(output).toContain('>2</a>');
});

test('mobile snapshot on the server is deterministic without window', () => {
  function View() {
    return h('output', null, String(useIsMobile()));
  }
  expect(html(h(View))).toBe('<output>false</output>');
});

test('chart scalar keys format numbers and function keys fall back to a configured name', () => {
  const config = {
    0: { label: 'Zero series' },
    value: { label: 'Fallback series' },
  };
  const tooltip = (dataKey, name) =>
    html(
      h(
        ChartContainer,
        { config },
        h(ChartTooltipContent, {
          active: true,
          payload: [{ dataKey, name, value: 10 }],
        }),
      ),
    );
  expect(tooltip(0, undefined)).toContain('Zero series');
  expect(tooltip(() => 10, 'value')).toContain('Fallback series');
  const legend = html(
    h(
      ChartContainer,
      { config },
      h(ChartLegendContent, {
        payload: [{ dataKey: () => 10, value: 'Ignored', color: 'red' }],
      }),
    ),
  );
  expect(legend).toContain('Fallback series');
});
