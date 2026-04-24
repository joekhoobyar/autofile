import { useState } from 'react';

import { Button } from 'primereact/button';
import { Card } from 'primereact/card';
import { InputTextarea } from 'primereact/inputtextarea';
import { Message } from 'primereact/message';

import { DocumentViewLayout } from '../components/DocumentViewLayout';
import { useDocument, useTestTemplate } from '../queries/useDocuments';
import { useId } from '../util';

export function DocumentTemplateTest() {
  const documentId = useId('id');
  const { data: document, isLoading: isDocumentLoading, isError: isDocumentError, error: documentError } = useDocument(documentId);
  const testTemplate = useTestTemplate();

  const [template, setTemplate] = useState('');
  const [rendered, setRendered] = useState('');
  const [renderError, setRenderError] = useState('');

  const onSubmit = async () => {
    const result = await testTemplate.mutateAsync({
      document_id: documentId,
      template,
    });

    setRendered(result.rendered ?? '');
    setRenderError(result.error ?? '');
  };

  if (isDocumentError) {
    return <Message severity="error" text={documentError.message} />;
  }

  if (isDocumentLoading) {
    return <div>Loading</div>;
  }

  return (
    <DocumentViewLayout documentId={documentId}>
      <Card title={`Template Test${document?.title ? `: ${document.title}` : ''}`}>
        <div className="flex flex-column gap-3">
          <div className="flex flex-column gap-2">
            <label htmlFor="template" className="font-medium">Template</label>
            <InputTextarea
              id="template"
              value={template}
              onChange={(event) => setTemplate(event.target.value)}
              rows={8}
              autoResize
              className="w-full"
              placeholder="Enter a MiniJinja template, e.g. {{ doc.title }}"
            />
          </div>

          <div>
            <Button
              label="Run Test"
              icon="pi pi-play"
              onClick={() => void onSubmit()}
              disabled={template.trim().length === 0 || testTemplate.isPending}
              loading={testTemplate.isPending}
            />
          </div>

          {testTemplate.isError && (
            <Message severity="error" text={testTemplate.error.message} />
          )}

          {!testTemplate.isError && renderError && (
            <Message severity="error" text={renderError} />
          )}

          {!testTemplate.isError && rendered && (
            <pre className="aut-document-text-content">{rendered}</pre>
          )}
        </div>
      </Card>
    </DocumentViewLayout>
  );
}
