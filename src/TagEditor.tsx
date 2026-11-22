import { useMemo, useState } from 'react';
import { translate, type Language } from './i18n';

interface Props {
  language: Language;
  value: string[];
  suggestions: string[];
  onChange: (tags: string[]) => void;
  onPendingChange: (value: string) => void;
  onError: (message: string) => void;
}

export function TagEditor({ language, value, suggestions, onChange, onPendingChange, onError }: Props) {
  const [pending, setPending] = useState('');
  const [open, setOpen] = useState(false);
  const [active, setActive] = useState(-1);
  const t = (key: Parameters<typeof translate>[1]) => translate(language, key);
  const matches = useMemo(() => suggestions
    .filter(tag => !value.some(selected => selected.toLocaleLowerCase() === tag.toLocaleLowerCase()))
    .filter(tag => !pending || tag.toLocaleLowerCase().includes(pending.toLocaleLowerCase()))
    .slice(0, 8), [suggestions, value, pending]);

  function changePending(next: string) {
    setPending(next);
    onPendingChange(next);
    setActive(-1);
    setOpen(true);
  }

  function add(raw: string) {
    const trimmed = raw.trim();
    if (!trimmed) return;
    if ([...trimmed].length > 50) { onError(t('tagTooLong')); return; }
    if (value.length >= 30) { onError(t('tooManyTags')); return; }
    const tag = suggestions.find(existing => existing.toLocaleLowerCase() === trimmed.toLocaleLowerCase()) ?? trimmed;
    if (!value.some(existing => existing.toLocaleLowerCase() === tag.toLocaleLowerCase())) {
      onChange([...value, tag]);
    }
    changePending('');
  }

  function handleInput(next: string) {
    if (!next.includes(',')) { changePending(next); return; }
    const parts = next.split(',');
    const additions = parts.slice(0, -1).map(part => part.trim()).filter(Boolean);
    const combined = [...value];
    for (const addition of additions) {
      if ([...addition].length > 50) { onError(t('tagTooLong')); return; }
      if (combined.length >= 30) { onError(t('tooManyTags')); return; }
      const tag = suggestions.find(existing => existing.toLocaleLowerCase() === addition.toLocaleLowerCase()) ?? addition;
      if (!combined.some(existing => existing.toLocaleLowerCase() === tag.toLocaleLowerCase())) combined.push(tag);
    }
    onChange(combined);
    changePending(parts.at(-1) ?? '');
  }

  return <div className="tag-editor" onBlur={event => {
    if (!event.currentTarget.contains(event.relatedTarget)) setOpen(false);
  }}>
    {value.length > 0 && <div className="editable-tags">{value.map((tag, index) =>
      <span className="tag-chip editable-tag" key={`${tag}-${index}`}>{tag}
        <button type="button" aria-label={`${t('removeTag')} ${tag}`} onClick={() => onChange(value.filter((_, i) => i !== index))}>×</button>
      </span>
    )}</div>}
    <div className="tag-entry">
      <input id="tags" value={pending} placeholder={t('typeTag')} autoComplete="off"
        role="combobox" aria-autocomplete="list" aria-expanded={open && matches.length > 0}
        aria-controls="tag-suggestions" onFocus={() => setOpen(true)}
        onChange={event => handleInput(event.target.value)} onKeyDown={event => {
          if (event.key === 'Enter' || event.key === ',') {
            event.preventDefault();
            add(event.key === 'Enter' && active >= 0 ? matches[active] : pending);
          } else if (event.key === 'ArrowDown' && matches.length) {
            event.preventDefault(); setActive((active + 1) % matches.length); setOpen(true);
          } else if (event.key === 'ArrowUp' && matches.length) {
            event.preventDefault(); setActive((active - 1 + matches.length) % matches.length); setOpen(true);
          } else if (event.key === 'Backspace' && !pending && value.length) {
            onChange(value.slice(0, -1));
          } else if (event.key === 'Escape') setOpen(false);
        }} />
      <button className="tag-add" type="button" onClick={() => add(pending)}>{t('addTag')}</button>
    </div>
    <div className="tag-suggestions" id="tag-suggestions" role="listbox" hidden={!open || matches.length === 0}>
      {matches.map((tag, index) => <button type="button" className={`tag-suggestion ${active === index ? 'highlighted' : ''}`}
        role="option" aria-selected={active === index} key={tag} onMouseDown={event => event.preventDefault()}
        onClick={() => add(tag)}>{tag}</button>)}
    </div>
  </div>;
}
