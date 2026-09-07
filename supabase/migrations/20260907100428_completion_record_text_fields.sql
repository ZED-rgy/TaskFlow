-- Keep the wire payload deserializable even for malformed direct RPC calls.
alter table private.completion_records add constraint completion_record_text_fields check (
  jsonb_typeof(body->'title') is not distinct from 'string' and
  jsonb_typeof(body->'projectName') is not distinct from 'string'
);
