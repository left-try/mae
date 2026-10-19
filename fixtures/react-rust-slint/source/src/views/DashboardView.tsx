import React from 'react';

export interface DashboardViewProps {
  completionPercent: number;
  activeSprint: string;
}

export function DashboardView(props: DashboardViewProps) {
  return (
    <section aria-label="Dashboard Overview Screen">
      <h2>Executive Productivity Dashboard</h2>
      <span role="status" aria-label="Active Sprint Badge">
        {props.activeSprint}
      </span>
      <progress
        aria-label="Sprint Completion Progress"
        value={props.completionPercent}
        max={100}
      />
      <button aria-label="Refresh Metrics Button" onClick={() => setMetricsRefreshed(true)}>
        Refresh Metrics
      </button>
      <button aria-label="Export CSV Report Button" onClick={() => setReportExported(true)}>
        Export CSV Report
      </button>
      <button aria-label="Archive Completed Button" onClick={() => setCompletedArchived(true)}>
        Archive Completed
      </button>
      <button aria-label="Run Diagnostics Button" onClick={() => setModalOpen(true)}>
        Run Diagnostics
      </button>
    </section>
  );
}
