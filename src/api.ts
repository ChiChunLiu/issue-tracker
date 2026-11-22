import { invoke } from '@tauri-apps/api/core';
import { open, save } from '@tauri-apps/plugin-dialog';
import type { AiReportDraft, AppSettings, BackupStatus, Issue, IssueInput, Project, Report, ReportFilters, ReportInput, ReportSummary } from './types';

export const api = {
  listProjects: () => invoke<Project[]>('list_projects'),
  createProject: (name: string) => invoke<Project>('create_project', { name }),
  renameProject: (id: number, name: string) => invoke<Project>('rename_project', { id, name }),
  deleteProject: (id: number) => invoke<void>('delete_project', { id }),
  selectProject: (id: number) => invoke<void>('select_project', { id }),
  listReports: (filters: ReportFilters) => invoke<ReportSummary[]>('list_reports', { filters }),
  listIssues: (projectId: number) => invoke<Issue[]>('list_issues', { projectId }),
  getIssue: (id: number) => invoke<Issue>('get_issue', { id }),
  createIssue: (input: IssueInput) => invoke<Issue>('create_issue', { input }),
  updateIssue: (id: number, input: IssueInput) => invoke<Issue>('update_issue', { id, input }),
  deleteIssue: (id: number) => invoke<void>('delete_issue', { id }),
  linkReportIssue: (reportId: number, issueId: number | null) => invoke<Report>('link_report_issue', { reportId, issueId }),
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
  hasAiToken: () => invoke<boolean>('has_ai_token'),
  saveAiToken: (token: string) => invoke<void>('save_ai_token', { token }),
  deleteAiToken: () => invoke<void>('delete_ai_token'),
  generateAiReport: (text: string) => invoke<AiReportDraft>('generate_ai_report', { text }),
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
