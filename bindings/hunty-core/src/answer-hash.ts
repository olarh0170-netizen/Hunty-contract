import { Buffer } from "buffer";
import { hash } from "@stellar/stellar-sdk";

/** Maximum answer length accepted by the contract, in bytes of UTF-8. */
export const MAX_ANSWER_BYTES = 256;

/**
 * Normalizes a clue answer exactly like `hunty-core` does before hashing.
 *
 * - ASCII whitespace (`space`, `\t`, `\n`, `\r`) is trimmed from both ends.
 * - Internal runs of ASCII whitespace collapse to a single space, so
 *   `"New   York"` and `"new york"` normalize identically.
 * - Only ASCII `A-Z` is lowercased. Non-ASCII characters are left untouched:
 *   the contract cannot case-fold Unicode on-chain, so `"ÉCOLE"` normalizes to
 *   `"École"`, not `"école"`.
 */
export function normalizeAnswer(answer: string): string {
  let normalized = "";
  let pendingSpace = false;

  for (let i = 0; i < answer.length; i += 1) {
    const char = answer[i];
    if (char === " " || char === "\t" || char === "\n" || char === "\r") {
      pendingSpace = normalized.length > 0;
      continue;
    }
    if (pendingSpace) {
      normalized += " ";
      pendingSpace = false;
    }
    const code = char.charCodeAt(0);
    normalized +=
      code >= 0x41 && code <= 0x5a ? String.fromCharCode(code + 0x20) : char;
  }

  return normalized;
}

/**
 * Builds the `answer_hash` accepted by `submit_answer_with_hash`:
 * `SHA256(hunt_id || clue_id || normalized_answer)`, where `hunt_id` is a
 * big-endian `u64`, `clue_id` a big-endian `u32` and the normalized answer is
 * UTF-8. This is the same preimage `normalize_and_hash_answer` hashes on-chain.
 *
 * @throws if the normalized answer is empty or longer than 256 UTF-8 bytes,
 * in which case the contract would reject it as `InvalidAnswer`.
 */
export function hashAnswer(
  huntId: bigint | number,
  clueId: number,
  answer: string,
): Buffer {
  const normalized = normalizeAnswer(answer);
  const normalizedBytes = Buffer.from(normalized, "utf8");
  if (normalizedBytes.length === 0) {
    throw new Error("Answer is empty after normalization");
  }
  if (normalizedBytes.length > MAX_ANSWER_BYTES) {
    throw new Error(
      `Answer is ${normalizedBytes.length} bytes after normalization; the contract accepts at most ${MAX_ANSWER_BYTES}`,
    );
  }

  // The `buffer` polyfill used by this package has no BigInt write helpers,
  // so write the big-endian u64 hunt id as two u32 halves.
  const id = BigInt(huntId);
  const preimage = Buffer.alloc(12 + normalizedBytes.length);
  preimage.writeUInt32BE(Number((id >> 32n) & 0xffffffffn), 0);
  preimage.writeUInt32BE(Number(id & 0xffffffffn), 4);
  preimage.writeUInt32BE(Number(clueId) >>> 0, 8);
  normalizedBytes.copy(preimage, 12);

  return Buffer.from(hash(preimage));
}
