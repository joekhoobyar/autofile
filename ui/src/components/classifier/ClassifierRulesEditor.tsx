import { useEffect, useRef, useState, type MouseEvent, type ReactNode } from 'react';

import CodeMirror from '@uiw/react-codemirror';
import { EditorState } from '@codemirror/state';
import { EditorView } from '@codemirror/view';
import { yaml as yamlLanguage } from '@codemirror/lang-yaml';
import { vscodeDark } from '@uiw/codemirror-theme-vscode';
import { Button } from 'primereact/button';
import { Checkbox } from 'primereact/checkbox';
import { Dialog } from 'primereact/dialog';
import { Dropdown } from 'primereact/dropdown';
import { InputNumber } from 'primereact/inputnumber';
import { InputText } from 'primereact/inputtext';
import { Message } from 'primereact/message';
import { MultiSelect } from 'primereact/multiselect';
import { Panel } from 'primereact/panel';
import { Tooltip } from 'primereact/tooltip';

import type {
  ClassifierModifier,
  ClassifierPattern,
  ClassifierRules,
} from '../../models/classifierBlock';
import type { MetadataType } from '../../models/metadataType';
import { useClassifierRuleOptions, type ClassifierRuleOptions } from '../../queries/useClassifierRuleOptions';
import {
  validateClassifierRules,
  type ClassifierRulesValidation,
} from '../../queries/useClassifierBlocks';
import { rulesToYaml, yamlToRules } from '../../util/classifierRulesYaml';

type Option = { label: string; value: string; disabled?: boolean };

type ClassifierRulesEditorProps = {
  value: ClassifierRules;
  onChange: (rules: ClassifierRules) => void;
  onValidationChange: (valid: boolean) => void;
};

const regexExtensions = [
  EditorState.changeFilter.of((transaction) => transaction.newDoc.lines === 1),
];
const yamlExtensions = [yamlLanguage(), EditorView.lineWrapping];

const modifierOptions: Option[] = [
  { label: 'Metadata', value: 'metadata' },
  { label: 'Month Number', value: 'month_number' },
  { label: 'Month End', value: 'month_end' },
  { label: 'Month Start', value: 'month_start' },
  { label: 'Next Day', value: 'next_day' },
  { label: 'Previous Day', value: 'prev_day' },
  { label: 'Next Month', value: 'next_month' },
  { label: 'Previous Month', value: 'prev_month' },
  { label: 'Tax Year', value: 'tax_year' },
  { label: 'Currency', value: 'currency' },
  { label: 'Zero Pad', value: 'zero_pad' },
  { label: 'Replace / Compose', value: 'replace' },
  { label: 'Alphanumeric Sanitize', value: 'alnum_sanitize' },
  { label: 'Date Format', value: 'date_format' },
  { label: 'Add', value: 'add' },
  { label: 'Subtract', value: 'sub' },
  { label: 'Multiply', value: 'mul' },
  { label: 'Divide', value: 'div' },
];

const specialActionOptions: Option[] = [
  { label: 'Document Type', value: '_suggested_doctype' },
  { label: 'Document Title', value: '_suggested_filename' },
  { label: 'Tags', value: '_suggested_tags' },
  { label: 'Cabinets', value: '_suggested_cabinets' },
];

function actionOptions(options?: ClassifierRuleOptions): Option[] {
  return [
    ...specialActionOptions,
    ...(options?.metadataTypes.map((metadataType) => ({
      label: `Metadata: ${metadataType.name}`,
      value: metadataType.slug,
    })) ?? []),
  ];
}

function metadataOptions(options?: ClassifierRuleOptions): Option[] {
  return options?.metadataTypes.map((metadataType) => ({
    label: `Metadata: ${metadataType.name}`,
    value: metadataType.slug,
  })) ?? [];
}

function withCurrentOption(options: Option[], value: string, prefix: string): Option[] {
  if (!value || options.some((option) => option.value === value)) return options;
  return [{ label: `${prefix}: ${value} (not found)`, value }, ...options];
}

function replaceRecordKey(
  record: Record<string, string>,
  oldKey: string,
  newKey: string,
): Record<string, string> {
  if (!newKey || oldKey === newKey || Object.hasOwn(record, newKey)) return record;
  return Object.fromEntries(Object.entries(record).map(([key, value]) => [key === oldKey ? newKey : key, value]));
}

function addRecordEntry(record: Record<string, string>, preferredKeys: string[]): Record<string, string> {
  const key = preferredKeys.find((candidate) => !Object.hasOwn(record, candidate));
  if (!key) return record;
  return { ...record, [key]: '' };
}

function removeRecordEntry(record: Record<string, string>, keyToRemove: string): Record<string, string> {
  return Object.fromEntries(Object.entries(record).filter(([key]) => key !== keyToRemove));
}

function metadataTypeFor(slug: string, options?: ClassifierRuleOptions): MetadataType | undefined {
  return options?.metadataTypes.find((metadataType) => metadataType.slug === slug);
}

function listValue(value: string): string[] {
  return value.split(',').map((entry) => entry.trim()).filter(Boolean);
}

function defaultModifier(type: ClassifierModifier['type']): ClassifierModifier {
  switch (type) {
    case 'metadata': return { type, slug: '', to: 1 };
    case 'zero_pad': return { type, from: '\\1', to: 1, length: 2 };
    case 'date_format': return { type, from: '\\1', to: 1, format: '%Y-%m-%d' };
    case 'add':
    case 'sub':
    case 'mul':
    case 'div': return { type, from: 1, to: 2 };
    default: return { type, from: '\\1', to: 1 };
  }
}

function tokenNumbers(captureCount: number, modifiers: ClassifierModifier[], beforeIndex?: number): number[] {
  const tokens = new Set<number>();
  for (let index = 1; index <= captureCount; index += 1) tokens.add(index);
  const availableModifiers = beforeIndex === undefined ? modifiers : modifiers.slice(0, beforeIndex);
  availableModifiers.forEach((modifier) => tokens.add(modifier.to));
  return [...tokens].sort((left, right) => left - right);
}

function moveItem<T>(items: T[], from: number, to: number): T[] {
  if (to < 0 || to >= items.length) return items;
  const next = [...items];
  const [item] = next.splice(from, 1);
  next.splice(to, 0, item);
  return next;
}

function RulePanel({
  heading,
  actions,
  children,
}: Readonly<{
  heading: string;
  actions: ReactNode;
  children: ReactNode;
}>) {
  const [collapsed, setCollapsed] = useState(false);
  const toggle = () => setCollapsed((value) => !value);

  return (
    <Panel
      header={(
        <button
          type="button"
          className="aut-panel-title-button"
          aria-expanded={!collapsed}
        >
          {heading}
        </button>
      )}
      toggleable
      collapsed={collapsed}
      onToggle={(event) => setCollapsed(event.value)}
      className="aut-rule-panel"
      icons={actions}
      pt={{
        header: {
          onClick: (event: MouseEvent<HTMLDivElement>) => {
            if ((event.target as HTMLElement).closest('.p-panel-icons')) return;
            toggle();
          },
        },
        toggler: {
          className: 'aut-rule-panel-toggler',
          'aria-label': collapsed ? `Expand ${heading}` : `Collapse ${heading}`,
          title: collapsed ? `Expand ${heading}` : `Collapse ${heading}`,
          'data-pr-tooltip': collapsed ? `Expand ${heading}` : `Collapse ${heading}`,
          'data-pr-position': 'top',
        },
      }}
    >
      {children}
    </Panel>
  );
}

function TemplateInput({
  value,
  onChange,
  tokens,
  placeholder,
}: Readonly<{
  value: string;
  onChange: (value: string) => void;
  tokens: number[];
  placeholder?: string;
}>) {
  const inputRef = useRef<HTMLInputElement>(null);

  const insertToken = (token: number) => {
    const input = inputRef.current;
    const start = input?.selectionStart ?? value.length;
    const end = input?.selectionEnd ?? start;
    const replacement = `\\${token}`;
    onChange(`${value.slice(0, start)}${replacement}${value.slice(end)}`);
    requestAnimationFrame(() => {
      input?.focus();
      input?.setSelectionRange(start + replacement.length, start + replacement.length);
    });
  };

  return (
    <div className="aut-template-input">
      <InputText
        ref={inputRef}
        value={value}
        onChange={(event) => onChange(event.target.value)}
        placeholder={placeholder}
      />
      {tokens.length > 0 && (
        <div className="aut-capture-chips" aria-label="Available replacement snippets">
          {tokens.map((token) => (
            <Button
              key={token}
              type="button"
              label={`\\${token}`}
              severity="secondary"
              text
              size="small"
              title={`Insert snippet \\${token}`}
              onMouseDown={(event) => event.preventDefault()}
              onClick={() => insertToken(token)}
            />
          ))}
        </div>
      )}
    </div>
  );
}

function PatternEditor({
  pattern,
  onChange,
  options,
  issues,
  captureReplacements,
}: Readonly<{
  pattern: ClassifierPattern;
  onChange: (pattern: ClassifierPattern) => void;
  options?: ClassifierRuleOptions;
  issues: string[];
  captureReplacements: boolean;
}>) {
  const metadata = pattern.metadata ?? {};
  const metadataEntries = Object.entries(metadata);
  const availableMetadataOptions = metadataOptions(options);
  const singleLineText = (pattern.text ?? '').replace(/[\r\n]+/g, '');

  return (
    <div className="aut-pattern-editor">
      <label className="font-medium block mb-2">Text pattern</label>
      <CodeMirror
        value={singleLineText}
        height="2.5rem"
        theme={vscodeDark}
        extensions={regexExtensions}
        basicSetup={{ lineNumbers: false, foldGutter: false, highlightActiveLineGutter: false }}
        onChange={(text) => onChange({ ...pattern, text })}
        className="aut-regex-editor"
        placeholder="Rust regular expression"
      />
      <small className="text-600 block mt-1">
        {captureReplacements
          ? 'Matching is case-insensitive and multiline. Capturing parentheses produce \\1, \\2, and later snippets.'
          : 'Matching is case-insensitive and multiline. Top-level captures are not reused; use child rules to extract values.'}
      </small>

      <div className="flex align-items-center justify-content-between gap-2 mt-3 mb-2">
        <span className="font-medium">Metadata conditions</span>
        <Button
          type="button"
          label="Add metadata condition"
          icon="pi pi-plus"
          severity="secondary"
          text
          size="small"
          onClick={() => onChange({
            ...pattern,
            metadata: addRecordEntry(metadata, availableMetadataOptions.map((option) => option.value)),
          })}
          disabled={metadataEntries.length >= availableMetadataOptions.length}
        />
      </div>
      <small className="text-600 block mb-2">All metadata conditions in this pattern must match exactly.</small>

      {metadataEntries.map(([slug, expectedValue]) => {
        const metadataType = metadataTypeFor(slug, options);
        const choices = metadataType?.data_type === 'lookup' ? metadataType.options?.choices ?? [] : [];
        return (
          <div className="aut-editor-row" key={slug}>
            <Dropdown
              value={slug}
              options={withCurrentOption(availableMetadataOptions, slug, 'Metadata')}
              optionDisabled={(option: Option) => option.value !== slug && Object.hasOwn(metadata, option.value)}
              onChange={(event) => onChange({ ...pattern, metadata: replaceRecordKey(metadata, slug, String(event.value ?? '')) })}
              filter
              placeholder="Metadata Type"
            />
            {choices.length > 0 ? (
              <Dropdown
                value={expectedValue}
                options={choices.map((choice) => ({ label: choice, value: choice }))}
                onChange={(event) => onChange({ ...pattern, metadata: { ...metadata, [slug]: String(event.value ?? '') } })}
                editable
                placeholder="Exact value"
              />
            ) : (
              <InputText
                value={expectedValue}
                onChange={(event) => onChange({ ...pattern, metadata: { ...metadata, [slug]: event.target.value } })}
                placeholder={metadataType?.data_type === 'date' ? 'YYYY-MM-DD' : 'Exact value'}
              />
            )}
            <Button
              type="button"
              icon="pi pi-trash"
              severity="danger"
              text
              aria-label={`Remove Metadata: ${metadataType?.name ?? slug}`}
              onClick={() => onChange({ ...pattern, metadata: removeRecordEntry(metadata, slug) })}
            />
          </div>
        );
      })}

      {issues.map((issue) => <small className="p-error block mt-2" key={issue}>{issue}</small>)}
    </div>
  );
}

function ActionValueEditor({
  actionKey,
  value,
  onChange,
  child,
  tokens,
  options,
}: Readonly<{
  actionKey: string;
  value: string;
  onChange: (value: string) => void;
  child: boolean;
  tokens: number[];
  options?: ClassifierRuleOptions;
}>) {
  const supportsPicker = ['_suggested_doctype', '_suggested_tags', '_suggested_cabinets'].includes(actionKey);
  const [expressionMode, setExpressionMode] = useState(child && supportsPicker && /\\\d+/.test(value));

  if (child && supportsPicker && expressionMode) {
    return (
      <div className="aut-action-value-with-mode">
        <TemplateInput value={value} onChange={onChange} tokens={tokens} placeholder="Expression such as \\1" />
        <Button type="button" label="Pick values" severity="secondary" text size="small" onClick={() => setExpressionMode(false)} />
      </div>
    );
  }

  let editor;
  if (actionKey === '_suggested_doctype') {
    editor = (
      <Dropdown
        value={value}
        options={options?.documentTypes.map((item) => ({ label: item.name, value: item.slug })) ?? []}
        onChange={(event) => onChange(String(event.value ?? ''))}
        editable
        filter
        placeholder="Document Type"
      />
    );
  } else if (actionKey === '_suggested_tags') {
    editor = (
      <MultiSelect
        value={listValue(value)}
        options={options?.tags.map((item) => ({ label: item.name, value: item.slug })) ?? []}
        onChange={(event) => onChange((event.value as string[]).join(','))}
        filter
        display="chip"
        placeholder="Tags"
      />
    );
  } else if (actionKey === '_suggested_cabinets') {
    editor = (
      <MultiSelect
        value={listValue(value)}
        options={options?.cabinets.map((item) => ({ label: item.displayName ?? item.name, value: item.slug })) ?? []}
        onChange={(event) => onChange((event.value as string[]).join(','))}
        filter
        display="chip"
        placeholder="Cabinets"
      />
    );
  } else {
    const metadataType = metadataTypeFor(actionKey, options);
    const choices = metadataType?.data_type === 'lookup' ? metadataType.options?.choices ?? [] : [];
    editor = choices.length > 0 && !child ? (
      <Dropdown
        value={value}
        options={choices.map((choice) => ({ label: choice, value: choice }))}
        onChange={(event) => onChange(String(event.value ?? ''))}
        editable
        placeholder="Value"
      />
    ) : (
      <TemplateInput
        value={value}
        onChange={onChange}
        tokens={child ? tokens : []}
        placeholder={metadataType?.data_type === 'date' && !child ? 'YYYY-MM-DD' : 'Value'}
      />
    );
  }

  return (
    <div className="aut-action-value-with-mode">
      {editor}
      {child && supportsPicker && (
        <Button type="button" label="Use expression" severity="secondary" text size="small" onClick={() => setExpressionMode(true)} />
      )}
    </div>
  );
}

function ActionsEditor({
  actions,
  onChange,
  child,
  tokens,
  options,
}: Readonly<{
  actions: Record<string, string>;
  onChange: (actions: Record<string, string>) => void;
  child: boolean;
  tokens: number[];
  options?: ClassifierRuleOptions;
}>) {
  const optionsForActions = actionOptions(options);
  const entries = Object.entries(actions);

  return (
    <div>
      <div className="flex align-items-center justify-content-between gap-2 mb-2">
        <span className="font-medium">Actions</span>
        <Button
          type="button"
          label="Add action"
          icon="pi pi-plus"
          severity="secondary"
          text
          size="small"
          onClick={() => onChange(addRecordEntry(actions, optionsForActions.map((option) => option.value)))}
          disabled={entries.length >= optionsForActions.length}
        />
      </div>
      {entries.length === 0 && <small className="text-600">No actions configured.</small>}
      {entries.map(([key, actionValue]) => (
        <div className="aut-editor-row aut-action-row" key={key}>
          <Dropdown
            value={key}
            options={withCurrentOption(optionsForActions, key, 'Action')}
            optionDisabled={(option: Option) => option.value !== key && Object.hasOwn(actions, option.value)}
            onChange={(event) => onChange(replaceRecordKey(actions, key, String(event.value ?? '')))}
            filter
            editable
            placeholder="Action"
          />
          <ActionValueEditor
            key={key}
            actionKey={key}
            value={actionValue}
            onChange={(nextValue) => onChange({ ...actions, [key]: nextValue })}
            child={child}
            tokens={tokens}
            options={options}
          />
          <Button
            type="button"
            icon="pi pi-trash"
            severity="danger"
            text
            aria-label={`Remove action ${key}`}
            onClick={() => onChange(removeRecordEntry(actions, key))}
          />
        </div>
      ))}
    </div>
  );
}

function ModifierEditor({
  modifier,
  onChange,
  tokens,
  options,
}: Readonly<{
  modifier: ClassifierModifier;
  onChange: (modifier: ClassifierModifier) => void;
  tokens: number[];
  options?: ClassifierRuleOptions;
}>) {
  const update = (patch: Record<string, unknown>) => onChange({ ...modifier, ...patch } as ClassifierModifier);
  const actionSourceOptions = actionOptions(options);
  const arithmetic = ['add', 'sub', 'mul', 'div'].includes(modifier.type);

  return (
    <div className="aut-modifier-fields">
      <div className="aut-modifier-field">
        <label>Modifier</label>
        <Dropdown
          value={modifier.type}
          options={modifierOptions}
          onChange={(event) => onChange(defaultModifier(event.value as ClassifierModifier['type']))}
          placeholder="Modifier"
        />
      </div>
      <div className="aut-modifier-field">
        <label>{modifier.type === 'metadata' ? 'From action' : arithmetic ? 'From snippet' : 'From'}</label>
        {modifier.type === 'metadata' ? (
          <Dropdown
            value={modifier.slug}
            options={withCurrentOption(actionSourceOptions, modifier.slug, 'Action')}
            onChange={(event) => update({ slug: String(event.value ?? '') })}
            filter
            editable
            placeholder="Computed metadata or action"
          />
        ) : arithmetic ? (
          <InputNumber
            value={'from' in modifier && typeof modifier.from === 'number' ? modifier.from : 0}
            onValueChange={(event) => update({ from: event.value ?? 0 })}
            min={0}
            useGrouping={false}
            placeholder="From snippet"
          />
        ) : (
          <TemplateInput
            value={'from' in modifier && typeof modifier.from === 'string' ? modifier.from : ''}
            onChange={(from) => update({ from })}
            tokens={tokens}
            placeholder="From template"
          />
        )}
      </div>
      <div className="aut-modifier-field">
        <label>To snippet</label>
        <InputNumber
          value={modifier.to}
          onValueChange={(event) => update({ to: event.value ?? 0 })}
          min={0}
          useGrouping={false}
        />
      </div>
      {modifier.type === 'zero_pad' && (
        <div className="aut-modifier-field">
          <label>Length</label>
          <InputNumber
            value={modifier.length}
            onValueChange={(event) => update({ length: event.value ?? 0 })}
            min={0}
            useGrouping={false}
          />
        </div>
      )}
      {modifier.type === 'date_format' && (
        <div className="aut-modifier-field">
          <label>Format</label>
          <InputText value={modifier.format} onChange={(event) => update({ format: event.target.value })} placeholder="Chrono format" />
        </div>
      )}
    </div>
  );
}

function ModifiersEditor({
  modifiers,
  onChange,
  captureCount,
  options,
}: Readonly<{
  modifiers: ClassifierModifier[];
  onChange: (modifiers: ClassifierModifier[]) => void;
  captureCount: number;
  options?: ClassifierRuleOptions;
}>) {
  return (
    <div className="mt-3">
      <div className="flex align-items-center justify-content-between gap-2 mb-2">
        <span className="font-medium">Modifier pipeline</span>
        <Button
          type="button"
          label="Add modifier"
          icon="pi pi-plus"
          severity="secondary"
          text
          size="small"
          onClick={() => onChange([...modifiers, defaultModifier('replace')])}
        />
      </div>
      {modifiers.length === 0 && <small className="text-600">No transformations configured.</small>}
      {modifiers.map((modifier, index) => (
        <div className="aut-modifier-row" key={`${index}-${modifier.type}`}>
          <span className="aut-row-number">{index + 1}</span>
          <ModifierEditor
            modifier={modifier}
            onChange={(nextModifier) => onChange(modifiers.map((item, itemIndex) => itemIndex === index ? nextModifier : item))}
            tokens={tokenNumbers(captureCount, modifiers, index)}
            options={options}
          />
          <div className="aut-row-actions">
            <Button type="button" icon="pi pi-arrow-up" text aria-label="Move modifier up" disabled={index === 0} onClick={() => onChange(moveItem(modifiers, index, index - 1))} />
            <Button type="button" icon="pi pi-arrow-down" text aria-label="Move modifier down" disabled={index === modifiers.length - 1} onClick={() => onChange(moveItem(modifiers, index, index + 1))} />
            <Button type="button" icon="pi pi-trash" severity="danger" text aria-label="Remove modifier" onClick={() => onChange(modifiers.filter((_, itemIndex) => itemIndex !== index))} />
          </div>
        </div>
      ))}
    </div>
  );
}

function AdvancedYamlDialog({
  visible,
  rules,
  onApply,
  onHide,
}: Readonly<{
  visible: boolean;
  rules: ClassifierRules;
  onApply: (rules: ClassifierRules) => void;
  onHide: () => void;
}>) {
  const [draft, setDraft] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [applying, setApplying] = useState(false);

  useEffect(() => {
    if (visible) {
      setDraft(rulesToYaml(rules));
      setError(null);
    }
  }, [rules, visible]);

  const apply = async () => {
    const parsed = yamlToRules(draft);
    if (!parsed.value) {
      setError(parsed.error ?? 'Invalid YAML');
      return;
    }
    setApplying(true);
    try {
      const validation = await validateClassifierRules(parsed.value);
      if (!validation.valid) {
        setError(validation.issues[0]?.message ?? 'Invalid classifier rules');
        return;
      }
      onApply(parsed.value);
      onHide();
    } catch (validationError) {
      setError(validationError instanceof Error ? validationError.message : 'Validation failed');
    } finally {
      setApplying(false);
    }
  };

  return (
    <Dialog
      header="Advanced YAML"
      visible={visible}
      onHide={onHide}
      className="aut-classifier-yaml-dialog"
      footer={(
        <>
          <Button type="button" label="Cancel" severity="secondary" onClick={onHide} />
          <Button type="button" label="Apply YAML" icon="pi pi-check" loading={applying} onClick={() => void apply()} />
        </>
      )}
    >
      <Message severity="warn" text="Applying YAML replaces the structured rules after validation." className="mb-3 w-full" />
      <CodeMirror
        value={draft}
        height="60vh"
        theme={vscodeDark}
        extensions={yamlExtensions}
        onChange={setDraft}
        className="aut-yaml-editor"
      />
      {error && <small className="p-error block mt-2">{error}</small>}
    </Dialog>
  );
}

export function ClassifierRulesEditor({ value, onChange, onValidationChange }: Readonly<ClassifierRulesEditorProps>) {
  const optionsQuery = useClassifierRuleOptions();
  const [validation, setValidation] = useState<ClassifierRulesValidation>();
  const [validationError, setValidationError] = useState<string | null>(null);
  const [yamlVisible, setYamlVisible] = useState(false);

  useEffect(() => {
    let active = true;
    onValidationChange(false);
    const timeout = window.setTimeout(() => {
      void validateClassifierRules(value)
        .then((result) => {
          if (!active) return;
          setValidation(result);
          setValidationError(null);
          onValidationChange(result.valid);
        })
        .catch((error) => {
          if (!active) return;
          setValidation(undefined);
          setValidationError(error instanceof Error ? error.message : 'Could not validate rules');
          onValidationChange(false);
        });
    }, 400);

    return () => {
      active = false;
      window.clearTimeout(timeout);
    };
  }, [onValidationChange, value]);

  const issuesFor = (path: string) => validation?.issues
    .filter((issue) => issue.path === path || issue.path.startsWith(`${path}.`))
    .map((issue) => issue.message) ?? [];
  const captureCount = (childIndex: number) => validation?.patterns
    .find((pattern) => pattern.path === `child_rules[${childIndex}].pattern.text`)?.capture_count ?? 0;

  const options = optionsQuery.data;
  const parentPatterns = value.match_patterns;
  const childRules = value.child_rules;

  return (
    <div className="aut-classifier-rules-editor">
      <Tooltip target=".aut-classifier-rules-editor .aut-rule-panel-toggler" />
      <div className="flex flex-wrap align-items-start justify-content-between gap-3 mb-3">
        <div>
          <h3 className="mt-0 mb-1">Rules</h3>
          <p className="text-600 mt-0 mb-0">Build matching conditions, transformations, and document actions without editing YAML.</p>
        </div>
        <Button type="button" label="Advanced YAML" icon="pi pi-code" severity="secondary" outlined onClick={() => setYamlVisible(true)} />
      </div>

      {optionsQuery.isError && <Message severity="warn" text={`Lookup options could not be loaded: ${optionsQuery.error.message}`} className="mb-3 w-full" />}
      {validationError && <Message severity="error" text={validationError} className="mb-3 w-full" />}

      <section className="aut-rule-section">
        <div className="flex flex-wrap align-items-center justify-content-between gap-2 mb-2">
          <div>
            <h4 className="m-0">Match if any pattern matches</h4>
            <small className="text-600">At least one pattern is required. Text and metadata inside a pattern must all match.</small>
          </div>
          <Button
            type="button"
            label="Add pattern"
            icon="pi pi-plus"
            onClick={() => onChange({ ...value, match_patterns: [...parentPatterns, { text: '' }] })}
          />
        </div>

        {parentPatterns.map((pattern, index) => (
          <RulePanel
            key={index}
            heading={`Pattern ${index + 1}`}
            actions={(
              <div className="aut-panel-actions">
                <Button type="button" icon="pi pi-arrow-up" text aria-label="Move pattern up" tooltip="Move pattern up" tooltipOptions={{ position: 'top' }} disabled={index === 0} onClick={() => onChange({ ...value, match_patterns: moveItem(parentPatterns, index, index - 1) })} />
                <Button type="button" icon="pi pi-arrow-down" text aria-label="Move pattern down" tooltip="Move pattern down" tooltipOptions={{ position: 'top' }} disabled={index === parentPatterns.length - 1} onClick={() => onChange({ ...value, match_patterns: moveItem(parentPatterns, index, index + 1) })} />
                <Button type="button" icon="pi pi-trash" severity="danger" text aria-label="Remove pattern" tooltip="Remove pattern" tooltipOptions={{ position: 'top' }} disabled={parentPatterns.length === 1} onClick={() => onChange({ ...value, match_patterns: parentPatterns.filter((_, itemIndex) => itemIndex !== index) })} />
              </div>
            )}
          >
            <PatternEditor
              pattern={pattern}
              onChange={(nextPattern) => onChange({ ...value, match_patterns: parentPatterns.map((item, itemIndex) => itemIndex === index ? nextPattern : item) })}
              options={options}
              issues={issuesFor(`match_patterns[${index}]`)}
              captureReplacements={false}
            />
          </RulePanel>
        ))}
        {validation?.issues
          .filter((issue) => issue.path === 'match_patterns')
          .map((issue) => <small className="p-error block mt-2" key={issue.code}>{issue.message}</small>)}
      </section>

      <section className="aut-rule-section">
        <h4 className="mt-0 mb-1">Actions when matched</h4>
        <small className="text-600 block mb-3">These values are literal. Capture replacements are available only in child-rule actions.</small>
        <ActionsEditor
          actions={value.match_actions}
          onChange={(match_actions) => onChange({ ...value, match_actions })}
          child={false}
          tokens={[]}
          options={options}
        />
      </section>

      <section className="aut-rule-section">
        <div className="flex flex-wrap align-items-center justify-content-between gap-2 mb-2">
          <div>
            <h4 className="m-0">Child rules</h4>
            <small className="text-600">Every matching child rule runs in order.</small>
          </div>
          <Button
            type="button"
            label="Add child rule"
            icon="pi pi-plus"
            onClick={() => onChange({
              ...value,
              child_rules: [...childRules, { pattern: { text: '' }, modifiers: [], actions: {} }],
            })}
          />
        </div>

        {childRules.length === 0 && <Message severity="info" text="No child rules. Add one to capture text and compute dynamic action values." className="w-full" />}
        {childRules.map((rule, index) => {
          const modifiers = rule.modifiers ?? [];
          const captures = captureCount(index);
          return (
            <RulePanel
              key={index}
              heading={`Child Rule ${index + 1}`}
              actions={(
                <div className="aut-panel-actions">
                  <Button type="button" icon="pi pi-copy" text aria-label="Duplicate child rule" tooltip="Duplicate child rule" tooltipOptions={{ position: 'top' }} onClick={() => onChange({ ...value, child_rules: [...childRules.slice(0, index + 1), structuredClone(rule), ...childRules.slice(index + 1)] })} />
                  <Button type="button" icon="pi pi-arrow-up" text aria-label="Move child rule up" tooltip="Move child rule up" tooltipOptions={{ position: 'top' }} disabled={index === 0} onClick={() => onChange({ ...value, child_rules: moveItem(childRules, index, index - 1) })} />
                  <Button type="button" icon="pi pi-arrow-down" text aria-label="Move child rule down" tooltip="Move child rule down" tooltipOptions={{ position: 'top' }} disabled={index === childRules.length - 1} onClick={() => onChange({ ...value, child_rules: moveItem(childRules, index, index + 1) })} />
                  <Button type="button" icon="pi pi-trash" severity="danger" text aria-label="Remove child rule" tooltip="Remove child rule" tooltipOptions={{ position: 'top' }} onClick={() => onChange({ ...value, child_rules: childRules.filter((_, itemIndex) => itemIndex !== index) })} />
                </div>
              )}
            >
              <PatternEditor
                pattern={rule.pattern}
                onChange={(pattern) => onChange({ ...value, child_rules: childRules.map((item, itemIndex) => itemIndex === index ? { ...item, pattern } : item) })}
                options={options}
                issues={issuesFor(`child_rules[${index}].pattern`)}
                captureReplacements
              />
              {captures > 0 && (
                <Message
                  severity="success"
                  text={`${captures} capture ${captures === 1 ? 'group' : 'groups'} available as ${Array.from({ length: captures }, (_, captureIndex) => `\\${captureIndex + 1}`).join(', ')}`}
                  className="mt-3 w-full"
                />
              )}
              <ModifiersEditor
                modifiers={modifiers}
                onChange={(nextModifiers) => onChange({ ...value, child_rules: childRules.map((item, itemIndex) => itemIndex === index ? { ...item, modifiers: nextModifiers } : item) })}
                captureCount={captures}
                options={options}
              />
              <div className="mt-4">
                <ActionsEditor
                  actions={rule.actions}
                  onChange={(actions) => onChange({ ...value, child_rules: childRules.map((item, itemIndex) => itemIndex === index ? { ...item, actions } : item) })}
                  child
                  tokens={tokenNumbers(captures, modifiers)}
                  options={options}
                />
              </div>
            </RulePanel>
          );
        })}
      </section>

      <section className="aut-rule-section aut-continue-section">
        <Checkbox
          inputId="continueAfterMatch"
          checked={value.continue_after_match === true}
          onChange={(event) => onChange({ ...value, continue_after_match: event.checked === true })}
        />
        <label htmlFor="continueAfterMatch">
          <span className="font-medium block">Continue processing later classifier blocks after this block matches</span>
          <small className="text-600">When disabled, classification stops after this block matches.</small>
        </label>
      </section>

      <AdvancedYamlDialog
        visible={yamlVisible}
        rules={value}
        onApply={onChange}
        onHide={() => setYamlVisible(false)}
      />
    </div>
  );
}
