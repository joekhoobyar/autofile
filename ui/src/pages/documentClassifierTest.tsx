import { useMemo, useState } from 'react';

import { Button } from 'primereact/button';
import { Card } from 'primereact/card';
import { Dropdown } from 'primereact/dropdown';
import { Message } from 'primereact/message';
import { stringify } from 'yaml';

import { DocumentViewLayout } from '../components/DocumentViewLayout';
import { useClassifierBlocks } from '../queries/useClassifierBlocks';
import { useDocument, useTestClassifierBlock } from '../queries/useDocuments';
import { useId } from '../util';

export function DocumentClassifierTest() {
  const documentId = useId('id');
  const { data: document, isLoading: isDocumentLoading, isError: isDocumentError, error: documentError } = useDocument(documentId);
  const { data: classifierBlocks, isLoading: isBlocksLoading, isError: isBlocksError, error: blocksError, isFetching: isBlocksFetching } = useClassifierBlocks({ page: 1, per_page: 1000, sf: 'order' });
  const testClassifierBlock = useTestClassifierBlock();

  const [selectedClassifierBlockId, setSelectedClassifierBlockId] = useState<number | null>(null);
  const [resultYaml, setResultYaml] = useState('');
  const [copyStatus, setCopyStatus] = useState<'idle' | 'copied' | 'error'>('idle');

  const classifierOptions = useMemo(
    () => (classifierBlocks?.items ?? [])
      .map((block) => ({ label: block.name, value: block.id }))
      .sort((a, b) => a.label.localeCompare(b.label, undefined, { sensitivity: 'base' })),
    [classifierBlocks?.items]
  );

  const onSubmit = async () => {
    if (!selectedClassifierBlockId) {
      return;
    }

    const result = await testClassifierBlock.mutateAsync({
      document_id: documentId,
      classifier_block_id: selectedClassifierBlockId,
    });

    const payload = {
      document_id: documentId,
      classifier_block_id: selectedClassifierBlockId,
      generated_at: new Date().toISOString(),
      computed_actions: result.computed_actions,
    };

    setResultYaml(stringify(payload, { indent: 2 }).trim());
    setCopyStatus('idle');
  };

  const copyYaml = async () => {
    if (!resultYaml) {
      return;
    }

    try {
      await navigator.clipboard.writeText(resultYaml);
      setCopyStatus('copied');
    } catch {
      setCopyStatus('error');
    }
  };

  if (isDocumentError) {
    return <Message severity="error" text={documentError.message} />;
  }

  if (isDocumentLoading) {
    return <div>Loading</div>;
  }

  return (
    <DocumentViewLayout documentId={documentId}>
      <Card title={`Classifier Test${document?.title ? `: ${document.title}` : ''}`}>
        {isBlocksError && <Message severity="error" text={blocksError.message} className="mb-3" />}

        <div className="flex flex-column gap-3">
          <div className="flex flex-column gap-2 md:flex-row md:align-items-end">
            <div className="flex flex-column gap-2 w-full md:w-20rem">
              <label htmlFor="classifier_block_id" className="font-medium">Classifier Block</label>
              <Dropdown
                id="classifier_block_id"
                value={selectedClassifierBlockId}
                onChange={(event) => setSelectedClassifierBlockId((event.value as number) ?? null)}
                options={classifierOptions}
                optionLabel="label"
                optionValue="value"
                filter
                filterBy="label"
                filterPlaceholder="Search classifier blocks"
                showClear
                placeholder={isBlocksLoading ? 'Loading classifier blocks...' : 'Select a classifier block'}
                loading={isBlocksLoading || isBlocksFetching}
                className="w-full"
              />
            </div>

            <Button
              label="Run Test"
              icon="pi pi-play"
              onClick={() => void onSubmit()}
              disabled={!selectedClassifierBlockId || testClassifierBlock.isPending || classifierOptions.length === 0}
              loading={testClassifierBlock.isPending}
            />
          </div>

          {!isBlocksLoading && classifierOptions.length === 0 && (
            <Message severity="info" text="No classifier blocks are available." />
          )}

          {testClassifierBlock.isError && (
            <Message severity="error" text={testClassifierBlock.error.message} />
          )}

          {resultYaml && (
            <>
              <div className="flex flex-wrap gap-2 align-items-center">
                <Button label={copyStatus === 'copied' ? 'Copied' : 'Copy YAML'} icon="pi pi-copy" severity="secondary" outlined onClick={() => void copyYaml()} />
                {copyStatus === 'error' && (
                  <Message severity="error" text="Failed to copy YAML to clipboard." />
                )}
              </div>
              <pre className="aut-document-text-content">{resultYaml}</pre>
            </>
          )}
        </div>
      </Card>
    </DocumentViewLayout>
  );
}
