#!/usr/bin/env node

const XOR_KEY = process.env.XOR_KEY || "s3cr3t_k3y_pr0xy";

const DEFAULT_URL = "https://myanime.sharepoint.com/sites/chartlousty/_layouts/15/download.aspx?share=IQB3jpvjg7K1SJlW200EHKJGAcR4LBn_Ia6xgze_dJjC_fs";

const url = process.argv[2] || DEFAULT_URL;

function xorTransform(input) {
  const data = Buffer.from(input, "utf-8");
  const key = Buffer.from(XOR_KEY, "utf-8");
  for (let i = 0; i < data.length; i++) {
    data[i] ^= key[i % key.length];
  }
  return data;
}


function encryptUrl(url) {
  const xored = xorTransform(url);
  return xored
    .toString("base64")
    .replace(/\+/g, "-")
    .replace(/\//g, "_")
    .replace(/=+$/, "");
}

function decryptUrl(encrypted) {
  let b64 = encrypted.replace(/-/g, "+").replace(/_/g, "/");
  while (b64.length % 4 !== 0) b64 += "=";
  const data = Buffer.from(b64, "base64");
  const key = Buffer.from(XOR_KEY, "utf-8");
  for (let i = 0; i < data.length; i++) {
    data[i] ^= key[i % key.length];
  }
  return data.toString("utf-8");
}

const encrypted = encryptUrl(url);
const PROXY_HOST = process.env.PROXY_HOST || "http://localhost:8080";

console.log("Original URL: ", url);
console.log("Encrypted:    ", encrypted);
console.log(`Proxy URL:     ${PROXY_HOST}/?u=${encrypted}`);
const decrypted = decryptUrl(encrypted);
console.log("Roundtrip OK: ", decrypted === url ? "YES" : `NO (got: ${decrypted})`);
