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
      <button aria-label="Navigate Settings" onClick={() => setActiveRoute('/settings')}>
        Settings
      </button>
    </nav>
  );
}
