import React from 'react';

export interface NotificationsDrawerProps {
  unreadCount: number;
  muted: boolean;
}

export function NotificationsDrawer(props: NotificationsDrawerProps) {
  return (
    <aside aria-label="Notifications Drawer">
      <h2>System Notifications</h2>
      <span role="status" aria-label="Unread Notifications Badge">
        {props.unreadCount} Unread
      </span>
      <select aria-label="Notification Severity Select" onChange={(e) => setSeverityFilter(e.target.value)}>
        <option value="all">All Severities</option>
        <option value="critical">Critical Only</option>
        <option value="info">Info Only</option>
      </select>
      <input
        type="checkbox"
        aria-label="Mute Desktop Alerts"
        checked={props.muted}
        onChange={() => setNotificationsMuted(!props.muted)}
      />
      <button aria-label="Mark All Read Button" onClick={() => setUnreadCount(0)}>
        Mark All Read
      </button>
      <button aria-label="Clear Dismissed Alerts Button" onClick={() => setDismissedCleared(true)}>
        Clear Dismissed
      </button>
    </aside>
  );
}
