import React from 'react';

interface FilterBarProps {
  filterQuery: string;
}

export function FilterBar(props: FilterBarProps) {
  return (
    <section aria-label="Task Filter Section">
      <input
        type="search"
        aria-label="Search Tasks Input"
        value={props.filterQuery}
        onChange={(e) => setFilterQuery(e.target.value)}
      />
      <select aria-label="Priority Filter Select" onChange={(e) => setPriorityFilter(e.target.value)}>
        <option value="all">All Priorities</option>
        <option value="critical">Critical</option>
        <option value="high">High</option>
        <option value="normal">Normal</option>
      </select>
      <button role="tab" aria-label="Filter All Tab" onClick={() => setStatusTab('all')}>
        All Tasks
      </button>
      <button role="tab" aria-label="Filter Active Tab" onClick={() => setStatusTab('active')}>
        Active Only
      </button>
      <button role="tab" aria-label="Filter Completed Tab" onClick={() => setStatusTab('completed')}>
        Completed Only
      </button>
      <button aria-label="Clear Filters Button" onClick={() => setFilterQuery('')}>
        Clear Filters
      </button>
    </section>
  );
}
