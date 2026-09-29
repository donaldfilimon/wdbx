/* oxlint-disable next/no-html-link-for-pages -- Shared browser/desktop view uses host-neutral local navigation. */
import type { RefObject } from 'react';
import {
  Boxes,
  Moon,
  PanelLeftClose,
  PanelLeftOpen,
  Settings2,
  Sun,
  X,
} from 'lucide-react';
import type { View } from '../state/navigation';
import type { Theme } from './theme';
import { viewGroups, views } from './view-meta';

/**
 * The one main navigation. On desktop it collapses to an icon rail; at 760 px
 * and below it is the slide-in drawer the e2e suites pin (#workspace-navigation,
 * .sidebar.is-open, .nav-backdrop only while open).
 */
export function AppSidebar({
  view,
  nav,
  nodeCount,
  mobileNav,
  setMobileNav,
  mobileViewport,
  collapsed,
  toggleCollapsed,
  theme,
  toggleTheme,
  sidebarRef,
  mobileCloseRef,
}: {
  view: View;
  nav: (v: View) => void;
  nodeCount: number;
  mobileNav: boolean;
  setMobileNav: (open: boolean) => void;
  mobileViewport: boolean;
  collapsed: boolean;
  toggleCollapsed: () => void;
  theme: Theme;
  toggleTheme: () => void;
  sidebarRef: RefObject<HTMLElement | null>;
  mobileCloseRef: RefObject<HTMLButtonElement | null>;
}) {
  const hidden = mobileViewport && !mobileNav ? true : undefined;
  return (
    <>
      <aside
        ref={sidebarRef}
        id="workspace-navigation"
        className={`sidebar ${mobileNav ? 'is-open' : ''} ${collapsed ? 'is-collapsed' : ''}`}
        aria-label="WDBX workspace"
        aria-hidden={hidden}
        inert={hidden}
      >
        <a
          className="brand"
          href="?view=studio"
          title="WDBX Specimen Studio"
          onClick={(e) => {
            e.preventDefault();
            nav('studio');
          }}
        >
          <Boxes size={39} strokeWidth={1.4} />
          <span className="nav-label">
            <strong translate="no">WDBX</strong>
            <small>Specimen Studio</small>
          </span>
        </a>
        <button
          ref={mobileCloseRef}
          className="icon-button mobile-close"
          aria-label="Close navigation"
          onClick={() => setMobileNav(false)}
        >
          <X />
        </button>
        <nav aria-label="Main navigation">
          {viewGroups.map((group) => (
            <div className="nav-group" key={group.label}>
              <span className="nav-group-label nav-label">{group.label}</span>
              {group.ids.map((id) => {
                const item = views.find((candidate) => candidate.id === id)!;
                const Icon = item.icon;
                return (
                  <a
                    key={id}
                    href={`?view=${id}`}
                    title={collapsed ? item.label : undefined}
                    className={`nav-link ${view === id ? 'active' : ''}`}
                    aria-current={view === id ? 'page' : undefined}
                    onClick={(e) => {
                      if (!e.metaKey && !e.ctrlKey) {
                        e.preventDefault();
                        nav(id);
                      }
                    }}
                  >
                    <Icon size={19} />
                    <span className="nav-label">{item.label}</span>
                    {id === 'nodes' && (
                      <span className="nav-count">{nodeCount}</span>
                    )}
                  </a>
                );
              })}
            </div>
          ))}
        </nav>
        <div className="sidebar-bottom">
          <a
            className={`nav-link ${view === 'settings' ? 'active' : ''}`}
            href="?view=settings"
            title={collapsed ? 'Settings' : undefined}
            onClick={(e) => {
              e.preventDefault();
              nav('settings');
            }}
          >
            <Settings2 size={20} />
            <span className="nav-label">Settings</span>
          </a>
          <div className="sidebar-tools">
            <button
              className="icon-button sidebar-tool"
              aria-label={
                theme === 'dark'
                  ? 'Switch to light theme'
                  : 'Switch to dark theme'
              }
              title={
                theme === 'dark'
                  ? 'Switch to light theme'
                  : 'Switch to dark theme'
              }
              onClick={toggleTheme}
            >
              {theme === 'dark' ? <Sun size={18} /> : <Moon size={18} />}
            </button>
            {!mobileViewport && (
              <button
                className="icon-button sidebar-tool"
                aria-label={collapsed ? 'Expand sidebar' : 'Collapse sidebar'}
                title={
                  collapsed ? 'Expand sidebar (⌘B)' : 'Collapse sidebar (⌘B)'
                }
                aria-pressed={collapsed}
                onClick={toggleCollapsed}
              >
                {collapsed ? (
                  <PanelLeftOpen size={18} />
                ) : (
                  <PanelLeftClose size={18} />
                )}
              </button>
            )}
          </div>
          <div className="workspace-caption nav-label">
            <span className="status-dot" />
            <div>
              Local workspace<small>Your data stays on this device</small>
            </div>
          </div>
        </div>
      </aside>
      {mobileNav && (
        <button
          aria-label="Close navigation overlay"
          className="nav-backdrop"
          onClick={() => setMobileNav(false)}
        />
      )}
    </>
  );
}
