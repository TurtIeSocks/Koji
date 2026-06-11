// Minimal static server that sets the COOP/COEP headers threaded wasm needs
// (SharedArrayBuffer requires cross-origin isolation). Serves this crate dir, so
// /www/index.html can import ../pkg/koji_wasm.js.
//
//   node crates/koji-wasm/serve.mjs        # http://localhost:8080/www/
//
// (http-server's --header flag is unreliable; this guarantees the headers.)
import { createServer } from "node:http";
import { readFile } from "node:fs/promises";
import { extname, join, normalize, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = dirname(fileURLToPath(import.meta.url));
const PORT = Number(process.env.PORT) || 8080;
const TYPES = {
  ".html": "text/html",
  ".js": "application/javascript",
  ".mjs": "application/javascript",
  ".wasm": "application/wasm",
  ".json": "application/json",
  ".ts": "text/plain",
};

createServer(async (req, res) => {
  res.setHeader("Cross-Origin-Opener-Policy", "same-origin");
  res.setHeader("Cross-Origin-Embedder-Policy", "require-corp");
  try {
    let p = decodeURIComponent(new URL(req.url, "http://x").pathname);
    if (p.endsWith("/")) p += "index.html";
    const file = normalize(join(ROOT, p));
    if (!file.startsWith(ROOT)) throw new Error("forbidden");
    const data = await readFile(file);
    res.setHeader("Content-Type", TYPES[extname(file)] || "application/octet-stream");
    res.end(data);
  } catch (e) {
    res.statusCode = 404;
    res.end("404 " + e.message);
  }
}).listen(PORT, () => console.log(`koji-wasm demo: http://localhost:${PORT}/www/`));
