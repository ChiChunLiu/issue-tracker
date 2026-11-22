import { useEffect, useState, type FormEvent } from 'react';
import { api } from './api';
import { TagEditor } from './TagEditor';
import { ReportedAtPicker } from './ReportedAtPicker';
import appIcon from '../src-tauri/icons/128x128.png';
import { translate, type Language, type TranslationKey } from './i18n';
import { emptyFilters, type AiReportDraft, type AppSettings, type BackupStatus, type Platform, type Project, type Report, type ReportFilters, type ReportInput, type ReportSource, type ReportSummary, type Theme } from './types';

type View = 'reports' | 'new-choice' | 'ai-import' | 'new' | 'edit' | 'detail' | 'projects' | 'settings';
type Message = { text: string; error: boolean } | null;
type FilterDraft = Pick<ReportFilters, 'title' | 'source' | 'description' | 'note' | 'platform' | 'system_info' | 'app_version' | 'application_log' | 'tag'> & { from: string; to: string };

const blankFilterDraft = (): FilterDraft => ({
  title: '', source: '', description: '', note: '', platform: '', system_info: '', app_version: '', application_log: '', tag: '', from: '', to: ''
});

function toUtc(local: string): string {
  const date = new Date(local);
  if (Number.isNaN(date.getTime())) throw new Error('Invalid reported time.');
  return date.toISOString();
}
function toUtcEndOfMinute(local: string): string {
  const date = new Date(local);
  if (Number.isNaN(date.getTime())) throw new Error('Invalid reported time.');
  return new Date(date.getTime() + 59_999).toISOString();
}
function formatDate(iso: string, language: Language): string {
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return iso || '—';
  return new Intl.DateTimeFormat(language, { dateStyle: 'medium', timeStyle: 'short' }).format(date);
}
function TagChips({ tags, language }: { tags: string[]; language: Language }) {
  return <div className="row-tags">{tags.length
    ? tags.map(tag => <span className="tag-chip" key={tag}>{tag}</span>)
    : <span className="no-tags">{translate(language, 'noTags')}</span>}</div>;
}
function platformOptions(t: (key: TranslationKey) => string) {
  return <><option value="">{t('selectPlatform')}</option>
    <option>Linux</option><option>macOS</option><option>Windows</option></>;
}

interface ReportFormProps {
  initial: Report | null;
  draft: AiReportDraft | null;
  projectId: number;
  suggestions: string[];
  language: Language;
  onCancel: () => void;
  onSave: (input: ReportInput) => Promise<void>;
  onError: (text: string) => void;
}
function ReportForm({ initial, draft, projectId, suggestions, language, onCancel, onSave, onError }: ReportFormProps) {
  const t = (key: TranslationKey) => translate(language, key);
  const [form, setForm] = useState<ReportInput>(() => initial ? {
    project_id: projectId, title: initial.title, source: initial.source, description: initial.description, note: initial.note, platform: initial.platform,
    system_info: initial.system_info, app_version: initial.app_version,
    application_log: initial.application_log, reported_at: initial.reported_at, tags: [...initial.tags]
  } : draft ? { ...draft, project_id: projectId } : {
    project_id: projectId, title: '', source: '', description: '', note: '', platform: '' as Platform, system_info: '',
    app_version: '', application_log: '', reported_at: new Date().toISOString(), tags: []
  });
  const [pendingTag, setPendingTag] = useState('');
  const [availableTags, setAvailableTags] = useState(suggestions);
  const [saving, setSaving] = useState(false);
  useEffect(() => { api.listTags(form.project_id).then(setAvailableTags).catch(() => setAvailableTags([])); }, [form.project_id]);

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const tags = [...form.tags];
    for (const raw of pendingTag.split(',')) {
      const tag = raw.trim();
      if (!tag || tags.some(existing => existing.toLocaleLowerCase() === tag.toLocaleLowerCase())) continue;
      if ([...tag].length > 50) { onError(t('tagTooLong')); return; }
      tags.push(availableTags.find(existing => existing.toLocaleLowerCase() === tag.toLocaleLowerCase()) ?? tag);
    }
    if (tags.length > 30) { onError(t('tooManyTags')); return; }
    setSaving(true);
    try { await onSave({ ...form, title: form.title.trim(), tags }); }
    finally { setSaving(false); }
  }

  return <div>
    <button className="back" type="button" onClick={onCancel}>← {initial ? t('reportDetail') : draft ? t('aiImport') : t('newReport')}</button>
    <div className="page-head"><h1>{initial ? t('editReport') : t('newReport')}</h1></div>
    {draft && <p className="import-review">{t('reviewDraft')}</p>}
    <form className="form-panel" onSubmit={submit}><div className="fields">
      <div className="field"><label htmlFor="report-title">{t('title')}</label>
        <input id="report-title" required maxLength={500} placeholder={t('titlePlaceholder')} value={form.title}
          onChange={event => setForm({ ...form, title: event.target.value })} /></div>
      <div className="form-pair">
        <div className="field"><label htmlFor="report-platform">{t('platform')}</label>
          <select id="report-platform" required value={form.platform} onChange={event => setForm({ ...form, platform: event.target.value as Platform })}>
            {platformOptions(t)}</select></div>
        <div className="field"><label htmlFor="report-source">{t('source')}</label>
          <select id="report-source" value={form.source} onChange={event => setForm({ ...form, source: event.target.value as ReportSource })}>
            <option value="">{t('selectSource')}</option><option>Slack</option><option>Reddit</option><option>Mail</option><option>Zendesk</option>
          </select></div>
      </div>
      <div className="field"><label id="report-time-label">{t('reportedTime')}</label>
        <ReportedAtPicker value={form.reported_at} language={language} onChange={reported_at => setForm({ ...form, reported_at })} /></div>
      <div className="field"><label htmlFor="report-description">{t('description')}</label>
        <textarea id="report-description" value={form.description}
          onChange={event => setForm({ ...form, description: event.target.value })} /></div>
      <div className="field"><label htmlFor="report-system">{t('systemInfo')}</label>
        <textarea id="report-system" placeholder={t('systemPlaceholder')} value={form.system_info}
          onChange={event => setForm({ ...form, system_info: event.target.value })} /></div>
      <div className="field"><label htmlFor="report-version">{t('appVersion')}</label>
        <input id="report-version" placeholder={t('versionPlaceholder')} value={form.app_version}
          onChange={event => setForm({ ...form, app_version: event.target.value })} /></div>
      <div className="field"><label htmlFor="tags">{t('tags')}</label>
        <TagEditor language={language} value={form.tags} suggestions={availableTags}
          onChange={tags => setForm({ ...form, tags })} onPendingChange={setPendingTag} onError={onError} />
        <span className="field-hint">{t('tagHint')}</span></div>
      <div className="field"><label htmlFor="report-log">{t('applicationLog')}</label>
        <textarea className="log" id="report-log" placeholder={t('logPlaceholder')} value={form.application_log}
          onChange={event => setForm({ ...form, application_log: event.target.value })} /></div>
      <div className="field"><label htmlFor="report-note">{t('note')}</label>
        <textarea id="report-note" value={form.note}
          onChange={event => setForm({ ...form, note: event.target.value })} /></div>
    </div><div className="actions">
      <button className="secondary" type="button" onClick={onCancel}>{t('cancel')}</button>
      <button className="primary" type="submit" disabled={saving}>{initial ? t('saveChanges') : t('save')}</button>
    </div></form>
  </div>;
}

export function App() {
  const [sidebarCollapsed, setSidebarCollapsed] = useState(() => window.localStorage.getItem('sidebarCollapsed') === 'true');
  const [settings, setSettings] = useState<AppSettings | null>(null);
  const [projects, setProjects] = useState<Project[]>([]);
  const [view, setView] = useState<View>('reports');
  const [reports, setReports] = useState<ReportSummary[]>([]);
  const [page, setPage] = useState(1);
  const [selected, setSelected] = useState<Report | null>(null);
  const [importText, setImportText] = useState('');
  const [importedDraft, setImportedDraft] = useState<AiReportDraft | null>(null);
  const [importing, setImporting] = useState(false);
  const [hasAiToken, setHasAiToken] = useState<boolean | null>(null);
  const [tokenInput, setTokenInput] = useState('');
  const [savingToken, setSavingToken] = useState(false);
  const [aiProviderInput, setAiProviderInput] = useState<AppSettings['ai_provider']>('openai-compatible');
  const [aiEndpointInput, setAiEndpointInput] = useState('');
  const [aiModelInput, setAiModelInput] = useState('');
  const [savingAiConfig, setSavingAiConfig] = useState(false);
  const [settingsReturnView, setSettingsReturnView] = useState<View | null>(null);
  const [availableTags, setAvailableTags] = useState<string[]>([]);
  const [filters, setFilters] = useState<ReportFilters>(emptyFilters(0));
  const [filterDraft, setFilterDraft] = useState<FilterDraft>(blankFilterDraft());
  const [filterOpen, setFilterOpen] = useState(false);
  const [backupStatus, setBackupStatus] = useState<BackupStatus>({ last_synced_at: '', last_error: '' });
  const [message, setMessage] = useState<Message>(null);
  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState(false);
  const language = settings?.language ?? 'en';
  const t = (key: TranslationKey) => translate(language, key);
  const projectId = settings?.selected_project_id ?? 0;
  const aiConfigPending = settings !== null && (aiProviderInput !== settings.ai_provider ||
    aiEndpointInput.trim() !== (settings.ai_provider === 'anthropic' ? settings.anthropic_endpoint : settings.ai_endpoint));

  useEffect(() => {
    document.documentElement.dataset.theme = settings?.theme ?? 'light';
    document.documentElement.lang = language;
  }, [settings?.theme, language]);
  useEffect(() => { window.localStorage.setItem('sidebarCollapsed', String(sidebarCollapsed)); }, [sidebarCollapsed]);
  useEffect(() => {
    if (!message) return;
    const timer = window.setTimeout(() => setMessage(null), 5000);
    return () => window.clearTimeout(timer);
  }, [message]);
  useEffect(() => {
    let alive = true;
    Promise.all([api.getSettings(), api.listProjects(), api.getBackupStatus()])
      .then(([nextSettings, nextProjects, status]) => {
        if (!alive) return;
        setSettings(nextSettings);
        setProjects(nextProjects);
        setBackupStatus(status);
        setFilters(emptyFilters(nextSettings.selected_project_id));
      })
      .catch(error => { if (alive) { console.error(error); setLoadError(true); } })
      .finally(() => { if (alive) setLoading(false); });
    return () => { alive = false; };
  }, []);
  useEffect(() => {
    if (!settings) return;
    setAiProviderInput(settings.ai_provider);
    setAiEndpointInput(settings.ai_provider === 'anthropic' ? settings.anthropic_endpoint : settings.ai_endpoint);
    setAiModelInput(settings.ai_provider === 'anthropic' ? settings.anthropic_model : settings.ai_model);
  }, [settings?.ai_provider, settings?.ai_endpoint, settings?.ai_model, settings?.anthropic_endpoint, settings?.anthropic_model]);
  useEffect(() => {
    if (!settings) return;
    let alive = true;
    setHasAiToken(null);
    api.hasAiToken().then(value => { if (alive) setHasAiToken(value); })
      .catch(error => { if (alive) { setHasAiToken(false); showError(error); } });
    return () => { alive = false; };
  }, [settings?.ai_provider, settings?.ai_endpoint, settings?.anthropic_endpoint]);
  useEffect(() => {
    if (!filters.project_id) return;
    let alive = true;
    Promise.all([api.listReports(filters), api.listTags(filters.project_id)])
      .then(([nextReports, nextTags]) => { if (alive) { setReports(nextReports); setAvailableTags(nextTags); } })
      .catch(error => { if (alive) showError(error); });
    return () => { alive = false; };
  }, [filters]);
  useEffect(() => { setPage(1); }, [filters]);

  const pageCount = Math.max(1, Math.ceil(reports.length / 10));
  const currentPage = Math.min(page, pageCount);
  const visibleReports = reports.slice((currentPage - 1) * 10, currentPage * 10);

  function showError(error: unknown) {
    setMessage({ text: String(error), error: true });
  }
  function showMessage(text: string) { setMessage({ text, error: false }); }
  async function refresh(project = projectId, activeFilters = filters) {
    const [nextProjects, nextReports, nextTags, status] = await Promise.all([
      api.listProjects(), api.listReports({ ...activeFilters, project_id: project }),
      api.listTags(project), api.getBackupStatus()
    ]);
    setProjects(nextProjects);
    setReports(nextReports);
    setAvailableTags(nextTags);
    setBackupStatus(status);
  }
  async function selectProject(id: number) {
    if (!settings) return;
    try {
      await api.selectProject(id);
      setSettings({ ...settings, selected_project_id: id });
      setFilters(emptyFilters(id));
      setFilterDraft(blankFilterDraft());
      setView('reports');
      setSelected(null);
    } catch (error) { showError(error); }
  }
  async function saveReport(input: ReportInput) {
    try {
      const saved = selected && view === 'edit' ? await api.updateReport(selected.id, input) : await api.createReport(input);
      setSelected(saved);
      if (settings && input.project_id !== projectId) {
        await api.selectProject(input.project_id);
        setSettings({ ...settings, selected_project_id: input.project_id });
        setFilters(emptyFilters(input.project_id));
        setFilterDraft(blankFilterDraft());
      }
      await refresh(input.project_id, input.project_id === projectId ? filters : emptyFilters(input.project_id));
      setImportText('');
      setImportedDraft(null);
      setView('detail');
      showMessage(t('reportSaved'));
    } catch (error) { showError(error); throw error; }
  }
  async function deleteReport() {
    if (!selected || !window.confirm(t('deleteReportConfirm'))) return;
    try {
      await api.deleteReport(selected.id);
      setSelected(null);
      await refresh();
      setView('reports');
      showMessage(t('reportDeleted'));
    } catch (error) { showError(error); }
  }
  async function addSamples() {
    try {
      await api.addSamples(projectId);
      const cleared = emptyFilters(projectId);
      setFilters(cleared);
      setFilterDraft(blankFilterDraft());
      await refresh(projectId, cleared);
      showMessage(t('samplesAdded'));
    } catch (error) { showError(error); }
  }
  async function updateSettings(next: AppSettings) {
    try {
      const saved = await api.updateSettings(next);
      setSettings(saved);
      setBackupStatus(await api.getBackupStatus());
      showMessage(t('settingsSaved'));
    } catch (error) { showError(error); }
  }
  async function chooseFolder() {
    try {
      const folder = await api.chooseFolder();
      if (folder && settings) await updateSettings({ ...settings, backup_folder: folder });
    } catch (error) { showError(error); }
  }
  async function syncNow() {
    try { await api.syncBackup(); setBackupStatus(await api.getBackupStatus()); showMessage(t('backupSynced')); }
    catch (error) { setBackupStatus(await api.getBackupStatus()); showError(error); }
  }
  async function exportBackup() {
    try { if (await api.exportBackup()) showMessage(t('backupSaved')); }
    catch (error) { showError(error); }
  }
  async function generateDraft(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setImporting(true);
    try {
      const draft = await api.generateAiReport(importText);
      setImportedDraft(draft);
      setView('new');
    } catch (error) { showError(error); }
    finally { setImporting(false); }
  }
  async function saveAiConfig(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!settings) return;
    setSavingAiConfig(true);
    try {
      const saved = await api.updateSettings({
        ...settings,
        ai_provider: aiProviderInput,
        ...(aiProviderInput === 'anthropic'
          ? { anthropic_endpoint: aiEndpointInput.trim(), anthropic_model: aiModelInput.trim() }
          : { ai_endpoint: aiEndpointInput.trim(), ai_model: aiModelInput.trim() })
      });
      setSettings(saved);
      setBackupStatus(await api.getBackupStatus());
      showMessage(t('aiConfigSaved'));
    } catch (error) { showError(error); }
    finally { setSavingAiConfig(false); }
  }
  function chooseAiProvider(provider: AppSettings['ai_provider']) {
    if (!settings) return;
    setAiProviderInput(provider);
    setAiEndpointInput(provider === 'anthropic' ? settings.anthropic_endpoint : settings.ai_endpoint);
    setAiModelInput(provider === 'anthropic' ? settings.anthropic_model : settings.ai_model);
  }
  async function saveAiToken(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setSavingToken(true);
    try {
      await api.saveAiToken(tokenInput);
      setTokenInput('');
      setHasAiToken(true);
      showMessage(t('tokenSaved'));
    } catch (error) { showError(error); }
    finally { setSavingToken(false); }
  }
  async function removeAiToken() {
    try {
      await api.deleteAiToken();
      setTokenInput('');
      setHasAiToken(false);
      showMessage(t('tokenRemoved'));
    } catch (error) { showError(error); }
  }
  async function createProject(name: string) {
    try {
      const project = await api.createProject(name);
      await refresh();
      await selectProject(project.id);
      showMessage(t('projectCreated'));
    } catch (error) { showError(error); throw error; }
  }
  async function renameProject(id: number, name: string) {
    try { await api.renameProject(id, name); await refresh(); showMessage(t('projectRenamed')); }
    catch (error) { showError(error); throw error; }
  }
  async function deleteProject(id: number) {
    if (!window.confirm(t('deleteProjectConfirm'))) return;
    try {
      await api.deleteProject(id);
      const nextSettings = await api.getSettings();
      setSettings(nextSettings);
      setFilters(emptyFilters(nextSettings.selected_project_id));
      await refresh(nextSettings.selected_project_id, emptyFilters(nextSettings.selected_project_id));
      showMessage(t('projectDeleted'));
    } catch (error) { showError(error); }
  }

  if (loading) return <div className="loading-screen">{t('loading')}</div>;
  if (loadError || !settings) return <div className="loading-screen"><h1>{t('fallbackTitle')}</h1><p>{t('fallbackBody')}</p></div>;

  return <div className={'app' + (sidebarCollapsed ? ' sidebar-collapsed' : '')}>
    <aside className="sidebar">
      <div className="brand"><img className="brand-mark" src={appIcon} alt="" /><span className="brand-name">{t('appName')}</span>
        <button className="sidebar-toggle" type="button" aria-label={t(sidebarCollapsed ? 'expandSidebar' : 'collapseSidebar')}
          title={t(sidebarCollapsed ? 'expandSidebar' : 'collapseSidebar')} onClick={() => setSidebarCollapsed(!sidebarCollapsed)}>{sidebarCollapsed ? '›' : '‹'}</button></div>
      <div className="project-selector">
        <div className="project-picker"><select id="current-project" aria-label={t('projects')} value={projectId} onChange={event => void selectProject(Number(event.target.value))}>
          {projects.map(project => <option key={project.id} value={project.id}>{project.name}</option>)}
        </select><button className="project-add" type="button" title={t('manageProjects')} aria-label={t('newProject')}
          onClick={() => setView('projects')}>+</button></div></div>
      <nav className="main-nav" aria-label={t('appName')}>
        <button className={'nav-button ' + (['reports', 'new-choice', 'ai-import', 'new', 'edit', 'detail'].includes(view) ? 'active' : '')}
          aria-label={t('reports')} title={sidebarCollapsed ? t('reports') : undefined}
          onClick={() => setView('reports')}>▤ <span>{t('reports')}</span></button>
      </nav>
      <button className={'settings-button nav-button ' + (view === 'settings' ? 'active' : '')}
        aria-label={t('settings')} title={sidebarCollapsed ? t('settings') : undefined}
        onClick={() => { setSettingsReturnView(null); setView('settings'); }}>⚙ <span>{t('settings')}</span></button>
    </aside>
    <div className="content"><main className="page">
      {view === 'reports' && <div>
        <div className="page-head"><h1>{t('reports')}</h1><div className="page-actions">
          <button className="secondary sample-button" onClick={() => void addSamples()}>{t('addSamples')}</button>
          <button className="primary" onClick={() => { setSelected(null); setImportedDraft(null); setImportText(''); setView('new-choice'); }}>{'+ ' + t('newReport')}</button>
        </div></div>
        <details className="filter-panel" open={filterOpen} onToggle={event => setFilterOpen(event.currentTarget.open)}>
          <summary>{t('filterReports')} {Object.entries(filters).some(([key, value]) => key !== 'project_id' && key !== 'oldest_first' && Boolean(value)) && <span className="filter-active">{t('active')}</span>}</summary>
          <form onSubmit={event => {
            event.preventDefault();
            try {
              const next: ReportFilters = { ...filters, ...filterDraft,
                reported_from: filterDraft.from ? toUtc(filterDraft.from) : '',
                reported_to: filterDraft.to ? toUtcEndOfMinute(filterDraft.to) : '' };
              if (next.reported_from && next.reported_to && next.reported_from > next.reported_to) throw new Error(t('fromBeforeTo'));
              setFilters(next);
            } catch (error) { showError(error); }
          }}>
            <div className="filter-grid">
              {([
                ['title', 'title', 'searchTitle'], ['system_info', 'systemInfo', 'searchSystem'],
                ['app_version', 'appVersion', 'searchVersion'], ['application_log', 'applicationLog', 'searchLog'],
                ['description', 'description', 'description'], ['note', 'note', 'note']
              ] as const).map(([field, label, placeholder]) => <div className="field" key={field}>
                <label htmlFor={'filter-' + field}>{t(label)}</label>
                <input id={'filter-' + field} value={filterDraft[field]} placeholder={t(placeholder)}
                  onChange={event => setFilterDraft({ ...filterDraft, [field]: event.target.value })} /></div>)}
              <div className="field"><label htmlFor="filter-platform">{t('platform')}</label>
                <select id="filter-platform" value={filterDraft.platform} onChange={event => setFilterDraft({ ...filterDraft, platform: event.target.value })}>
                  <option value="">{t('allPlatforms')}</option><option>Linux</option><option>macOS</option><option>Windows</option></select></div>
              <div className="field"><label htmlFor="filter-source">{t('source')}</label>
                <select id="filter-source" value={filterDraft.source} onChange={event => setFilterDraft({ ...filterDraft, source: event.target.value })}>
                  <option value="">{t('selectSource')}</option><option>Slack</option><option>Reddit</option><option>Mail</option><option>Zendesk</option></select></div>
              <div className="field"><label htmlFor="filter-tag">{t('tag')}</label>
                <select id="filter-tag" value={filterDraft.tag} onChange={event => setFilterDraft({ ...filterDraft, tag: event.target.value })}>
                  <option value="">{t('allTags')}</option>{availableTags.map(tag => <option key={tag}>{tag}</option>)}</select></div>
              <div className="field"><label htmlFor="filter-from">{t('reportedFrom')}</label>
                <input id="filter-from" type="datetime-local" value={filterDraft.from} onChange={event => setFilterDraft({ ...filterDraft, from: event.target.value })} /></div>
              <div className="field"><label htmlFor="filter-to">{t('reportedTo')}</label>
                <input id="filter-to" type="datetime-local" value={filterDraft.to} onChange={event => setFilterDraft({ ...filterDraft, to: event.target.value })} /></div>
            </div><div className="filter-actions">
              <button className="secondary" type="button" onClick={() => { setFilterDraft(blankFilterDraft()); setFilters(emptyFilters(projectId)); }}>{t('clearFilters')}</button>
              <button className="primary" type="submit">{t('applyFilters')}</button>
            </div></form>
        </details>
        {reports.length ? <div className="panel report-table" role="table" aria-label={t('reports')}>
          <div className="table-head" role="row">
            <span role="columnheader">{t('title')}</span><span role="columnheader">{t('platform')}</span>
            <span role="columnheader">{t('appVersion')}</span>
            <span role="columnheader" aria-sort={filters.oldest_first ? 'ascending' : 'descending'}>
              <button className="time-sort" type="button" onClick={() => setFilters({ ...filters, oldest_first: !filters.oldest_first })}
                aria-label={t('reportedTime') + ', ' + t(filters.oldest_first ? 'sortOldest' : 'sortNewest')}>
                {t('reportedTime')} <span className="sort-arrows" aria-hidden="true"><span className={filters.oldest_first ? 'active' : ''}>↑</span><span className={filters.oldest_first ? '' : 'active'}>↓</span></span>
              </button></span><span role="columnheader" />
          </div>
          {visibleReports.map(report => <div className="report-row" role="row" key={report.id}>
            <div className="title-cell" role="cell"><button className="report-title" onClick={() => void api.getReport(report.id).then(item => { setSelected(item); setView('detail'); }).catch(showError)}>{report.title}</button>
              <TagChips tags={report.tags} language={language} /></div>
            <span className="platform-cell" role="cell">{report.platform || '—'}</span>
            <span className="version" role="cell">{report.app_version || '—'}</span>
            <span className="time-cell" role="cell">{formatDate(report.reported_at, language)}</span>
            <span className="view-cell" role="cell"><button className="view-button" onClick={() => void api.getReport(report.id).then(item => { setSelected(item); setView('detail'); }).catch(showError)}>{t('view')}</button></span>
          </div>)}
        </div> : <div className="panel empty"><h2>{Object.entries(filters).some(([key, value]) => key !== 'project_id' && key !== 'oldest_first' && Boolean(value)) ? t('noMatches') : t('noReports')}</h2></div>}
        {reports.length > 0 && <div className="pagination"><span>{t('showing')} {(currentPage - 1) * 10 + 1}–{Math.min(currentPage * 10, reports.length)} {t('of')} {reports.length}</span>
          <div><button className="secondary" type="button" disabled={currentPage === 1} onClick={() => setPage(currentPage - 1)}>{t('previous')}</button>
            <span>{currentPage} / {pageCount}</span>
            <button className="secondary" type="button" disabled={currentPage === pageCount} onClick={() => setPage(currentPage + 1)}>{t('next')}</button></div></div>}
      </div>}
      {view === 'new-choice' && <div>
        <button className="back" onClick={() => setView('reports')}>← {t('reports')}</button>
        <div className="page-head"><h1>{t('newReport')}</h1></div>
        <div className="creation-options">
          <button type="button" onClick={() => { setImportedDraft(null); setView('new'); }}>
            <strong>{t('form')}</strong><span>{t('formChoice')}</span>
          </button>
          <button type="button" onClick={() => setView('ai-import')}>
            <strong>{t('aiImport')}</strong><span>{t('aiImportChoice')}</span>
          </button>
        </div>
      </div>}
      {view === 'ai-import' && <div>
        <button className="back" onClick={() => setView('new-choice')}>← {t('newReport')}</button>
        <div className="page-head"><h1>{t('aiImport')}</h1></div>
        <form className="form-panel ai-import-panel" onSubmit={event => void generateDraft(event)}>
          <div className="field"><label htmlFor="import-text">{t('pasteReport')}</label>
            <textarea id="import-text" className="import-text" required maxLength={100000} autoFocus
              placeholder={t('pasteReportPlaceholder')} value={importText} onChange={event => setImportText(event.target.value)} /></div>
          <p className="field-hint">{t('aiPrivacy')}</p>
          {hasAiToken === false && <div className="import-key-notice"><span>{settings.ai_provider === 'anthropic' ? t('anthropicTokenRequired') : t('tokenOptional')}</span>
            <button className="secondary" type="button" onClick={() => { setSettingsReturnView('ai-import'); setView('settings'); }}>{t('openSettings')}</button></div>}
          <div className="actions"><button className="secondary" type="button" onClick={() => setView('new-choice')}>{t('cancel')}</button>
            <button className="primary" type="submit" disabled={importing || !importText.trim() || (settings.ai_provider === 'anthropic' && hasAiToken !== true)}>
              {importing ? t('generatingDraft') : t('generateDraft')}
            </button></div>
        </form>
      </div>}
      {(view === 'new' || view === 'edit') && <ReportForm key={view + ':' + (selected?.id ?? projectId)} initial={view === 'edit' ? selected : null}
        draft={view === 'new' ? importedDraft : null}
        projectId={projectId} suggestions={availableTags} language={language}
        onCancel={() => setView(view === 'edit' ? 'detail' : importedDraft ? 'ai-import' : 'new-choice')} onSave={saveReport}
        onError={text => setMessage({ text, error: true })} />}
      {view === 'detail' && selected && <div>
        <button className="back" onClick={() => setView('reports')}>← {t('reports')}</button>
        <div className="page-head"><div><h1>{selected.title}</h1><TagChips tags={selected.tags} language={language} /></div>
          <div className="detail-actions"><button className="secondary" onClick={() => setView('edit')}>{t('edit')}</button>
            <button className="danger" onClick={() => void deleteReport()}>{t('delete')}</button></div></div>
        <div className="detail-panel">
          {([
            [t('title'), selected.title], [t('projects'), projects.find(project => project.id === selected.project_id)?.name ?? '—'],
            [t('platform'), selected.platform || '—'], [t('source'), selected.source || '—'],
            [t('reportedTime'), formatDate(selected.reported_at, language)], [t('description'), selected.description || '—'],
            [t('appVersion'), selected.app_version || '—'], [t('systemInfo'), selected.system_info || '—']
          ] as const).map(([label, value]) => <div className="detail-block" key={label}><span className="detail-label">{label}</span><p className="detail-value">{value}</p></div>)}
          <div className="detail-block"><span className="detail-label">{t('tags')}</span><TagChips tags={selected.tags} language={language} /></div>
          <div className="detail-block"><span className="detail-label">{t('applicationLog')}</span><pre className="detail-log">{selected.application_log || '—'}</pre></div>
          <div className="detail-block"><span className="detail-label">{t('note')}</span><p className="detail-value">{selected.note || '—'}</p></div>
        </div>
      </div>}
      {view === 'projects' && <ProjectsView projects={projects} language={language} selectedId={projectId}
        onCreate={createProject} onRename={renameProject} onDelete={deleteProject} onSelect={selectProject} />}
      {view === 'settings' && <div>
        {settingsReturnView && <button className="back" type="button" onClick={() => { setView(settingsReturnView); setSettingsReturnView(null); }}>← {t('aiImport')}</button>}
        <div className="page-head"><h1>{t('settings')}</h1></div>
        <div className="settings-panel">
          <div className="setting-row"><div><h2>{t('theme')}</h2></div><div className="segmented">
            {(['light', 'dark'] as Theme[]).map(theme => <button key={theme} className={settings.theme === theme ? 'selected' : ''}
              onClick={() => void updateSettings({ ...settings, theme })}>{t(theme)}</button>)}</div></div>
          <div className="setting-row"><div><h2>{t('language')}</h2></div><select value={settings.language}
            onChange={event => void updateSettings({ ...settings, language: event.target.value as Language })}>
            <option value="en">{t('english')}</option><option value="zh-TW">{t('traditionalChinese')}</option></select></div>
          <div className="setting-row ai-key-row"><div><h2>{t('aiProvider')}</h2>
            <p className="setting-value">{t(aiProviderInput === 'anthropic' ? 'anthropicEndpointHint' : 'aiEndpointHint')}</p></div>
            <form className="ai-config-form" onSubmit={event => void saveAiConfig(event)}>
              <label htmlFor="ai-provider">{t('aiProvider')}</label>
              <select id="ai-provider" value={aiProviderInput}
                onChange={event => chooseAiProvider(event.target.value as AppSettings['ai_provider'])}>
                <option value="openai-compatible">{t('openAiCompatible')}</option>
                <option value="anthropic">Anthropic</option>
              </select>
              <label htmlFor="ai-endpoint">{t('aiEndpoint')}</label>
              <input id="ai-endpoint" type="url" required value={aiEndpointInput}
                placeholder={aiProviderInput === 'anthropic' ? 'https://api.anthropic.com/v1/messages' : 'https://api.together.ai/v1/chat/completions'}
                onChange={event => setAiEndpointInput(event.target.value)} />
              <label htmlFor="ai-model">{t('aiModel')}</label>
              <input id="ai-model" required value={aiModelInput} placeholder={aiProviderInput === 'anthropic' ? 'claude-sonnet-4-6' : 'zai-org/GLM-5.3'}
                onChange={event => setAiModelInput(event.target.value)} />
              <div><button className="primary" type="submit" disabled={savingAiConfig}>{t('saveAiConfig')}</button></div>
            </form></div>
          <div className="setting-row ai-key-row"><div><h2>{t('aiToken')}</h2>
            <p className="setting-value">{aiConfigPending ? t('saveEndpointFirst') : hasAiToken === null ? t('loading') : hasAiToken ? t('tokenStored') : t('tokenMissing')}</p></div>
            <form className="api-key-form" onSubmit={event => void saveAiToken(event)}>
              <input type="password" value={tokenInput} onChange={event => setTokenInput(event.target.value)}
                aria-label={t('aiToken')} placeholder={t('tokenPlaceholder')} autoComplete="new-password" spellCheck={false} />
              <div><button className="primary" type="submit" disabled={savingToken || !tokenInput.trim() || aiConfigPending}>{t('saveToken')}</button>
                {hasAiToken && <button className="secondary" type="button" disabled={aiConfigPending} onClick={() => void removeAiToken()}>{t('removeToken')}</button>}</div>
            </form></div>
          <div className="setting-row folder-row"><div><h2>{t('backupFolder')}</h2>
            <p className="setting-value">{settings.backup_folder || t('noFolder')}</p>
            {settings.backup_folder && <p className="setting-hint">{t('syncReady')}</p>}
            {backupStatus.last_error && <p className="setting-error">{t('backupFailed')} {backupStatus.last_error}</p>}
            {backupStatus.last_synced_at && !backupStatus.last_error && <p className="setting-hint">{t('backupSynced')} {formatDate(backupStatus.last_synced_at, language)}</p>}
          </div><div className="setting-actions">
            <button className="secondary" onClick={() => void chooseFolder()}>{t('chooseFolder')}</button>
            {settings.backup_folder && <><button className="secondary" onClick={() => void syncNow()}>{t('syncNow')}</button>
              <button className="secondary" onClick={() => void updateSettings({ ...settings, backup_folder: '' })}>{t('removeFolder')}</button></>}
          </div></div>
          <div className="setting-row export-row"><button className="secondary" onClick={() => void exportBackup()}>{t('exportBackup')}</button></div>
        </div>
      </div>}
    </main></div>
    {message && <div className={'message' + (message.error ? ' error' : '')} role="status">{message.text}</div>}
  </div>;
}

interface ProjectsViewProps {
  projects: Project[];
  selectedId: number;
  language: Language;
  onCreate: (name: string) => Promise<void>;
  onRename: (id: number, name: string) => Promise<void>;
  onDelete: (id: number) => Promise<void>;
  onSelect: (id: number) => Promise<void>;
}
function ProjectsView({ projects, selectedId, language, onCreate, onRename, onDelete, onSelect }: ProjectsViewProps) {
  const t = (key: TranslationKey) => translate(language, key);
  const [name, setName] = useState('');
  const [renaming, setRenaming] = useState<number | null>(null);
  const [renameValue, setRenameValue] = useState('');
  return <div><div className="page-head"><h1>{t('projects')}</h1></div>
    <form className="project-create" onSubmit={event => {
      event.preventDefault();
      if (!name.trim()) return;
      void onCreate(name.trim()).then(() => setName('')).catch(() => {});
    }}><label htmlFor="project-name">{t('newProject')}</label><div>
      <input id="project-name" value={name} maxLength={100} autoFocus placeholder={t('projectName')} onChange={event => setName(event.target.value)} />
      <button className="primary" type="submit">{t('newProject')}</button>
    </div></form>
    <div className="project-list">{projects.map(project => <div className="project-row" key={project.id}>
      {renaming === project.id ? <form className="rename-form" onSubmit={event => {
        event.preventDefault();
        void onRename(project.id, renameValue).then(() => setRenaming(null)).catch(() => {});
      }}><input value={renameValue} autoFocus maxLength={100} onChange={event => setRenameValue(event.target.value)} />
        <button className="primary" type="submit">{t('saveChanges')}</button>
        <button className="secondary" type="button" onClick={() => setRenaming(null)}>{t('cancel')}</button></form>
        : <><div className="project-info"><strong>{project.name}</strong><span>{project.report_count} {t('projectReports')}</span></div>
          <div className="project-actions">
            {project.id !== selectedId && <button className="secondary" onClick={() => void onSelect(project.id)}>{t('selectProject')}</button>}
            <button className="secondary" onClick={() => { setRenaming(project.id); setRenameValue(project.name); }}>{t('rename')}</button>
            <button className="danger" disabled={project.report_count > 0 || projects.length < 2}
              title={project.report_count > 0 ? t('cannotDeleteProject') : ''} onClick={() => void onDelete(project.id)}>{t('delete')}</button>
          </div></>}
    </div>)}</div>
  </div>;
}
