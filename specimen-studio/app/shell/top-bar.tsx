import type { RefObject } from 'react';
import {
  ArrowDownToLine,
  FileText,
  Menu,
  PanelBottom,
  RotateCcw,
  Save,
  Search,
  Upload,
} from 'lucide-react';
import type { View } from '../state/navigation';
import type { StudioModel } from '../state/use-studio';
import { labels } from './view-meta';

/** Page header: the view's only h1, specimen actions, palette and dock. */
export function TopBar({
  view,
  model,
  mobileNav,
  setMobileNav,
  mobileMenuRef,
  openPalette,
  dockOpen,
  toggleDock,
}: {
  view: View;
  model: StudioModel;
  mobileNav: boolean;
  setMobileNav: (open: boolean) => void;
  mobileMenuRef: RefObject<HTMLButtonElement | null>;
  openPalette: () => void;
  dockOpen: boolean;
  toggleDock: () => void;
}) {
  const { state, busy, openDialog, setDialogReturn, fileRef, save } = model;
  return (
    <header className="page-header">
      <div className="heading-wrap">
        <button
          ref={mobileMenuRef}
          className="icon-button mobile-menu"
          aria-label="Open navigation"
          aria-controls="workspace-navigation"
          aria-expanded={mobileNav}
          onClick={() => setMobileNav(true)}
        >
          <Menu />
        </button>
        <div>
          <div className="page-title-row">
            <h1>{labels[view][0]}</h1>
            {view === 'studio' && (
              <span className="specimen-name header-specimen">
                <FileText size={15} aria-hidden="true" />
                {state.name}
                <i className={`status-dot ${busy ? 'busy' : ''}`} />
                <small>{busy ? 'Running' : 'Ready'}</small>
              </span>
            )}
          </div>
          <p>{labels[view][1]}</p>
        </div>
      </div>
      <div className="header-actions">
        <button
          className="button utility palette-trigger"
          aria-label="Search or run a command"
          aria-keyshortcuts="Meta+K Control+K"
          onClick={openPalette}
        >
          <Search size={16} />
          <span>Commands</span>
          <kbd>⌘K</kbd>
        </button>
        <button
          className="button utility icon-only"
          aria-label={dockOpen ? 'Hide activity dock' : 'Show activity dock'}
          aria-pressed={dockOpen}
          title="Activity dock (`)"
          onClick={toggleDock}
        >
          <PanelBottom size={17} />
        </button>
        {view === 'specification' ? (
          <a
            className="button outline"
            href="/WDBX-Specimen-Architecture-Specification.md"
            download
          >
            <ArrowDownToLine size={17} />
            Download Markdown
          </a>
        ) : (
          <>
            {view === 'studio' && (
              <button
                className="button utility starter-action"
                aria-label="Reset to starter specimen"
                onClick={() => openDialog('reset')}
              >
                <RotateCcw size={16} />
                <span>Starter</span>
              </button>
            )}
            <button
              className="button utility"
              aria-label="Load specimen"
              onClick={(event) => {
                setDialogReturn(event.currentTarget);
                fileRef.current?.click();
              }}
              disabled={busy}
            >
              <Upload size={17} />
              <span>Load</span>
            </button>
            <button
              className="button outline"
              aria-label="Save specimen"
              onClick={save}
            >
              <Save size={17} />
              <span>Save specimen</span>
            </button>
          </>
        )}
      </div>
    </header>
  );
}
