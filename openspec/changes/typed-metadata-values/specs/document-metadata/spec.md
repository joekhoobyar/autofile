## ADDED Requirements

### Requirement: Typed document metadata storage
The system SHALL store each document metadata row in exactly one typed value column: `string_value`, `number_value`, or `date_value`.

#### Scenario: Store string metadata
- **WHEN** a user saves metadata for a Metadata Type with data type `string`
- **THEN** the system stores the submitted value in `string_value`
- **AND** `number_value` and `date_value` are null

#### Scenario: Store lookup metadata
- **WHEN** a user saves metadata for a Metadata Type with data type `lookup`
- **THEN** the system validates the value against configured lookup choices
- **AND** stores the submitted value in `string_value`
- **AND** `number_value` and `date_value` are null

#### Scenario: Store date metadata
- **WHEN** a user saves metadata for a Metadata Type with data type `date`
- **THEN** the system validates the submitted date as `YYYY-MM-DD`
- **AND** stores the value in `date_value` as a date
- **AND** `string_value` and `number_value` are null

### Requirement: Typed document metadata API
The system SHALL expose document metadata row request and response bodies with typed value fields instead of a single `value` field.

#### Scenario: Return typed metadata row
- **WHEN** a client requests stored metadata rows for a document
- **THEN** each returned row includes `string_value`, `number_value`, and `date_value`
- **AND** each row has exactly one non-null typed value field

#### Scenario: Reject multiple submitted value fields
- **WHEN** a client submits a document metadata upsert item with more than one non-null typed value field
- **THEN** the system rejects the request with `422 Unprocessable Entity`

#### Scenario: Reject value field that does not match metadata type
- **WHEN** a client submits a document metadata upsert item whose non-null typed value field does not match the Metadata Type data type
- **THEN** the system rejects the request with `422 Unprocessable Entity`

#### Scenario: Reject number value before number metadata exists
- **WHEN** a client submits a non-null `number_value`
- **THEN** the system rejects the request with `422 Unprocessable Entity`

#### Scenario: Delete optional metadata with empty typed values
- **WHEN** a client submits an optional document metadata upsert item with all typed value fields null or empty
- **THEN** the system deletes the stored metadata row for that field if it exists

### Requirement: String document metadata view representation
The system SHALL keep document detail and list metadata maps as slug-keyed strings for display, templates, classifiers, and document index inputs.

#### Scenario: Return document view metadata strings
- **WHEN** the system builds a document view containing stored metadata
- **THEN** `string_value` metadata appears as the stored string
- **AND** `date_value` metadata appears as a `YYYY-MM-DD` string

### Requirement: Classifier metadata writes use typed metadata validation
The system SHALL persist classifier-produced metadata through the same typed metadata validation and storage rules as document metadata API upserts.

#### Scenario: Classifier writes date metadata
- **WHEN** a classifier block produces a metadata action for a Metadata Type with data type `date`
- **AND** the produced value is a valid `YYYY-MM-DD` date string
- **THEN** the system stores the value in `date_value` as a date
- **AND** `string_value` and `number_value` are null

#### Scenario: Classifier rejects invalid date metadata
- **WHEN** a classifier block produces a metadata action for a Metadata Type with data type `date`
- **AND** the produced value is not a valid `YYYY-MM-DD` date string
- **THEN** the system rejects the metadata write with the same validation behavior as an invalid document metadata API upsert

### Requirement: String-only metadata value search
The system SHALL apply document list `metadata_value` filtering only to string metadata values.

#### Scenario: Search string metadata values
- **WHEN** a client lists documents with `metadata_value` set
- **THEN** the system matches documents by case-insensitive partial match against `string_value`

#### Scenario: Do not search date metadata values
- **WHEN** a document has matching text only in `date_value`
- **AND** a client lists documents with `metadata_value` set
- **THEN** the system does not match that document because of the date value

#### Scenario: Metadata type existence filter remains typed-value agnostic
- **WHEN** a client lists documents with `metadata_type_id` set and no `metadata_value`
- **THEN** the system matches documents that have a stored row for that Metadata Type regardless of which typed value column is populated

### Requirement: String metadata value suggestions
The system SHALL provide metadata value suggestions from string metadata values only.

#### Scenario: Suggest string metadata values
- **WHEN** a client requests value suggestions for a string Metadata Type
- **THEN** the system returns distinct non-empty `string_value` values for that Metadata Type

#### Scenario: Reject non-string suggestions
- **WHEN** a client requests value suggestions for a non-string Metadata Type
- **THEN** the system rejects the request with `422 Unprocessable Entity`
