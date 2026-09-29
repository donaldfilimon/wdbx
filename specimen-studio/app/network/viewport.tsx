import {
  Component,
  Suspense,
  useSyncExternalStore,
  type ReactNode,
} from 'react';

function hasWebGL() {
  try {
    const canvas = document.createElement('canvas');
    return !!(canvas.getContext('webgl2') ?? canvas.getContext('webgl'));
  } catch {
    return false;
  }
}
let webgl: boolean | undefined;
const subscribe = () => () => {};
const readWebGL = () => (webgl ??= hasWebGL());
const serverWebGL = () => false;

class SceneBoundary extends Component<
  { fallback: ReactNode; children: ReactNode },
  { failed: boolean }
> {
  state = { failed: false };
  static getDerivedStateFromError() {
    return { failed: true };
  }
  render() {
    return this.state.failed ? this.props.fallback : this.props.children;
  }
}

/**
 * Hosts a 3D scene: one named image whose data is in the table beside it.
 * Without WebGL, or if the scene fails, a notice points to that table.
 */
export function Viewport({
  label,
  tableHint,
  children,
}: {
  label: string;
  tableHint: string;
  children: ReactNode;
}) {
  const supported = useSyncExternalStore(subscribe, readWebGL, serverWebGL);
  const notice = (
    <p className="m-0 grid h-full place-items-center p-6 text-center text-sm text-ink-soft">
      3D view unavailable in this browser. {tableHint}
    </p>
  );
  return (
    <div
      // oxlint-disable-next-line jsx-a11y/prefer-tag-over-role -- An <img> cannot host a WebGL canvas; the wrapper names the scene as one image.
      role="img"
      aria-label={`${label}. ${tableHint}`}
      className="h-80 overflow-hidden rounded-lg border border-line bg-surface-2 md:h-96"
    >
      {supported ? (
        <SceneBoundary fallback={notice}>
          <Suspense
            fallback={
              <p className="m-0 grid h-full place-items-center text-sm text-muted-foreground">
                Loading 3D view…
              </p>
            }
          >
            {children}
          </Suspense>
        </SceneBoundary>
      ) : (
        notice
      )}
    </div>
  );
}
