
drop trigger if exists document_index_templates_prevent_cycles_trg on document_index_templates;
drop function if exists document_index_templates_prevent_cycles();

drop trigger if exists document_index_values_prevent_cycles_trg on document_index_values;
drop function if exists document_index_values_prevent_cycles();
