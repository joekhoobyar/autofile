-- Your SQL goes here

CREATE TABLE document_types_metadata_types (
  document_type_id INT8 NOT NULL REFERENCES document_types(id),
  metadata_type_id INT8 NOT NULL REFERENCES metadata_types(id),
  PRIMARY KEY(document_type_id, metadata_type_id)
);
