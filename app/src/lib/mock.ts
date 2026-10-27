// Backend simulato per provare l'interfaccia in un browser: `?demo=<scenario>`.
// Simula anche le transizioni (allineamento, controllo, espulsione).

import type { Backend } from "./api";
import type { DiskView, HistoryEvent, Progress, Question, Settings, State, Summary } from "./types";

const GB = 1_000_000_000;

const diskA: DiskView = { side: "A", label: "Disco A", root: "/Volumes/Disco A", total: 2000 * GB, free: 820 * GB };
const diskB: DiskView = { side: "B", label: "Disco B", root: "/Volumes/Disco B", total: 2000 * GB, free: 806 * GB };

const ago = (days: number) => new Date(Date.now() - days * 86_400_000).toISOString();

const questions: Question[] = [
  {
    key: "foto/2024/mare 2024/img_0042.jpg",
    path: "Foto/2024/Mare 2024/IMG_0042.jpg",
    kind: "deletion",
    deleted_on: "A",
    is_dir: false,
    a: null,
    b: { size: 4_200_000, modified: ago(200), preview: null },
  },
  {
    key: "foto/2023/prove",
    path: "Foto/2023/Prove",
    kind: "deletion",
    deleted_on: "A",
    is_dir: true,
    a: null,
    b: { size: 58_000_000, modified: ago(400), preview: null },
  },
  {
    key: "documenti/note.txt",
    path: "Documenti/Note.txt",
    kind: "conflict",
    deleted_on: null,
    is_dir: false,
    a: { size: 2_400, modified: ago(2), preview: null },
    b: { size: 3_100, modified: ago(1), preview: null },
  },
];

const summary = (withQuestions: boolean): Summary => ({
  lines: [
    { kind: "new", from: "A", count: 34, bytes: 1.2 * GB },
    { kind: "moved", from: "B", count: 2, bytes: 0 },
    { kind: "changed", from: "A", count: 3, bytes: 12_000_000 },
    { kind: "folders", from: "A", count: 1, bytes: 0 },
  ],
  files: [
    ...Array.from({ length: 34 }, (_, i) => ({
      kind: "new" as const,
      from: "A" as const,
      path: `Foto/2026/Pasqua/IMG_${String(1000 + i).padStart(4, "0")}.jpg`,
      size: 35_000_000,
    })),
    { kind: "moved", from: "B", path: "Foto/2025/Compleanno/IMG_0003.jpg", size: 0 },
    { kind: "moved", from: "B", path: "Foto/2025/Compleanno/IMG_0004.jpg", size: 0 },
    { kind: "changed", from: "A", path: "Documenti/Contratto affitto.pdf", size: 4_000_000 },
    { kind: "changed", from: "A", path: "Documenti/Lista.txt", size: 4_000 },
    { kind: "changed", from: "A", path: "Documenti/Spese.xlsx", size: 8_000_000 },
    { kind: "folders", from: "A", path: "Foto/2026/Vuota", size: 0 },
  ],
  questions: withQuestions ? questions : [],
  missing_space: { A: 0, B: 0 },
});

const progress = (done: number, total: number, bytesTotal: number, current: string): Progress => ({
  done,
  total,
  bytes_done: Math.round((bytesTotal * done) / total),
  bytes_total: bytesTotal,
  current,
  eta: total - done > 0 ? (total - done) * 7 : null,
});

const base: State = {
  pair: { labels: { A: "Disco A", B: "Disco B" } },
  disks: { A: diskA, B: diskB },
  foreign: [],
  busy: null,
  summary: null,
  outcome: null,
  last_sync: ago(12),
  last_check: ago(40),
  check_due: false,
  check_estimate: 3 * 3600,
};

const SCENARIOS: Record<string, Partial<State>> = {
  welcome: { pair: null, disks: { A: null, B: null } },
  none: { disks: { A: null, B: null } },
  one: { disks: { A: diskA, B: null } },
  foreign: { disks: { A: null, B: null }, foreign: ["Seagate 2TB"] },
  comparing: { busy: { kind: "comparing" } },
  summary: { summary: summary(false) },
  questions: { summary: summary(true) },
  "only-questions": { summary: { ...summary(true), lines: [], files: [] } },
  "no-space": { summary: { ...summary(false), missing_space: { A: 0, B: 4.3 * GB } } },
  syncing: {
    summary: summary(false),
    busy: { kind: "syncing", progress: progress(12, 40, 1.2 * GB, "Foto/2026/Pasqua/IMG_1012.jpg") },
  },
  done: {
    outcome: { kind: "synced", copied: 37, bytes: 1.21 * GB, deleted: 2, skipped: [] },
  },
  "done-skipped": {
    outcome: {
      kind: "synced",
      copied: 36,
      bytes: 1.2 * GB,
      deleted: 0,
      skipped: [{ path: "Foto/2026/Pasqua/IMG_1007.jpg", reason: "Il file è cambiato durante la copia" }],
    },
  },
  "in-sync": {},
  ejected: { disks: { A: null, B: null }, outcome: { kind: "ejected" } },
  "check-due": { check_due: true, last_check: ago(120) },
  checking: {
    busy: { kind: "checking", progress: progress(21_480, 116_428, 1300 * GB, "Foto/2019/Natale/IMG_4410.jpg") },
  },
  "check-ok": { outcome: { kind: "checked", ok: 58_214, repairs: [], lost: [], completed: true } },
  "check-repairs": {
    outcome: {
      kind: "checked",
      ok: 58_211,
      repairs: [
        { path: "Foto/2019/Natale/IMG_4410.jpg", bad: "B" },
        { path: "Foto/2020/Neve/IMG_0122.jpg", bad: "B" },
      ],
      lost: ["Foto/2012/Vecchie/scansione 03.tif"],
      completed: true,
    },
  },
  "check-paused": { outcome: { kind: "checked", ok: 0, repairs: [], lost: [], completed: false } },
  error: { outcome: { kind: "error", code: "read_only", disk: "B", message: "Permission denied (os error 13)" } },
  disconnected: { outcome: { kind: "error", code: "disconnected", disk: "B", message: "No such file or directory" } },
};

const history: HistoryEvent[] = [
  { at: ago(0.1), kind: "synced", path: null, disk: null, details: "copied=37 deleted=2" },
  { at: ago(0.1), kind: "deleted", path: "Foto/2024/Mare 2024/IMG_0042.jpg", disk: "Disco B", details: null },
  { at: ago(12), kind: "synced", path: null, disk: null, details: "copied=120 deleted=0" },
  { at: ago(40), kind: "full_check_completed", path: null, disk: null, details: "ok=58214 repairs=0 lost=0" },
  { at: ago(41), kind: "repaired", path: "Foto/2019/Natale/IMG_4410.jpg", disk: "Disco B", details: null },
  { at: ago(300), kind: "created", path: null, disk: null, details: null },
];

export function mockBackend(scenario: string): Backend {
  let state: State = structuredClone({ ...base, ...(SCENARIOS[scenario] ?? {}) });
  let settings: Settings = { autostart: true, check_months: 3, language: null };
  const listeners = new Set<(s: State) => void>();
  let timer: ReturnType<typeof setInterval> | undefined;

  const emit = (patch: Partial<State>) => {
    state = { ...state, ...patch };
    for (const l of listeners) l(structuredClone(state));
  };

  /** Simula un'operazione lunga, aggiornando l'avanzamento. */
  const run = (kind: "syncing" | "checking" | "repairing", total: number, done: () => Partial<State>) => {
    clearInterval(timer);
    let i = 0;
    timer = setInterval(() => {
      i = Math.min(total, i + Math.ceil(total / 25));
      if (i >= total) {
        clearInterval(timer);
        emit({ busy: null, ...done() });
        return;
      }
      emit({ busy: { kind, progress: progress(i, total, total * 30_000_000, `Foto/2026/Pasqua/IMG_${1000 + i}.jpg`) } });
    }, 160);
  };

  return {
    state: async () => structuredClone(state),
    async subscribe(listener) {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    async sync(answers) {
      const deleted = Object.values(answers).filter((a) => a === "delete").length;
      run("syncing", 40, () => ({
        summary: null,
        last_sync: new Date().toISOString(),
        outcome: { kind: "synced", copied: 37, bytes: 1.21 * GB, deleted, skipped: [] },
      }));
    },
    async eject() {
      emit({ busy: { kind: "ejecting" } });
      setTimeout(() => emit({ busy: null, disks: { A: null, B: null }, summary: null, outcome: { kind: "ejected" } }), 900);
    },
    async startCheck() {
      run("checking", 400, () => ({
        check_due: false,
        last_check: new Date().toISOString(),
        outcome: { kind: "checked", ok: 58_214, repairs: [], lost: [], completed: true },
      }));
    },
    async stopCheck() {
      clearInterval(timer);
      emit({ busy: null, outcome: { kind: "checked", ok: 0, repairs: [], lost: [], completed: false } });
    },
    async postponeCheck() {
      emit({ check_due: false });
    },
    async repair() {
      run("repairing", 2, () => ({ outcome: { kind: "repaired", done: 2, skipped: [] } }));
    },
    async refresh() {},
    async dismiss() {
      emit({ outcome: null });
    },
    settings: async () => ({ ...settings }),
    async setSettings(s) {
      settings = { ...s };
    },
    async renameDisk(side, label) {
      const labels = { ...state.pair!.labels, [side]: label };
      const disk = state.disks[side];
      emit({
        pair: { labels },
        disks: { ...state.disks, [side]: disk ? { ...disk, label } : null },
      });
    },
    history: async () => history,
    version: async () => "0.1.0 (demo)",
  };
}

export const SCENARIO_NAMES = Object.keys(SCENARIOS);
