create or replace function document_index_templates_prevent_cycles()
returns trigger as $$
declare
  cur bigint;
begin
  -- allow roots
  if new.parent_id is null then
    return new;
  end if;

  -- disallow self-parenting
  if new.parent_id = new.id then
    raise exception 'cycle: node cannot be its own parent';
  end if;

  -- walk up the parent chain from the proposed parent
  cur := new.parent_id;
  while cur is not null loop
    if cur = new.id then
      raise exception 'cycle: would create an ancestor loop';
    end if;

    select parent_id into cur
    from document_index_templates
    where id = cur;
  end loop;

  return new;
end;
$$ language plpgsql;

drop trigger if exists document_index_templates_prevent_cycles_trg on document_index_templates;

create trigger document_index_templates_prevent_cycles_trg
before insert or update of parent_id on document_index_templates
for each row
execute function document_index_templates_prevent_cycles();

create or replace function document_index_values_prevent_cycles()
returns trigger as $$
declare
  cur bigint;
begin
  -- allow roots
  if new.parent_id is null then
    return new;
  end if;

  -- disallow self-parenting
  if new.parent_id = new.id then
    raise exception 'cycle: node cannot be its own parent';
  end if;

  -- walk up the parent chain from the proposed parent
  cur := new.parent_id;
  while cur is not null loop
    if cur = new.id then
      raise exception 'cycle: would create an ancestor loop';
    end if;

    select parent_id into cur
    from document_index_values
    where id = cur;
  end loop;

  return new;
end;
$$ language plpgsql;

drop trigger if exists document_index_values_prevent_cycles_trg on document_index_values;

create trigger document_index_values_prevent_cycles_trg
before insert or update of parent_id on document_index_values
for each row
execute function document_index_values_prevent_cycles();
