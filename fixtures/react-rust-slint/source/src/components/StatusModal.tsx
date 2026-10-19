import React from 'react';

interface StatusModalProps {
  isOpen: boolean;
  message: string;
}

export function StatusModal(props: StatusModalProps) {
  return (
    <dialog aria-label="Status Dialog" open={props.isOpen}>
      <h2>Migration Contract Status</h2>
      <p>{props.message}</p>
      <progress aria-label="Contract Verification Progress" value={100} max={100} />
      <button aria-label="Re-run Verification Button" onClick={() => setVerificationRerun(true)}>
        Re-run Verification
      </button>
      <button aria-label="Export Audit Log Button" onClick={() => setAuditExported(true)}>
        Export Audit Log
      </button>
      <button aria-label="Close Modal" onClick={() => setModalOpen(false)}>
        Close
      </button>
    </dialog>
  );
}
