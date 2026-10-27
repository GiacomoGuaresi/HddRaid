// Contratto con il backend Rust (`src-tauri/src/state.rs`): i nomi dei campi
// sono quelli serializzati da serde.

export type Side = "A" | "B";

/** Un disco della coppia, collegato. */
export interface DiskView {
  side: Side;
  /** Il nome del volume, come in Esplora file / Finder. */
  label: string;
  root: string;
  /** "exfat", "ntfs", "apfs"…; `null` se non si sa. */
  filesystem: string | null;
  total: number;
  free: number;
}

/** Un volume collegato che non fa parte della coppia. */
export interface VolumeView {
  root: string;
  name: string;
  filesystem: string | null;
  total: number;
  free: number;
  /** Preparato da HddRaid per un'altra coppia. */
  other_pair: boolean;
}

/** Una riga del riepilogo: "34 file nuovi sul disco A → li copio sul B". */
export interface SummaryLine {
  kind: "new" | "changed" | "moved" | "folders";
  /** Il disco dove è avvenuto il cambiamento. */
  from: Side;
  count: number;
  bytes: number;
}

/** Un file dell'elenco dettagliato. */
export interface FileLine {
  kind: SummaryLine["kind"];
  from: Side;
  path: string;
  size: number;
}

/** Una versione di un file su un disco. */
export interface VersionView {
  size: number;
  /** RFC 3339. */
  modified: string;
  /** URL dell'anteprima per le immagini, se disponibile. */
  preview: string | null;
}

export interface Question {
  key: string;
  path: string;
  kind: "deletion" | "conflict";
  /** Per le cancellazioni: il disco da cui il file è stato cancellato. */
  deleted_on: Side | null;
  is_dir: boolean;
  a: VersionView | null;
  b: VersionView | null;
}

export type Answer = "delete" | "restore" | "keep_a" | "keep_b" | "both" | "later";

export interface Summary {
  lines: SummaryLine[];
  files: FileLine[];
  questions: Question[];
  /** Spazio che manca per disco, in byte (0 se basta). */
  missing_space: Record<Side, number>;
}

export interface Progress {
  done: number;
  total: number;
  bytes_done: number;
  bytes_total: number;
  current: string;
  /** Secondi stimati alla fine, se si possono stimare. */
  eta: number | null;
}

export type Busy =
  | { kind: "comparing" }
  | { kind: "syncing"; progress: Progress }
  | { kind: "checking"; progress: Progress }
  | { kind: "repairing"; progress: Progress }
  | { kind: "ejecting" }
  | { kind: "formatting"; label: string };

export interface Skipped {
  path: string;
  reason: string;
}

export interface Repair {
  path: string;
  bad: Side;
}

export type Outcome =
  | { kind: "synced"; copied: number; bytes: number; deleted: number; skipped: Skipped[] }
  | { kind: "checked"; ok: number; repairs: Repair[]; lost: string[]; completed: boolean }
  | { kind: "repaired"; done: number; skipped: Skipped[] }
  | { kind: "ejected" }
  | { kind: "error"; code: ErrorCode; disk: Side | null; message: string };

export type ErrorCode = "disconnected" | "no_space" | "read_only" | "eject" | "other";

export interface State {
  /** `null` finché non c'è una coppia di dischi: primo avvio. */
  pair: { labels: Record<Side, string> } | null;
  disks: Record<Side, DiskView | null>;
  /** Volumi collegati che non fanno parte della coppia. */
  others: VolumeView[];
  busy: Busy | null;
  /** Cosa c'è da fare, con entrambi i dischi collegati. */
  summary: Summary | null;
  /** I dischi sono stati confrontati da quando sono collegati: senza
   *  riepilogo, vuol dire che sono uguali. */
  compared: boolean;
  /** Esito dell'ultima operazione, finché non si chiude. */
  outcome: Outcome | null;
  last_sync: string | null;
  last_check: string | null;
  check_due: boolean;
  /** Durata stimata del controllo completo, in secondi. */
  check_estimate: number | null;
}

export interface Settings {
  autostart: boolean;
  /** Mesi tra un controllo completo e l'altro. */
  check_months: 1 | 3 | 6;
  /** `null`: lingua del sistema. */
  language: "it" | "en" | null;
}

export interface HistoryEvent {
  at: string;
  kind: string;
  path: string | null;
  disk: string | null;
  details: string | null;
}
