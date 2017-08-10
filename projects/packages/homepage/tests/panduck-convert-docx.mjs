import assert from "node:assert/strict";
import { mkdtemp, readFile, writeFile } from "node:fs/promises";
import { spawnSync } from "node:child_process";
import { tmpdir } from "node:os";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const panduckBin = join(dirname(fileURLToPath(import.meta.url)), "../../panduck/bin/panduck.mjs");

function crc32(data) {
    let crc = 0xffffffff;
    for (const byte of data) {
        crc ^= byte;
        for (let bit = 0; bit < 8; bit += 1) {
            const mask = -(crc & 1);
            crc = (crc >>> 1) ^ (0xedb88320 & mask);
        }
    }
    return (crc ^ 0xffffffff) >>> 0;
}

function storedZip(path, payload) {
    const name = Buffer.from(path, "utf8");
    const crc = crc32(payload);
    const chunks = [];

    chunks.push(Buffer.from("PK\x03\x04"));
    chunks.push(Buffer.from([0x14, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]));
    const localTail = Buffer.alloc(8);
    localTail.writeUInt32LE(crc, 0);
    localTail.writeUInt32LE(payload.length, 4);
    chunks.push(localTail);
    chunks.push(Buffer.from(new Uint32Array([payload.length]).buffer));
    const nameLen = Buffer.alloc(4);
    nameLen.writeUInt16LE(name.length, 0);
    nameLen.writeUInt16LE(0, 2);
    chunks.push(nameLen);
    chunks.push(name);
    chunks.push(payload);

    const cdOffset = Buffer.concat(chunks).length;
    chunks.push(Buffer.from("PK\x01\x02"));
    const cdFixed = Buffer.alloc(46);
    cdFixed.writeUInt16LE(0x0014, 0);
    cdFixed.writeUInt16LE(0x0014, 2);
    cdFixed.writeUInt32LE(crc, 12);
    cdFixed.writeUInt32LE(payload.length, 16);
    cdFixed.writeUInt32LE(payload.length, 20);
    cdFixed.writeUInt16LE(name.length, 24);
    cdFixed.writeUInt32LE(0, 38);
    chunks.push(cdFixed);
    chunks.push(name);

    const cdSize = Buffer.concat(chunks).length - cdOffset;
    chunks.push(Buffer.from("PK\x05\x06"));
    chunks.push(Buffer.from([0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x01, 0x00]));
    const eocdTail = Buffer.alloc(8);
    eocdTail.writeUInt32LE(cdSize, 0);
    eocdTail.writeUInt32LE(cdOffset, 4);
    chunks.push(eocdTail);
    chunks.push(Buffer.from([0x00, 0x00]));
    return Buffer.concat(chunks);
}

const documentXml = Buffer.from(`<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p><w:r><w:t>Hello DOCX</w:t></w:r></w:p>
    <w:p>
      <w:pPr><w:pStyle w:val="Heading1"/></w:pPr>
      <w:r><w:t>Title</w:t></w:r>
    </w:p>
  </w:body>
</w:document>`, "utf8");

const workdir = await mkdtemp(join(tmpdir(), "panduck-docx-"));
const input = join(workdir, "sample.docx");
const output = join(workdir, "sample.md");
await writeFile(input, storedZip("word/document.xml", documentXml));

const result = spawnSync(process.execPath, [
    panduckBin,
    "convert",
    input,
    "--to",
    "markdown",
    "-o",
    output,
    "--diagnostics",
    "silent",
], { encoding: "utf8" });

assert.equal(result.status, 0, result.stderr || result.stdout);
const markdown = await readFile(output, "utf8");
assert.match(markdown, /Hello DOCX/);
assert.match(markdown, /# Title/);
console.log("panduck-convert-docx ok");
