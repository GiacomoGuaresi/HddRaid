// Accesso al backend. Dentro Tauri parla con Rust; in un browser normale usa
// un backend simulato (`mock.ts`), scelto con `?demo=<scenario>`, per vedere
// e provare le schermate senza dischi.

import type { Answer, HistoryEvent, Settings, Side, State } from "./types";

export interface Backend {
  state(): Promise<State>;
  /** Chiama `listener` a ogni cambio di stato. */
  subscribe(listener: (state: State) => void): Promise<() => void>;
  sync(answers: Record<string, Answer>): Promise<void>;
  eject(): Promise<void>;
  startCheck(): Promise<void>;
  stopCheck(): Promise<void>;
  postponeCheck(): Promise<void>;
  repair(): Promise<void>;
  /** Chiude l'esito dell'ultima operazione. */
  dismiss(): Promise<void>;
  settings(): Promise<Settings>;
  setSettings(settings: Settings): Promise<void>;
  renameDisk(side: Side, label: string): Promise<void>;
  history(): Promise<HistoryEvent[]>;
  version(): Promise<string>;
}

function tauriBackend(): Backend {
  const call = async <T>(cmd: string, args?: Record<string, unknown>) => {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<T>(cmd, args);
  };
  return {
    state: () => call("get_state"),
    async subscribe(listener) {
      const { listen } = await import("@tauri-apps/api/event");
      return listen<State>("state", (e) => listener(e.payload));
    },
    sync: (answers) => call("sync", { answers }),
    eject: () => call("eject"),
    startCheck: () => call("start_check"),
    stopCheck: () => call("stop_check"),
    postponeCheck: () => call("postpone_check"),
    repair: () => call("repair"),
    dismiss: () => call("dismiss"),
    settings: () => call("get_settings"),
    setSettings: (settings) => call("set_settings", { settings }),
    renameDisk: (side, label) => call("rename_disk", { side, label }),
    history: () => call("history"),
    version: () => call("core_version"),
  };
}

export const inTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

let instance: Promise<Backend> | null = null;

export function backend(): Promise<Backend> {
  instance ??= inTauri
    ? Promise.resolve(tauriBackend())
    : import("./mock").then((m) =>
        m.mockBackend(new URLSearchParams(location.search).get("demo") ?? "summary"),
      );
  return instance;
}
