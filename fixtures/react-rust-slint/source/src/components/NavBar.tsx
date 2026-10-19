import React from 'react';

interface NavBarProps {
  activeRoute: string;
  title: string;
}

export function NavBar(props: NavBarProps) {
  return (
    <nav aria-label="Primary Navigation">
      <h1>{props.title}</h1>
      <button aria-label="Navigate Dashboard" onClick={() => setActiveRoute('/')}>
        Dashboard
      </button>
      <button aria-label="Navigate Tasks" onClick={() => setActiveRoute('/tasks')}>
        Tasks
      </button>
      <button aria-label="Navigate Kanban" onClick={() => setActiveRoute('/kanban')}>
        Kanban Board
      </button>
      <button aria-label="Navigate Analytics" onClick={() => setActiveRoute('/analytics')}>
        Analytics
      </button>
      <button aria-label="Navigate Notifications" onClick={() => setActiveRoute('/notifications')}>
        Notifications
      </button>
      <button aria-label="Navigate Settings" onClick={() => setActiveRoute('/settings')}>
        Settings
      </button>
      <button aria-label="Open Status Modal" onClick={() => setModalOpen(true)}>
        Status Modal
      </button>
      <button aria-label="Sync Workspace Now" onClick={() => setSyncTriggered(true)}>
        Sync Now
      </button>
    </nav>
  );
}
