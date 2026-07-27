const MAX_CRYPTO_CHUNK_BYTES = 65_536;

export function installWasiRandomGet(wasi) {
  wasi.wasiImport.random_get = function randomGet(pointer, byteLength) {
    const memory = new Uint8Array(
      wasi.inst.exports.memory.buffer,
      pointer,
      byteLength,
    );

    for (let offset = 0; offset < byteLength; offset += MAX_CRYPTO_CHUNK_BYTES) {
      const chunk = new Uint8Array(
        Math.min(MAX_CRYPTO_CHUNK_BYTES, byteLength - offset),
      );
      globalThis.crypto.getRandomValues(chunk);
      memory.set(chunk, offset);
    }
    return 0;
  };
}
