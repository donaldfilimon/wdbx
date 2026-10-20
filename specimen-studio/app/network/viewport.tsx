import {
  Component,
  Suspense,
  useState,
  useSyncExternalStore,
  type ReactNode,
} from 'react';
import { WButton } from '@/components/wdbx';

function hasWebGL() {
  try {
    const canvas = document.createElement('canvas');
    const gl = canvas.getContext('webgl2') ?? canvas.getContext('webgl');
    const ok = !!gl;
    gl?.getExtension('WEBGL_lose_context')?.loseContext();
    return ok;
  } catch {
    return false;
  }
}
let webgl: boolean | undefined;
const subscribe = () => () => {};
const readWebGL = () => (webgl ??= hasWebGL());
const serverWebGL = () => false;

class SceneBoundary extends Component<
  { fallback: ReactNode; children: ReactNode; onFail: () => void },
  { failed: boolean }
> {
  state = { failed: false };
  static getDerivedStateFromError() {
    return { failed: true };
  }
  componentDidCatch() {
    this.props.onFail();
  }
  render() {
    return this.state.failed ? this.props.fallback : this.props.children;
  }
}

/**
 * Hosts a 3D scene: one named image whose data is in the table beside it.
 * Without WebGL, or if the scene fails, a notice points to that table.
 * The probe context is released, and a failed scene can be tried again.
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
  const [attempt, setAttempt] = useState(0);
  const [failed, setFailed] = useState(false);
  const notice = (
    <p className="m-0 grid h-full place-items-center p-6 text-center text-sm text-ink-soft">
      3D view unavailable in this browser. {tableHint}
    </p>
  );
  return (
    <div className="grid gap-2">
      <div
        // oxlint-disable-next-line jsx-a11y/prefer-tag-over-role -- An <img> cannot host a WebGL canvas; the wrapper names the scene as one image.
        role="img"
        aria-label={`${label}. ${tableHint}`}
        className="h-80 overflow-hidden rounded-lg border border-line bg-surface-2 md:h-96"
      >
        {supported ? (
          <SceneBoundary
            key={attempt}
            onFail={() => setFailed(true)}
            fallback={notice}
          >
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
      {failed && (
        <WButton
          variant="outline"
          onClick={() => {
            setFailed(false);
            setAttempt((n) => n + 1);
          }}
        >
          Try the 3D view again
        </WButton>
      )}
    </div>
  );
}
