import { invoke } from '@tauri-apps/api/core';
import { open, save } from '@tauri-apps/plugin-dialog';
import type { AppSettings, BackupStatus, Project, Report, ReportFilters, ReportInput, ReportSummary } from './types';

export const api = {
  listProjects: () => invoke<Project[]>('list_projects'),
  createProject: (name: string) => invoke<Project>('create_project', { name }),
  renameProject: (id: number, name: string) => invoke<Project>('rename_project', { id, name }),
  deleteProject: (id: number) => invoke<void>('delete_project', { id }),
  selectProject: (id: number) => invoke<void>('select_project', { id }),
  listReports: (filters: ReportFilters) => invoke<ReportSummary[]>('list_reports', { filters }),
  listTags: (projectId: number) => invoke<string[]>('list_tags', { projectId }),
  getReport: (id: number) => invoke<Report>('get_report', { id }),
  createReport: (input: ReportInput) => invoke<Report>('create_report', { input }),
  updateReport: (id: number, input: ReportInput) => invoke<Report>('update_report', { id, input }),
  deleteReport: (id: number) => invoke<void>('delete_report', { id }),
  addSamples: (projectId: number) => invoke<number>('seed_sample_reports', { projectId }),
  getSettings: () => invoke<AppSettings>('get_settings'),
  updateSettings: (settings: AppSettings) => invoke<AppSettings>('update_settings', { settings }),
  getBackupStatus: () => invoke<BackupStatus>('get_backup_status'),
  syncBackup: () => invoke<void>('sync_backup_now'),
  chooseFolder: async () => {
    const result = await open({ directory: true, multiple: false });
    return typeof result === 'string' ? result : null;
  },
  exportBackup: async () => {
    const path = await save({ defaultPath: 'issue-tracker-backup.db', filters: [{ name: 'SQLite database', extensions: ['db'] }] });
    if (path) await invoke<void>('backup_database', { path });
    return Boolean(path);
  }
};
