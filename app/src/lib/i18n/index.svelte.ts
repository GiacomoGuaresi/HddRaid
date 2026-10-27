// Traduzioni: un file JSON per lingua. Un testo con i plurali è un oggetto
// `{ "one": ..., "other": ... }` e si sceglie con la variabile `count`.
// Le variabili si scrivono `{nome}`.

import it from "./it.json";
import en from "./en.json";

export type Language = "it" | "en";

type Messages = { [key: string]: string | Messages };

const catalogs: Record<Language, Messages> = { it, en };

function systemLanguage(): Language {
  const lang = typeof navigator === "undefined" ? "it" : navigator.language;
  return lang.toLowerCase().startsWith("it") ? "it" : "en";
}

const current = $state({ lang: systemLanguage() });

/** Lingua attuale. */
export function lang(): Language {
  return current.lang;
}

/** Imposta la lingua; `null` torna a quella del sistema. */
export function setLanguage(language: Language | null) {
  current.lang = language ?? systemLanguage();
  document.documentElement.lang = current.lang;
}

/** Locale per `Intl`. */
export function locale(): string {
  return current.lang === "it" ? "it-IT" : "en-GB";
}

function lookup(messages: Messages, key: string): string | Messages | undefined {
  let node: string | Messages | undefined = messages;
  for (const part of key.split(".")) {
    if (node === undefined || typeof node === "string") return undefined;
    node = node[part];
  }
  return node;
}

/** Testo tradotto. Se manca, prova l'italiano e poi restituisce la chiave. */
export function t(key: string, vars: Record<string, string | number> = {}): string {
  let node = lookup(catalogs[current.lang], key) ?? lookup(catalogs.it, key);
  if (node !== undefined && typeof node !== "string") {
    const count = typeof vars.count === "number" ? vars.count : 0;
    const form = new Intl.PluralRules(locale()).select(count);
    node = node[form] ?? node.other;
  }
  if (typeof node !== "string") return key;
  return node.replace(/\{(\w+)\}/g, (match, name: string) => {
    const value = vars[name];
    if (value === undefined) return match;
    return typeof value === "number" ? new Intl.NumberFormat(locale()).format(value) : value;
  });
}
