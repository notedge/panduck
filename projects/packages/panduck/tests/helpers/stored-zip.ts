export function crc32(data: Uint8Array | Buffer): number {
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

/** Builds a minimal stored ZIP archive with one or more members. */
export function storedZip(entries: ReadonlyArray<{ path: string; payload: Uint8Array | Buffer }>): Buffer {
    const archive: Buffer[] = [];
    const central: Buffer[] = [];

    for (const entry of entries) {
        const name = Buffer.from(entry.path, "utf8");
        const payload = Buffer.from(entry.payload);
        const crc = crc32(payload);
        const localOffset = Buffer.concat(archive).length;

        archive.push(Buffer.from("PK\x03\x04"));
        archive.push(Buffer.from([0x14, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]));
        const localTail = Buffer.alloc(8);
        localTail.writeUInt32LE(crc, 0);
        localTail.writeUInt32LE(payload.length, 4);
        archive.push(localTail);
        archive.push(Buffer.from(new Uint32Array([payload.length]).buffer));
        const nameLen = Buffer.alloc(4);
        nameLen.writeUInt16LE(name.length, 0);
        nameLen.writeUInt16LE(0, 2);
        archive.push(nameLen);
        archive.push(name);
        archive.push(payload);

        central.push(Buffer.from("PK\x01\x02"));
        const cdFixed = Buffer.alloc(46);
        cdFixed.writeUInt16LE(0x0014, 0);
        cdFixed.writeUInt16LE(0x0014, 2);
        cdFixed.writeUInt32LE(crc, 12);
        cdFixed.writeUInt32LE(payload.length, 16);
        cdFixed.writeUInt32LE(payload.length, 20);
        cdFixed.writeUInt16LE(name.length, 24);
        cdFixed.writeUInt32LE(localOffset, 38);
        central.push(cdFixed);
        central.push(name);
    }

    const cdOffset = Buffer.concat(archive).length;
    archive.push(...central);
    const cdSize = Buffer.concat(central).length;
    archive.push(Buffer.from("PK\x05\x06"));
    archive.push(Buffer.from([0x00, 0x00, 0x00, 0x00]));
    archive.push(Buffer.from(new Uint16Array([entries.length, entries.length]).buffer));
    const eocdTail = Buffer.alloc(8);
    eocdTail.writeUInt32LE(cdSize, 0);
    eocdTail.writeUInt32LE(cdOffset, 4);
    archive.push(eocdTail);
    archive.push(Buffer.from([0x00, 0x00]));
    return Buffer.concat(archive);
}
