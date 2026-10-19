import React from 'react';

interface StatusModalProps {
  isOpen: boolean;
  message: string;
}

export function StatusModal(props: StatusModalProps) {
  return (
    <dialog aria-label="Status Dialog" open={props.isOpen}>
      <p>{props.message}</p>
      <button aria-label="Close Modal" onClick={() => setModalOpen(false)}>
        Close
      </button>
    </dialog>
  );
}
