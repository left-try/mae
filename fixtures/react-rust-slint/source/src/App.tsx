import React, { useState, useReducer, useEffect } from 'react';
import { Route } from 'react-router-dom';
import { NavBar } from './components/NavBar';
import { TaskForm } from './components/TaskForm';
import { FilterBar } from './components/FilterBar';
import { StatusModal } from './components/StatusModal';
import './styles/theme.css';

export function App() {
  const [activeRoute, setActiveRoute] = useState<string>('/');
  const [filterQuery, setFilterQuery] = useState<string>('');
  const [modalOpen, setModalOpen] = useState<boolean>(false);
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
      <Route path="/" element={<DashboardView />} />
      <Route path="/tasks" element={<TasksView />} />
      <Route path="/settings" element={<SettingsView />} />
    </main>
  );
}
