import React from 'react';

export interface TasksViewProps {
  totalTasks: number;
  selectedCount: number;
}

export function TasksView(props: TasksViewProps) {
  return (
    <section aria-label="Tasks Management Screen">
      <h2>Task Backlog & Sprint Queue</h2>
      <input
        type="checkbox"
        aria-label="Select All Tasks Checkbox"
        onChange={() => setAllSelected(true)}
      />
      <select aria-label="Sort Order Select" onChange={(e) => setSortOrder(e.target.value)}>
        <option value="priority">Sort by Priority</option>
        <option value="due_date">Sort by Due Date</option>
        <option value="created_at">Sort by Created Date</option>
      </select>
      <button aria-label="Complete All Visible Button" onClick={() => setAllCompleted(true)}>
        Complete All Visible
      </button>
      <button aria-label="Assign to Sprint Button" onClick={() => setSprintAssigned(true)}>
        Assign to Current Sprint
      </button>
      <button aria-label="Bulk Delete Completed Button" onClick={() => setBulkDeleted(true)}>
        Bulk Delete Completed
      </button>
    </section>
  );
}
