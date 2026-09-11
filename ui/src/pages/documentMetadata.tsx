import { useCallback, useMemo, useRef, useState } from 'react';
import { useNavigate } from 'react-router-dom';

import { useDocument, useSaveDocumentMetadata } from '../queries/useDocuments';
import { Message } from 'primereact/message';
import { Card } from 'primereact/card';
import { DataTable } from 'primereact/datatable';
import { Column, type ColumnEditorOptions, type ColumnEvent } from 'primereact/column';
import { Button } from 'primereact/button';
import { Calendar } from 'primereact/calendar';
import { Dropdown } from 'primereact/dropdown';
import { AutoComplete, type AutoCompleteCompleteEvent } from 'primereact/autocomplete';

import { useId } from '../util';
import { fetchMetadataTypeValues, useDocumentTypeMetadataTypes, useMetadataTypesMap } from '../queries/useMetadataTypes';
import { DocumentViewLayout } from '../components/DocumentViewLayout';
import { buildMetadataRows, buildMetadataUpdates, getMissingRequiredRows, type MetadataRow } from '../util/documentMetadataRows';

type MetadataValueAutoCompleteProps = {
  metadataTypeId: number | undefined;
  value: string;
  onChange: (value: string) => void;
  onBlur: (event: React.FocusEvent<HTMLElement>, value: string) => void;
};

function MetadataValueAutoComplete({ metadataTypeId, value, onChange, onBlur }: MetadataValueAutoCompleteProps) {
  const [inputValue, setInputValue] = useState(value);
  const [suggestions, setSuggestions] = useState<string[]>([]);
  const requestRef = useRef(0);

  const completeMethod = useCallback((event: AutoCompleteCompleteEvent) => {
    const query = event.query ?? '';
    if (query.trim().length < 2) {
      setSuggestions([]);
      return;
    }

    if (!metadataTypeId) {
      setSuggestions([]);
      return;
    }

    const requestId = requestRef.current + 1;
    requestRef.current = requestId;

    fetchMetadataTypeValues(metadataTypeId, query)
      .then((values) => {
        if (requestRef.current === requestId) {
          setSuggestions(values);
        }
      })
      .catch(() => {
        if (requestRef.current === requestId) {
          setSuggestions([]);
        }
      });
  }, [metadataTypeId]);

  return (
    <AutoComplete
      value={inputValue}
      suggestions={suggestions}
      completeMethod={completeMethod}
      onChange={(event) => {
        const nextValue = String(event.value ?? '');
        setInputValue(nextValue);
        onChange(nextValue);
      }}
      onSelect={(event) => {
        const nextValue = String(event.value ?? '');
        setInputValue(nextValue);
        onChange(nextValue);
      }}
      onBlur={(event) => onBlur(event, inputValue)}
      inputClassName="w-full"
      className="w-full"
      dropdown={false}
    />
  );
}

export function EditDocumentMetadata() {
  const navigate = useNavigate();
  const id = useId('id');
  const saveDocumentMetadata = useSaveDocumentMetadata(id);
  const { isLoading, isError, data: doc, error } = useDocument(id);
  const { isLoading: isDocumentTypeMetadataLoading, data: dtmdts } = useDocumentTypeMetadataTypes(doc?.document_type_id);
  const { isLoading: isMetadataTypesLoading, data: mdt } = useMetadataTypesMap('id');

  const loadedRows = useMemo<MetadataRow[] | undefined>(() => {
    if (!doc || !mdt || !dtmdts) return undefined;
    return buildMetadataRows(doc, mdt, dtmdts);
  }, [doc, dtmdts, mdt]);
  const loadedRowsKey = useMemo(() => {
    if (!loadedRows) return '';
    return JSON.stringify(loadedRows.map((row) => [row.metadataTypeId, row.value, row.required]));
  }, [loadedRows]);
  const [editedRows, setEditedRows] = useState<{ sourceKey: string; rows: MetadataRow[] } | null>(null);
  const rows = useMemo(
    () => editedRows?.sourceKey === loadedRowsKey ? editedRows.rows : loadedRows ?? [],
    [editedRows, loadedRows, loadedRowsKey]
  );

  const missingRequiredRows = useMemo(
    () => getMissingRequiredRows(rows),
    [rows]
  );
  const hasMissingRequired = missingRequiredRows.length > 0;
  const missingRequiredMessage = useMemo(() => {
    if (!hasMissingRequired) {
      return '';
    }

    return `Required fields are missing: ${missingRequiredRows.map((row) => row.name).join(', ')}`;
  }, [hasMissingRequired, missingRequiredRows]);

  const requiredTemplate = useCallback((rowData: {required: boolean}) => {
    return rowData.required ?
        <i className="pi pi-check" style={{color: 'var(--green-600)'}} /> :
        <i className="pi pi-times" style={{color: 'var(--red-600)'}} />;
  }, []);

  const updateRowValue = useCallback((metadataTypeId: number | undefined, value: string) => {
    if (!metadataTypeId) return;

    setEditedRows({
      sourceKey: loadedRowsKey,
      rows: rows.map((row) => (
        row.metadataTypeId === metadataTypeId
          ? { ...row, value }
          : row
      )),
    });
  }, [loadedRowsKey, rows]);

  const closeCellEditor = useCallback((event: React.FocusEvent<HTMLElement>) => {
    const editor = event.currentTarget;
    const cell = event.currentTarget?.closest('td');
    if (!cell) return;
    const relatedTarget = event.relatedTarget as HTMLElement | null;
    if (relatedTarget) {
      if (cell.contains(relatedTarget)) return;
      if (relatedTarget.closest('.p-datepicker, .p-datepicker-panel, .p-datepicker-calendar, .p-datepicker-group')) return;
      if (relatedTarget.closest('.p-autocomplete-panel')) return;
    }

    setTimeout(() => {
      const activeElement = document.activeElement as HTMLElement | null;
      if (!activeElement) return;
      if (cell.contains(activeElement)) return;
      if (activeElement.closest('.p-datepicker, .p-datepicker-panel, .p-datepicker-calendar, .p-datepicker-group')) return;
      if (activeElement.closest('.p-autocomplete-panel')) return;

      const enterEvent = new KeyboardEvent('keydown', {
        bubbles: true,
        cancelable: true,
        key: 'Enter',
        code: 'Enter',
        keyCode: 13,
        which: 13,
      });
      editor.dispatchEvent(enterEvent);
      cell.dispatchEvent(new KeyboardEvent('keydown', {
        bubbles: true,
        cancelable: true,
        key: 'Enter',
        code: 'Enter',
        keyCode: 13,
        which: 13,
      }));
    }, 200);
  }, []);

  const textEditor = useCallback((options: ColumnEditorOptions) => {
    const rowMetadataTypeId = options.rowData?.metadataTypeId as number | undefined;

    return <MetadataValueAutoComplete
        metadataTypeId={rowMetadataTypeId}
        value={String(options.value ?? '')}
        onChange={(value) => options.editorCallback?.(value)}
        onBlur={(event, value) => {
          updateRowValue(rowMetadataTypeId, value);
          closeCellEditor(event);
        }}
    />;
  }, [closeCellEditor, updateRowValue]);

  const dateEditor = useCallback((options: ColumnEditorOptions) => {
    const dateValue = typeof options.value === 'string' && options.value
      ? (() => {
          const [year, month, day] = options.value.split('-').map(Number);
          if (!year || !month || !day) return null;
          return new Date(year, month - 1, day);
        })()
      : null;

    return (
      <Calendar
        value={dateValue}
        onChange={(event) => {
          const nextValue = event.value instanceof Date
            ? `${event.value.getFullYear()}-${String(event.value.getMonth() + 1).padStart(2, '0')}-${String(event.value.getDate()).padStart(2, '0')}`
            : (typeof event.value === 'string' ? event.value : '');
          options.editorCallback?.(nextValue);
          updateRowValue(options.rowData?.metadataTypeId as number | undefined, nextValue);
        }}
        onBlur={(event) => {
          closeCellEditor(event);
        }}
        dateFormat="yy-mm-dd"
        placeholder="yyyy-mm-dd"
        showIcon
        className="w-full"
      />
    );
  }, [closeCellEditor, updateRowValue]);

  const lookupEditor = useCallback((options: ColumnEditorOptions) => {
    const choices = options.rowData?.options?.choices ?? [];
    const lookupOptions = choices.map((choice: string) => ({ label: choice, value: choice }));

    return (
      <Dropdown
        value={options.value ?? ''}
        onChange={(event) => {
          const nextValue = event.value ?? '';
          options.editorCallback?.(nextValue);
          updateRowValue(options.rowData?.metadataTypeId as number | undefined, nextValue);
        }}
        onBlur={(event) => {
          closeCellEditor(event);
        }}
        options={lookupOptions}
        placeholder={lookupOptions.length > 0 ? 'Select a choice' : 'No choices'}
        className="w-full"
      />
    );
  }, [closeCellEditor, updateRowValue]);

  const onCellEditComplete = useCallback((e: ColumnEvent) => {
    const { rowData, newValue, field  } = e;
    rowData[field] = newValue;
    setEditedRows({
      sourceKey: loadedRowsKey,
      rows: rows.map((row) => (
        row.metadataTypeId === rowData.metadataTypeId
          ? { ...row, [field]: newValue }
          : row
      )),
    });
  }, [loadedRowsKey, rows]);

  const cellEditor = useCallback((options: ColumnEditorOptions) => {
    if (options.field !== 'value')
      return null;
    if (options.rowData?.dataType === 'date')
      return dateEditor(options);
    if (options.rowData?.dataType === 'lookup')
      return lookupEditor(options);
    return textEditor(options);
  }, [dateEditor, lookupEditor, textEditor]);

  const columns = useMemo(() => ([
    <Column key="name" field="name" header="Field" style={{ width: '25%' }} />,
    <Column key="value" field="value" header="Value" style={{ width: '60%' }}
      editor={cellEditor} onCellEditComplete={onCellEditComplete}
    />,
    <Column key="required" field="required" header="Required" style={{ width: '15%' }}
      body={requiredTemplate}
    />
  ]), [cellEditor, onCellEditComplete, requiredTemplate]);

  const onSave = async () => {
    if (hasMissingRequired) {
      return;
    }

    const original = doc?.metadata ?? {};
    const updates = buildMetadataUpdates(original, rows);

    try {
      await saveDocumentMetadata.mutateAsync(updates);
      navigate(`/documents/${id}/preview`);
    } catch (err) {
      console.error(err);
    }
  };

  if (!id)
    return <Message severity="error" text="Missing or invalid ID" />;
  if (isError)
    return <Message severity="error" text={error.message} />
  if (isLoading || isDocumentTypeMetadataLoading || isMetadataTypesLoading || !loadedRows)
    return <div>Loading</div>;

  return (
    <DocumentViewLayout documentId={id}>
      <Card title={`Document Metadata: ${doc?.title}`}>
        <DataTable value={rows} editMode="cell" tableStyle={{ minWidth: '50rem' }}>
          {columns}
        </DataTable>

        <div className="mb-3">
          {saveDocumentMetadata.isError && (
            <Message severity="error" text={saveDocumentMetadata.error.message} />
          )}
          {hasMissingRequired && (
            <Message severity="warn" text={missingRequiredMessage} />
          )}
        </div>

        <div className="text-end">
          <Button label="Save" type="submit" icon="pi pi-check" onClick={onSave} raised disabled={saveDocumentMetadata.isPending || hasMissingRequired} />
          <Button label="Cancel" type="button" severity="secondary" icon="pi pi-times" raised onClick={() => navigate('/documents')} />
        </div>
      </Card>
    </DocumentViewLayout>
  );
}
