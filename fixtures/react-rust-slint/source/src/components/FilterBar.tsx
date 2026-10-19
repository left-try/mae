import React from 'react';

interface FilterBarProps {
  filterQuery: string;
}

export function FilterBar(props: FilterBarProps) {
  return (
    <section aria-label="Task Filter Section">
      <input
        aria-label="Search Tasks Input"
        value={props.filterQuery}
        onChange={(e) => setFilterQuery(e.target.value)}
      />
    </section>
  );
}
