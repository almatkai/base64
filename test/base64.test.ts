import assert from "node:assert/strict";
import test from "node:test";
import { decodeBase64, encodeBase64, isValidBase64 } from "../src/lib/base64";

test("encodes and decodes Unicode text", () => {
  const input = "Tezbar on Windows — Сәлем 🌍";
  const encoded = encodeBase64(input);
  assert.equal(decodeBase64(encoded), input);
});

test("accepts padded and unpadded Base64", () => {
  assert.equal(isValidBase64("aGVsbG8="), true);
  assert.equal(isValidBase64("aGVsbG8"), true);
  assert.equal(decodeBase64("aGVsbG8"), "hello");
});

test("rejects malformed Base64", () => {
  assert.equal(isValidBase64("not base64!"), false);
  assert.throws(() => decodeBase64("%%%"));
});
