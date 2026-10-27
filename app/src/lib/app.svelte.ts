// Stato dell'interfaccia: lo stato del backend più la vista scelta
// dall'utente. `screen()` decide cosa mostrare.

import { backend, type Backend } from "./api";
import type { Answer, State } from "./types";

export type View = "home" | "questions" | "settings" | "history";

export type ScreenName =
  | "loading"
  | "welcome"
  | "noDisks"
  | "oneDisk"
  | "comparing"
  | "summary"
  | "questions"
  | "syncing"
  | "done"
  | "ejected"
  | "inSync"
  | "checkDue"
  | "checking"
  | "checkResult"
  | "error"
  | "settings"
  | "history";

export const app = $state<{
  state: State | null;
  view: View;
  /** Risposte date finora, per chiave del file. */
  answers: Record<string, Answer>;
}>({ state: null, view: "home", answers: {} });

let instance: Backend | null = null;

/** Il backend, pronto dopo `start()`. */
export function api(): Backend {
  if (!instance) throw new Error("backend non pronto");
  return instance;
}

export async function start() {
  const api = await backend();
  instance = api;
  await api.subscribe((s) => {
    // Le domande cambiano solo con un nuovo confronto: le risposte date a
    // file che non sono più in discussione si dimenticano.
    const keys = new Set(s.summary?.questions.map((q) => q.key) ?? []);
    for (const key of Object.keys(app.answers)) {
      if (!keys.has(key)) delete app.answers[key];
    }
    app.state = s;
  });
  app.state = await api.state();
}

export function screen(): ScreenName {
  const s = app.state;
  if (!s) return "loading";
  if (app.view === "settings" || app.view === "history") return app.view;
  if (!s.pair) return "welcome";

  switch (s.busy?.kind) {
    case "comparing":
      return "comparing";
    case "syncing":
      return "syncing";
    case "checking":
    case "repairing":
      return "checking";
    case "ejecting":
      return "done";
  }

  switch (s.outcome?.kind) {
    case "synced":
      return "done";
    case "checked":
    case "repaired":
      return "checkResult";
    case "ejected":
      return "ejected";
    case "error":
      return "error";
  }

  if (!s.disks.A && !s.disks.B) return "noDisks";
  if (!s.disks.A || !s.disks.B) return "oneDisk";
  const summary = s.summary;
  if (summary && (summary.lines.length > 0 || summary.questions.length > 0)) {
    return app.view === "questions" ? "questions" : "summary";
  }
  if (s.check_due) return "checkDue";
  return "inSync";
}

/** Nome del disco `side`, collegato o no. */
export function label(side: "A" | "B"): string {
  return app.state?.pair?.labels[side] ?? `Disco ${side}`;
}

export function other(side: "A" | "B"): "A" | "B" {
  return side === "A" ? "B" : "A";
}
