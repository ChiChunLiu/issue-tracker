import { useEffect, useRef, useState } from 'react';
import { translate, type Language } from './i18n';

interface Props {
  value: string;
  language: Language;
  onChange: (value: string) => void;
}

export function ReportedAtPicker({ value, language, onChange }: Props) {
  const t = (key: Parameters<typeof translate>[1]) => translate(language, key);
  const selected = new Date(value);
  const date = Number.isNaN(selected.getTime()) ? new Date() : selected;
  const [open, setOpen] = useState(false);
  const [placement, setPlacement] = useState<'above' | 'below' | 'floating'>('below');
  const [month, setMonth] = useState(() => new Date(date.getFullYear(), date.getMonth(), 1));
  const root = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    const closeOutside = (event: PointerEvent) => {
      if (!root.current?.contains(event.target as Node)) setOpen(false);
    };
    const closeEscape = (event: KeyboardEvent) => {
      if (event.key === 'Escape') setOpen(false);
    };
    document.addEventListener('pointerdown', closeOutside);
    document.addEventListener('keydown', closeEscape);
    return () => {
      document.removeEventListener('pointerdown', closeOutside);
      document.removeEventListener('keydown', closeEscape);
    };
  }, [open]);

  const update = (year: number, monthIndex: number, day: number, hour = date.getHours(), minute = date.getMinutes()) => {
    onChange(new Date(year, monthIndex, day, hour, minute).toISOString());
  };
  const firstWeekday = new Date(month.getFullYear(), month.getMonth(), 1).getDay();
  const days = Array.from({ length: 42 }, (_, index) =>
    new Date(month.getFullYear(), month.getMonth(), index - firstWeekday + 1));
  const weekdays = Array.from({ length: 7 }, (_, index) =>
    new Intl.DateTimeFormat(language, { weekday: 'short' }).format(new Date(2023, 0, index + 1)));
  const numberOptions = (count: number) => Array.from({ length: count }, (_, number) =>
    <option key={number} value={number}>{String(number).padStart(2, '0')}</option>);

  return <div className="datetime-picker" ref={root}>
    <button className="datetime-trigger" type="button" aria-haspopup="dialog" aria-expanded={open}
      onClick={() => {
        const rect = root.current?.getBoundingClientRect();
        if (rect) {
          const below = window.innerHeight - rect.bottom;
          setPlacement(below >= 470 ? 'below' : rect.top >= 470 ? 'above' : 'floating');
        }
        setMonth(new Date(date.getFullYear(), date.getMonth(), 1));
        setOpen(!open);
      }}>
      <span>{new Intl.DateTimeFormat(language, { dateStyle: 'medium', timeStyle: 'short' }).format(date)}</span>
      <span className="datetime-icon" aria-hidden="true">▦</span>
    </button>
    {open && <div className={'calendar-popover ' + placement} role="dialog" aria-label={t('reportedTime')}>
      <div className="calendar-heading">
        <button type="button" aria-label={t('previousMonth')} onClick={() => setMonth(new Date(month.getFullYear(), month.getMonth() - 1, 1))}>‹</button>
        <strong>{new Intl.DateTimeFormat(language, { month: 'long', year: 'numeric' }).format(month)}</strong>
        <button type="button" aria-label={t('nextMonth')} onClick={() => setMonth(new Date(month.getFullYear(), month.getMonth() + 1, 1))}>›</button>
      </div>
      <div className="calendar-grid">
        {weekdays.map((weekday, index) => <span className="calendar-weekday" key={index}>{weekday}</span>)}
        {days.map(day => {
          const isSelected = day.getFullYear() === date.getFullYear() && day.getMonth() === date.getMonth() && day.getDate() === date.getDate();
          return <button type="button" key={day.toISOString()} aria-label={new Intl.DateTimeFormat(language, { dateStyle: 'full' }).format(day)}
            aria-pressed={isSelected} className={'calendar-day' + (day.getMonth() !== month.getMonth() ? ' outside' : '') + (isSelected ? ' selected' : '')}
            onClick={() => { update(day.getFullYear(), day.getMonth(), day.getDate()); setMonth(new Date(day.getFullYear(), day.getMonth(), 1)); }}>
            {day.getDate()}
          </button>;
        })}
      </div>
      <div className="calendar-time"><span>{t('time')}</span><div>
        <select aria-label={t('time') + ' hours'} value={date.getHours()} onChange={event => update(date.getFullYear(), date.getMonth(), date.getDate(), Number(event.target.value))}>{numberOptions(24)}</select>
        <span>:</span>
        <select aria-label={t('time') + ' minutes'} value={date.getMinutes()} onChange={event => update(date.getFullYear(), date.getMonth(), date.getDate(), date.getHours(), Number(event.target.value))}>{numberOptions(60)}</select>
      </div></div>
      <div className="calendar-actions"><button type="button" onClick={() => { const now = new Date(); onChange(now.toISOString()); setMonth(new Date(now.getFullYear(), now.getMonth(), 1)); }}>{t('today')}</button>
        <button type="button" onClick={() => setOpen(false)}>{t('done')}</button></div>
    </div>}
  </div>;
}
