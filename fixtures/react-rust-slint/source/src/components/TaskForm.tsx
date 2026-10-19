import React, { useState } from 'react';

interface TaskFormProps {
  placeholder: string;
}

export function TaskForm(props: TaskFormProps) {
  const [draftTitle, setDraftTitle] = useState<string>('');

  const handleSubmit = () => {
    setDraftTitle('');
  };

  return (
    <form aria-label="New Task Form" onSubmit={handleSubmit}>
      <input
        aria-label="Task Title Input"
        placeholder={props.placeholder}
        value={draftTitle}
        onChange={(e) => setDraftTitle(e.target.value)}
      />
      <button aria-label="Add Task Button" onClick={handleSubmit}>
        Add Task
      </button>
    </form>
  );
}
