import React, { useState } from 'react';

interface TaskFormProps {
  placeholder: string;
}

export function TaskForm(props: TaskFormProps) {
  const [draftTitle, setDraftTitle] = useState<string>('');
  const [draftPriority, setDraftPriority] = useState<string>('high');
  const [isUrgent, setIsUrgent] = useState<boolean>(false);

  const handleSubmit = () => {
    setDraftTitle('');
  };

  return (
    <form aria-label="New Task Form" onSubmit={handleSubmit}>
      <input
        type="text"
        aria-label="Task Title Input"
        placeholder={props.placeholder}
        value={draftTitle}
        onChange={(e) => setDraftTitle(e.target.value)}
      />
      <select
        aria-label="Task Priority Select"
        value={draftPriority}
        onChange={(e) => setDraftPriority(e.target.value)}
      >
        <option value="critical">Critical</option>
        <option value="high">High</option>
        <option value="normal">Normal</option>
      </select>
      <input
        type="checkbox"
        aria-label="Urgent Task Checkbox"
        checked={isUrgent}
        onChange={() => setIsUrgent(!isUrgent)}
      />
      <button aria-label="Add Task Button" onClick={handleSubmit}>
        Add Task
      </button>
      <button aria-label="Save Draft Button" onClick={() => setDraftSaved(true)}>
        Save Draft
      </button>
    </form>
  );
}
