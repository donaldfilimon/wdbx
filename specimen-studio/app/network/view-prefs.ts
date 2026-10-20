/** Saved graph-view choices. Visual only; they do not train weights. */

export type CameraPreset = 'orbit' | 'top' | 'side';
export type EdgeFilter = 'all' | 'hard' | 'local';

export type NetworkViewPrefs = {
  camera: CameraPreset;
  edges: EdgeFilter;
  follow: boolean;
  edgeBudget: number;
};

export const DEFAULT_NETWORK_VIEW: NetworkViewPrefs = {
  camera: 'orbit',
  edges: 'all',
  follow: true,
  edgeBudget: 600,
};

const CAMERAS = new Set<CameraPreset>(['orbit', 'top', 'side']);
const EDGES = new Set<EdgeFilter>(['all', 'hard', 'local']);

export function parseNetworkViewPrefs(raw: string | null): NetworkViewPrefs {
  if (!raw) return { ...DEFAULT_NETWORK_VIEW };
  try {
    const value = JSON.parse(raw) as Partial<NetworkViewPrefs>;
    const budget = Number(value.edgeBudget);
    return {
      camera: CAMERAS.has(value.camera as CameraPreset)
        ? (value.camera as CameraPreset)
        : 'orbit',
      edges: EDGES.has(value.edges as EdgeFilter)
        ? (value.edges as EdgeFilter)
        : 'all',
      follow: value.follow !== false,
      edgeBudget:
        Number.isFinite(budget) && budget >= 80 && budget <= 600
          ? Math.round(budget)
          : 600,
    };
  } catch {
    return { ...DEFAULT_NETWORK_VIEW };
  }
}

/** Nodes to draw for the current edge filter. Local keeps the selection and its neighbors. */
export function visibleNodeRefs(
  all: string[],
  selected: string | undefined,
  edges: EdgeFilter,
  links: { from: string; to: string; hard: boolean }[],
): string[] {
  if (edges !== 'local' || !selected) return all;
  const keep = new Set<string>([selected]);
  for (const link of links) {
    if (link.from === selected) keep.add(link.to);
    if (link.to === selected) keep.add(link.from);
  }
  return all.filter((ref) => keep.has(ref));
}
