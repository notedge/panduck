import assert from 'node:assert/strict';
import { loadPanduckNode } from '@notedge/panduck/node';

const binding = loadPanduckNode();
const formats = binding.supportedFormats();
assert.ok(binding.panduckVersion().length > 0, 'panduckVersion');
assert.ok(formats.includes('markdown'), 'supportedFormats');
assert.equal(binding.isSupportedFormat('markdown'), true, 'isSupportedFormat');
console.log(`panduck-probe ok: version=${binding.panduckVersion()} formats=${formats.join(',')}`);
