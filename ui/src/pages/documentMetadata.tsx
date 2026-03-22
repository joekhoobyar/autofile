import { useCallback, useMemo } from 'react';
import { useNavigate } from 'react-router-dom';

import { useDocument, useSaveDocumentMetadata } from '../queries/useDocuments';
import { Message } from 'primereact/message';
import { Card } from 'primereact/card';
import { DataTable } from 'primereact/datatable';
import { Column, type ColumnEditorOptions, type ColumnEvent } from 'primereact/column';
import { InputText } from 'primereact/inputtext';
import { Button } from 'primereact/button';
import { Calendar } from 'primereact/calendar';

import { useId } from '../util';
import { useDocumentTypeMetadataTypes, useMetadataTypesMap } from '../queries/useMetadataTypes';

export function EditDocumentMetadata() {
  const navigate = useNavigate();
  const id = useId('id');
  const saveDocumentMetadata = useSaveDocumentMetadata(id);
  const { isLoading, isError, data: doc, error } = useDocument(id);
  const { data: dtmdts } = useDocumentTypeMetadataTypes(doc?.document_type_id);
  const { data: mdt } = useMetadataTypesMap('id');

  const rows = useMemo(() => {
    if (!mdt || !dtmdts) return [];
    return dtmdts.map(dtmdt => {
      const mdType = mdt?.[dtmdt.metadata_type_id];
        return {
          metadataTypeId: dtmdt.metadata_type_id,
          slug: mdType.slug,
          name: mdType?.name ?? mdType.slug,
          value: doc?.metadata?.[mdType.slug] ?? '',
          dataType: mdType?.data_type ?? 'string',
          required: dtmdt.required,
        };
    });
  }, [doc?.metadata, dtmdts, mdt]);

  const requiredTemplate = useCallback((rowData: {required: boolean}) => {
    return rowData.required ?
        <i className="pi pi-check" style={{color: 'var(--green-600)'}} /> :
        <i className="pi pi-times" style={{color: 'var(--red-600)'}} />;
  }, []);

  const closeCellEditor = useCallback((event: React.FocusEvent<HTMLElement>) => {
    const cell = event.currentTarget.closest('td');
    if (!cell) return;
    const relatedTarget = event.relatedTarget as HTMLElement | null;
    if (relatedTarget) {
      if (cell.contains(relatedTarget)) return;
      if (relatedTarget.closest('.p-datepicker, .p-datepicker-panel, .p-datepicker-calendar, .p-datepicker-group')) return;
    }

    setTimeout(() => {
      const activeElement = document.activeElement as HTMLElement | null;
      if (!activeElement) return;
      if (cell.contains(activeElement)) return;
      if (activeElement.closest('.p-datepicker, .p-datepicker-panel, .p-datepicker-calendar, .p-datepicker-group')) return;

      const enterEvent = new KeyboardEvent('keydown', {
        bubbles: true,
        cancelable: true,
        key: 'Enter',
        code: 'Enter',
      });
      cell.dispatchEvent(enterEvent);
    }, 50);
  }, []);

  const textEditor = useCallback((options: ColumnEditorOptions) => {
    return <InputText
        type="text" value={options.value} className="p-inputtext-sm"
        onChange={(e) => options.editorCallback?.(e.target.value)}
        onBlur={(event) => {
          options.editorCallback?.(event.target.value);
          closeCellEditor(event);
        }}
    />;
  }, [closeCellEditor]);

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
            : typeof event.value === 'string'
              ? event.value
              : '';
          options.editorCallback?.(nextValue);
        }}
        onBlur={(event) => {
          closeCellEditor(event);
        }}
        dateFormat="yy-mm-dd"
        placeholder="yyyy-mm-dd"
        showIcon
        className="p-inputtext-sm"
      />
    );
  }, [closeCellEditor]);

  const onCellEditComplete = useCallback((e: ColumnEvent) => {
    const { rowData, newValue, field, originalEvent: event } = e;
    rowData[field] = newValue;
    console.log(event);
  }, []);

  const cellEditor = useCallback((options: ColumnEditorOptions) => {
    if (options.field !== 'value')
      return null;
    if (options.rowData?.dataType === 'date')
      return dateEditor(options);
    return textEditor(options);
  }, [dateEditor, textEditor]);

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
    const original = doc?.metadata ?? {};
    const updates = rows
      .filter((row) => (original[row.slug] ?? '') !== row.value)
      .map((row) => ({ metadata_type_id: row.metadataTypeId, value: row.value }));

    try {
      await saveDocumentMetadata.mutateAsync(updates);
      navigate('/documents');
    } catch (err) {
      console.error(err);
    }
  };

  if (!id)
    return <Message severity="error" text="Missing or invalid ID" />;
  if (isError)
    return <Message severity="error" text={error.message} />
  if (isLoading)
    return <div>Loading</div>;

  return (
    <Card title="Document Metadata">
      <DataTable value={rows} editMode="cell" tableStyle={{ minWidth: '50rem' }}>
        {columns}
      </DataTable>

      <div className="text-end">
        {saveDocumentMetadata.isError && (
          <Message className="float-start" severity="error" text={saveDocumentMetadata.error.message} />
        )}

        <Button label="Save" type="submit" icon="pi pi-check" onClick={onSave} disabled={saveDocumentMetadata.isPending} />
        <Button label="Cancel" type="button" severity="secondary" icon="pi pi-times" style={{ marginLeft: '0.5em' }} onClick={() => navigate('/documents')} />
      </div>
    </Card>
  );
}
