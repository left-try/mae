import React from 'react';

export interface KanbanBoardViewProps {
  wipLimit: number;
  inProgressCount: number;
}

export function KanbanBoardView(props: KanbanBoardViewProps) {
  return (
    <section aria-label="Kanban Board Screen">
      <h2>Kanban Workflow Board</h2>
      <span role="status" aria-label="WIP Limit Badge">
        WIP: {props.inProgressCount}/{props.wipLimit}
      </span>
      <input
        type="range"
        aria-label="WIP Limit Slider"
        min={1}
        max={15}
        value={props.wipLimit}
        onChange={(e) => setWipLimit(e.target.value)}
      />
      <button aria-label="Move to In Progress Button" onClick={() => setCardColumn('in_progress')}>
        Move to In Progress
      </button>
      <button aria-label="Move to Review Button" onClick={() => setCardColumn('review')}>
        Move to Review
      </button>
      <button aria-label="Move to Done Button" onClick={() => setCardColumn('done')}>
        Move to Done
      </button>
      <button aria-label="Rebalance Columns Button" onClick={() => setBoardRebalanced(true)}>
        Rebalance Columns
      </button>
    </section>
  );
}
