// Controlla che tutte le lingue abbiano gli stessi testi.
import { readdirSync, readFileSync } from "node:fs";

const dir = new URL("../src/lib/i18n/", import.meta.url);
const isPlural = (v) => typeof v === "object" && Object.keys(v).every((k) => ["one", "other"].includes(k));

function keys(node, prefix = "") {
  return Object.entries(node).flatMap(([k, v]) =>
    typeof v === "object" && !isPlural(v) ? keys(v, `${prefix}${k}.`) : [`${prefix}${k}`],
  );
}

const files = readdirSync(dir).filter((f) => f.endsWith(".json"));
const sets = Object.fromEntries(
  files.map((f) => [f, new Set(keys(JSON.parse(readFileSync(new URL(f, dir), "utf8"))))]),
);
const all = new Set(Object.values(sets).flatMap((s) => [...s]));
let ok = true;
for (const [file, set] of Object.entries(sets)) {
  const missing = [...all].filter((k) => !set.has(k));
  if (missing.length) {
    ok = false;
    console.error(`${file}: mancano ${missing.join(", ")}`);
  }
}
if (!ok) process.exit(1);
console.log(`Traduzioni: ${files.join(", ")} complete (${all.size} testi).`);
