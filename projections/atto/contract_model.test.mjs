import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync, readdirSync } from 'node:fs';

const root = new URL('../../', import.meta.url);
const source = readFileSync(new URL('./flag_catalog.atto', import.meta.url), 'utf8');
const manifest = JSON.parse(readFileSync(new URL('./manifest.json', import.meta.url), 'utf8'));
const schema = JSON.parse(readFileSync(new URL('schema/v1/flagcatalog.json', root), 'utf8'));
const typespec = readFileSync(new URL('schema/v1/flagcatalog.tsp', root), 'utf8');
const compact = source.replace(/\s+/g, ' ');
const keys = ['id', 'revision', 'payload'];

function isObject(value) {
  return value !== null && typeof value === 'object' && !Array.isArray(value);
}

function catalogShape(idIsString, idNonempty, revisionIsString, revisionNonempty, payloadIsObject, noUnknownProperties) {
  return idIsString && idNonempty && revisionIsString && revisionNonempty && payloadIsObject && noUnknownProperties;
}

function validateCatalogObject(value) {
  if (!isObject(value)) return false;
  return catalogShape(
    typeof value.id === 'string',
    typeof value.id === 'string' && value.id.length >= 1,
    typeof value.revision === 'string',
    typeof value.revision === 'string' && value.revision.length >= 1,
    isObject(value.payload),
    keys.every((key) => Object.hasOwn(value, key)) &&
      Object.keys(value).every((key) => keys.includes(key))
  );
}

test('canonical authored schema and TypeSpec remain the peer authorities', () => {
  assert.equal(schema.type, 'object');
  assert.equal(schema.additionalProperties, false);
  assert.deepEqual([...schema.required].sort(), [...keys].sort());
  for (const key of ['id', 'revision']) {
    assert.equal(schema.properties[key].type, 'string');
    assert.equal(schema.properties[key].minLength, 1);
  }
  assert.equal(schema.properties.payload.type, 'object');
  assert.match(typespec, /model FlagCatalog/);
  assert.match(typespec, /@minLength\(1\)/);
  assert.match(typespec, /payload:/);
  assert.equal(manifest.peer_authorities, true);
  assert.equal(manifest.is_authority, false);
});

test('source predicates match the bounded host oracle and remain unverified by compiler', () => {
  assert.match(source, /define module flags_2_env_contracts as/);
  assert.ok(compact.includes('return id_is_string && id_nonempty && revision_is_string && revision_nonempty && payload_is_object && no_unknown_properties;'));
  assert.ok(compact.includes('return ok_is_boolean && service_is_string && protocol_is_string;'));
  assert.equal(manifest.status, 'experimental-source-only');
  assert.equal(manifest.compile_verified, false);
  assert.equal(manifest.runtime_verified, false);
  assert.equal(manifest.parser_implemented, false);
  for (let mask = 0; mask < 64; mask += 1) {
    const bits = Array.from({ length: 6 }, (_, i) => Boolean(mask & (1 << i)));
    assert.equal(catalogShape(...bits), mask === 63, `mask=${mask}`);
  }
});

test('all authored valid/invalid catalog fixtures respect the projected object shape', () => {
  for (const kind of ['valid', 'invalid']) {
    const directory = new URL(`schema/v1/instances/FlagCatalog/${kind}/`, root);
    const files = readdirSync(directory).filter((name) => name.endsWith('.json'));
    assert.ok(files.length >= 1, `empty ${kind} fixture corpus`);
    for (const name of files) {
      const fixture = JSON.parse(readFileSync(new URL(name, directory), 'utf8'));
      assert.equal(validateCatalogObject(fixture), kind === 'valid', `${kind}/${name}`);
    }
  }
});

test('payload typing, unknown root fields, and boolean Health shape are explicit', () => {
  const valid = { id: 'x', revision: '1', payload: { nested: [1, true, null] } };
  assert.equal(validateCatalogObject(valid), true);
  assert.equal(validateCatalogObject({ ...valid, id: '' }), false);
  assert.equal(validateCatalogObject({ ...valid, payload: [] }), false);
  assert.equal(validateCatalogObject({ ...valid, unexpected: true }), false);
  assert.equal(validateCatalogObject({ ...valid, id: ' ' }), true);
  for (let mask = 0; mask < 8; mask += 1) {
    const bits = Array.from({ length: 3 }, (_, i) => Boolean(mask & (1 << i)));
    assert.equal(bits.every(Boolean), mask === 7);
  }
});
