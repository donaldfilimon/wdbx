export type View =
  | 'studio'
  | 'nodes'
  | 'memory'
  | 'activity'
  | 'specification'
  | 'settings'
  | 'lab';

/** Every view, in navigation order. */
export const VIEWS: readonly View[] = [
  'studio',
  'nodes',
  'memory',
  'activity',
  'lab',
  'specification',
  'settings',
];

export interface Route {
  view: View;
  chapter?: number;
  /** Second view shown beside `view`; never `view` itself or settings. */
  split?: View;
}

const CHAPTERS = 26;
const isView = (v: string | null): v is View =>
  v !== null && (VIEWS as readonly string[]).includes(v);

export function parseRoute(search: string): Route {
  const p = new URLSearchParams(search);
  const requested = p.get('view');
  const view: View = isView(requested) ? requested : 'studio';
  const route: Route = { view };
  const chapter = Number(p.get('chapter'));
  if (Number.isInteger(chapter) && chapter >= 1 && chapter <= CHAPTERS) {
    route.chapter = chapter;
  }
  const split = p.get('split');
  if (isView(split) && split !== view && split !== 'settings') {
    route.split = split;
  }
  return route;
}

export function routeSearch(route: Route): string {
  const p = new URLSearchParams();
  p.set('view', route.view);
  if (route.chapter) p.set('chapter', String(route.chapter));
  if (route.split) p.set('split', route.split);
  return `?${p}`;
}
