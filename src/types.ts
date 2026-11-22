import type { Language } from './i18n';

export type Platform = 'Linux' | 'macOS' | 'Windows';
export type ReportSource = 'Slack' | 'Reddit' | 'Mail' | 'Zendesk' | '';
export type Theme = 'light' | 'dark';

export interface Project {
  id: number;
  name: string;
  report_count: number;
}

export interface ReportSummary {
  id: number;
  project_id: number;
  title: string;
  platform: Platform | '';
  app_version: string;
  reported_at: string;
  tags: string[];
}

export interface Report extends ReportSummary {
  source: ReportSource;
  description: string;
  note: string;
  system_info: string;
  application_log: string;
  created_at: string;
  updated_at: string;
}

export type ReportInput = Pick<Report, 'project_id' | 'title' | 'source' | 'description' | 'note' | 'platform' | 'system_info' | 'app_version' | 'application_log' | 'reported_at' | 'tags'>;

export interface ReportFilters {
  project_id: number;
  title: string;
  source: string;
  description: string;
  note: string;
  platform: string;
  system_info: string;
  app_version: string;
  application_log: string;
  tag: string;
  reported_from: string;
  reported_to: string;
  oldest_first: boolean;
}

export interface AppSettings {
  theme: Theme;
  language: Language;
  backup_folder: string;
  selected_project_id: number;
}

export interface BackupStatus {
  last_synced_at: string;
  last_error: string;
}

export function emptyFilters(projectId: number): ReportFilters {
  return {
    project_id: projectId, title: '', source: '', description: '', note: '', platform: '', system_info: '', app_version: '',
    application_log: '', tag: '', reported_from: '', reported_to: '', oldest_first: false
  };
}
