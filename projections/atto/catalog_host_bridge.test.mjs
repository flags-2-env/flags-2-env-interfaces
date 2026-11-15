import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';

const root = new URL('../../', import.meta.url);
const schema = JSON.parse(readFileSync(new URL('schema/v1/flagcatalog.json', root), 'utf8'));
const source = readFileSync(new URL('src/atto_projection.rs', root), 'utf8');

// This test is only an independent host-source oracle, not Rust execution.
function accepts(value) {
  return value !== null && typeof value === 'object' && !Array.isArray(value) &&
    Object.keys(value).length === 3 &&
    ['id','revision'].every(k => typeof value[k] === 'string' && value[k].length >= 1) &&
    value.payload !== null && typeof value.payload === 'object' && !Array.isArray(value.payload);
}
test('bridge enforces deny-unknown root typing and byte limits', () => {
  assert.match(source, /#\[serde\(deny_unknown_fields\)\]/);
  assert.match(source, /payload:\s*Map<String, Value>/);
  assert.match(source, /MAX_CATALOG_BYTES:\s*usize\s*=\s*1024\s*\*\s*1024/);
  assert.match(source, /bytes\.len\(\) > byte_budget/);
  assert.match(source, /serde_json::from_slice/);
  assert.match(source, /decoded\.id\.is_empty\(\)/);
  assert.match(source, /decoded\.revision\.is_empty\(\)/);
});
test('schema authority has the expected shared root contract', () => {
  assert.equal(schema.type,'object');
  assert.equal(schema.additionalProperties,false);
  assert.deepEqual([...schema.required].sort(),['id','payload','revision']);
  assert.equal(schema.properties.id.type,'string');
  assert.equal(schema.properties.revision.type,'string');
  assert.equal(schema.properties.id.minLength,1);
  assert.equal(schema.properties.revision.minLength,1);
  assert.equal(schema.properties.payload.type,'object');
});
test('882 exhaustive object combinations follow root schema policy', () => {
  const names = [undefined,null,'',' ','valid',0,false];
  const payloads = [undefined,null,{}, {x:[1,true,null]}, [], 1, 'x', false, true];
  let cases = 0, admitted = 0;
  for (const id of names)
    for (const revision of names)
      for (const payload of payloads)
        for (const withExtra of [false,true]) {
          const object = {};
          if (id !== undefined) object.id = id;
          if (revision !== undefined) object.revision = revision;
          if (payload !== undefined) object.payload = payload;
          if (withExtra) object.unexpected = true;
          const expected = typeof id === 'string' && id.length > 0 &&
            typeof revision === 'string' && revision.length > 0 &&
            payload !== null && payload !== undefined &&
            typeof payload === 'object' && !Array.isArray(payload) && !withExtra;
          assert.equal(accepts(object), expected, JSON.stringify(object));
          cases += 1;
          if (expected) admitted += 1;
        }
  assert.equal(cases, 882);
  assert.equal(admitted, 8);
});
