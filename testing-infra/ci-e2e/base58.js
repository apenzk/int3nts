const fs = require("fs");

const ALPHABET = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

/**
 * Encode a byte buffer into a base58 string using the Bitcoin alphabet.
 * Preserves leading zero bytes as leading "1" characters.
 */
function base58Encode(buf) {
  let num = 0n;
  for (const b of buf) {
    num = (num << 8n) + BigInt(b);
  }
  let enc = "";
  while (num > 0n) {
    const rem = num % 58n;
    num /= 58n;
    enc = ALPHABET[Number(rem)] + enc;
  }
  let nPad = 0;
  for (const b of buf) {
    if (b !== 0) break;
    nPad++;
  }
  return "1".repeat(nPad) + enc;
}

/**
 * Decode a base58 string using the Bitcoin alphabet into a Buffer.
 * Preserves leading "1" characters as leading zero bytes.
 */
function base58Decode(str) {
  let num = 0n;
  for (const c of str) {
    const idx = ALPHABET.indexOf(c);
    if (idx < 0) {
      throw new Error("Invalid base58 character");
    }
    num = num * 58n + BigInt(idx);
  }
  let bytes = [];
  while (num > 0n) {
    bytes.push(Number(num & 0xffn));
    num >>= 8n;
  }
  bytes = bytes.reverse();
  let nPad = 0;
  for (const c of str) {
    if (c !== "1") break;
    nPad++;
  }
  return Buffer.concat([Buffer.alloc(nPad), Buffer.from(bytes)]);
}

/**
 * Print CLI usage and exit with a non-zero status.
 */
function usage() {
  console.error(
    "Usage: node base58.js <encode-base64|encode-keypair-json|decode-base58-to-hex> <input>"
  );
  process.exit(1);
}

const [, , command, input] = process.argv;

if (!command || !input) {
  usage();
}

if (command === "encode-base64") {
  const raw = Buffer.from(input, "base64");
  console.log(base58Encode(raw));
  process.exit(0);
}

if (command === "encode-keypair-json") {
  const bytes = new Uint8Array(JSON.parse(fs.readFileSync(input, "utf8")));
  console.log(base58Encode(bytes));
  process.exit(0);
}

if (command === "decode-base58-to-hex") {
  const raw = base58Decode(input);
  console.log(`0x${raw.toString("hex")}`);
  process.exit(0);
}

usage();
