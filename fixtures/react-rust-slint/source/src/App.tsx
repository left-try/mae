import React, { useState, useReducer, useEffect } from 'react';
import { Route } from 'react-router-dom';
import { NavBar } from './components/NavBar';
import { TaskForm } from './components/TaskForm';
import { FilterBar } from './components/FilterBar';
import { StatusModal } from './components/StatusModal';
import { NotificationsDrawer } from './components/NotificationsDrawer';
import { DashboardView } from './views/DashboardView';
import { TasksView } from './views/TasksView';
import { KanbanBoardView } from './views/KanbanBoardView';
import { AnalyticsView } from './views/AnalyticsView';
import { SettingsView } from './views/SettingsView';
import './styles/theme.css';

export function App() {
  const [activeRoute, setActiveRoute] = useState<string>('/');
  const [filterQuery, setFilterQuery] = useState<string>('');
  const [modalOpen, setModalOpen] = useState<boolean>(false);
  const [priorityFilter, setPriorityFilter] = useState<string>('all');
  const [wipLimit, setWipLimit] = useState<number>(5);
  const [compactMode, setCompactMode] = useState<boolean>(false);
  const [notificationsMuted, setNotificationsMuted] = useState<boolean>(false);
  const [timeRange, setTimeRange] = useState<string>('30d');
  const [taskState, dispatch] = useReducer(taskReducer, initialTaskState);

  useEffect(() => {
    const savedTheme = window.localStorage.getItem('mae-desktop-theme');
    fetch('/api/tasks').then(() => {
      dispatch({ type: 'LOADED', theme: savedTheme });
    });
  }, [activeRoute]);

  return (
    <main className="app-shell">
      <NavBar activeRoute={activeRoute} title="Desktop Productivity Hub" />
      <FilterBar filterQuery={filterQuery} />
      <TaskForm placeholder="Enter desktop task..." />
      <StatusModal isOpen={modalOpen} message="All contracts verified" />
      <NotificationsDrawer unreadCount={3} muted={notificationsMuted} />
      <Route path="/" element={<DashboardView completionPercent={78} activeSprint="Sprint 14" />} />
      <Route path="/tasks" element={<TasksView totalTasks={12} selectedCount={2} />} />
      <Route path="/kanban" element={<KanbanBoardView wipLimit={wipLimit} inProgressCount={4} />} />
      <Route path="/analytics" element={<AnalyticsView timeRange={timeRange} velocityScore={92} />} />
      <Route path="/notifications" element={<NotificationsDrawer unreadCount={3} muted={notificationsMuted} />} />
      <Route path="/settings" element={<SettingsView themeMode="dark-slate" compactMode={compactMode} />} />
    </main>
  );
}
