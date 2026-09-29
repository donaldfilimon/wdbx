import type { ReactNode } from 'react';
import { X } from 'lucide-react';
import {
  ResizableHandle,
  ResizablePanel,
  ResizablePanelGroup,
} from '@/components/ui/resizable';
import { VIEW_TITLES } from './commands';
import type { View } from '../state/navigation';

/**
 * One pane, or the primary view beside a side pane. The side pane renders only
 * on wide viewports; on narrow ones the split stays in the URL and returns
 * when the window widens.
 */
export function PaneLayout({
  primary,
  side,
  sideView,
  wide,
  onCloseSide,
}: {
  primary: ReactNode;
  side: ReactNode;
  sideView?: View;
  wide: boolean;
  onCloseSide: () => void;
}) {
  if (!sideView || !wide) return <>{primary}</>;
  const title = VIEW_TITLES[sideView];
  return (
    <ResizablePanelGroup orientation="horizontal" className="pane-group">
      <ResizablePanel defaultSize="60%" minSize="40%">
        <div className="pane pane-primary">{primary}</div>
      </ResizablePanel>
      <ResizableHandle withHandle className="pane-handle" />
      <ResizablePanel defaultSize="40%" minSize="25%">
        <section className="pane pane-side" aria-label={`Side pane: ${title}`}>
          <header className="pane-side-head">
            <h2>{title}</h2>
            <button
              className="icon-button"
              aria-label="Close side pane"
              title="Close side pane (⌘\)"
              onClick={onCloseSide}
            >
              <X size={16} />
            </button>
          </header>
          {side}
        </section>
      </ResizablePanel>
    </ResizablePanelGroup>
  );
}
