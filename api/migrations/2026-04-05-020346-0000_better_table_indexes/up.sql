CREATE INDEX cabinet_documents_document_id ON cabinet_documents(document_id);

CREATE INDEX document_files_document_id ON document_files(document_id);

CREATE INDEX document_metadatas_document_id ON document_metadatas(document_id);
CREATE INDEX document_metadatas_value ON document_metadatas(value);

CREATE INDEX documents_created_at ON documents(created_at);
CREATE INDEX documents_title ON documents(title);

CREATE INDEX tag_documents_document_id ON tag_documents(document_id);
