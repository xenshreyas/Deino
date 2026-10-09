/**
 * Stable IDs for fixture rows, identical to `deino_testkit::fixture::fixture_uuid`
 * (`crates/deino-testkit/src/fixture/ids.rs`): two FNV-1a hashes of a readable
 * key with the RFC 9562 version 8 and variant bits set. The mock serves the
 * same IDs the Rust seeder writes, so a test can name a row in both.
 */

const FNV_OFFSET = 0xcbf29ce484222325n;
const FNV_PRIME = 0x100000001b3n;
const U64 = 0xffffffffffffffffn;
const VERSION_MASK = 0xffffffffffff0fffn;
const VERSION_8 = 0x8000n;
const VARIANT_MASK = 0x3fffffffffffffffn;
const VARIANT_RFC = 0x8000000000000000n;

/** The 64-bit FNV-1a hash of `key`'s UTF-8 bytes. */
function fnv1a(key: string): bigint {
  let hash = FNV_OFFSET;
  for (const byte of new TextEncoder().encode(key)) {
    hash = ((hash ^ BigInt(byte)) * FNV_PRIME) & U64;
  }
  return hash;
}

/** `value` as lower-case hex, left-padded to `digits`. */
function hex(value: bigint, digits: number): string {
  return value.toString(16).padStart(digits, '0');
}

/** The fixture UUID for `key`, for example `segment/<meeting id>/<index>`. */
export function fixtureUuid(key: string): string {
  const high = (fnv1a(key) & VERSION_MASK) | VERSION_8;
  const low = (fnv1a(`${key}#`) & VARIANT_MASK) | VARIANT_RFC;
  return [
    hex(high >> 32n, 8),
    hex((high >> 16n) & 0xffffn, 4),
    hex(high & 0xffffn, 4),
    hex(low >> 48n, 4),
    hex(low & 0xffffffffffffn, 12),
  ].join('-');
}
