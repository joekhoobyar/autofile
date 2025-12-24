-- Your SQL goes here

create or replace function cabinets_prevent_cycles()
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
    from cabinets
    where id = cur;
  end loop;

  return new;
end;
$$ language plpgsql;

drop trigger if exists cabinets_prevent_cycles_trg on cabinets;

create trigger cabinets_prevent_cycles_trg
before insert or update of parent_id on cabinets
for each row
execute function cabinets_prevent_cycles();
