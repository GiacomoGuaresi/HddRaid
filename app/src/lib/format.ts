// Numeri, dimensioni, date e durate, nella lingua dell'interfaccia.

import { locale, t } from "./i18n/index.svelte";

const UNITS = ["byte", "KB", "MB", "GB", "TB"];

/** "1,2 GB". Multipli di 1000, come Esplora file e Finder sui dischi. */
export function bytes(n: number): string {
  let value = n;
  let unit = 0;
  while (value >= 1000 && unit < UNITS.length - 1) {
    value /= 1000;
    unit++;
  }
  const digits = unit === 0 || value >= 100 ? 0 : 1;
  const number = new Intl.NumberFormat(locale(), { maximumFractionDigits: digits }).format(value);
  return `${number} ${UNITS[unit]}`;
}

/** "circa 3 ore": durata approssimata per difetto alla sua unità. */
export function duration(seconds: number): string {
  if (seconds < 60) return t("time.lessThanMinute");
  const minutes = Math.round(seconds / 60);
  if (minutes < 60) return t("time.minutes", { count: minutes });
  const hours = Math.round(minutes / 60);
  if (hours < 48) return t("time.hours", { count: hours });
  return t("time.days", { count: Math.round(hours / 24) });
}

/** "12 marzo 2026". */
export function date(iso: string): string {
  return new Intl.DateTimeFormat(locale(), { dateStyle: "long" }).format(new Date(iso));
}

/** "12 marzo 2026, 18:40". */
export function dateTime(iso: string): string {
  return new Intl.DateTimeFormat(locale(), { dateStyle: "long", timeStyle: "short" }).format(
    new Date(iso),
  );
}

/** "oggi", "ieri", "3 giorni fa" o la data. */
export function relative(iso: string): string {
  const days = Math.floor((Date.now() - new Date(iso).getTime()) / 86_400_000);
  if (days > 6) return date(iso);
  return new Intl.RelativeTimeFormat(locale(), { numeric: "auto" }).format(-days, "day");
}

/** Percentuale intera di `done` su `total`. */
export function percent(done: number, total: number): number {
  return total > 0 ? Math.min(100, Math.floor((done / total) * 100)) : 0;
}
