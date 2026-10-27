# Roadmap (bozza)

Sviluppo a **piccoli incrementi**: ogni step è piccolo, testabile e lascia il
progetto funzionante.

## Regole di lavoro

- **Uno step = un commit.** Se uno step è troppo grande, si divide in `3a`, `3b`, …
- **Definition of done**: test verdi (`cargo test`, test del frontend), lint
  pulito (`clippy`, `rustfmt`), documentazione in `doc/` aggiornata, commit
  [Conventional Commits](https://www.conventionalcommits.org/).
- Il motore vive in un crate **senza UI**; l'app Tauri si limita a chiamarlo.
- **Sicurezza dei dati prima di tutto**: ogni operazione che scrive sui dischi
  ha test sugli scenari di guasto.

Legenda: ⬜ da fare · 🔄 in corso · ✅ fatto

---

## M0 — Fondamenta

| # | Step | Commit | Stato |
|---|---|---|---|
| 0.1 | `git init`, `README.md`, `LICENSE`, `.gitignore`, `doc/` | `first commit` | ⬜✅ |
| 0.2 | Workspace Cargo: crate `hddraid-core`, app Tauri 2 + Svelte vuota | `chore: project skeleton` | ✅ |
| 0.3 | CI: lint e test su Windows, macOS e Linux | `ci: lint and test` | ✅ |
| 0.4 | Generatore di dischi simulati per sviluppo e test | `chore(dev): simulated disks` | ✅ |

## M1 — Motore di base (senza UI) → `v0.1.0`

| # | Step | Commit | Stato |
|---|---|---|---|
| 1.1 | **Disco**: `.hddraid/disk.json`, scrittura atomica, riconoscimento | `feat(core): disk identity` | ✅ |
| 1.2 | **Indice** SQLite versionato, `revision`, copia tra dischi | `feat(core): versioned index` | ✅ |
| 1.3 | **Scanner**: esclusioni, NFC, maiuscole, tolleranza sulle date | `feat(core): scanner` | ✅ |
| 1.4 | **Hash** SHA-256 in streaming | `feat(core): hashing` | ✅ |
| 1.5 | **Copia sicura**: `.hddraid-partial` → `fsync` → verifica → rinomina | `feat(core): safe copy` | ⬜ |
| 1.6 | **CLI di debug**: `init`, `scan`, `status` | `feat(cli): debug commands` | ⬜ |

## M2 — Sincronizzazione (senza UI) → `v0.2.0`

| # | Step | Commit | Stato |
|---|---|---|---|
| 2.1 | **Confronto a tre vie** e piano delle azioni ([tabella dei casi](sincronizzazione.md#tabella-dei-casi)) | `feat(sync): three-way plan` | ⬜ |
| 2.2 | **Esecuzione** del piano, nuova base su entrambi i dischi | `feat(sync): apply plan` | ⬜ |
| 2.3 | **Cancellazioni e conflitti** in sospeso (`pending`) e risoluzioni | `feat(sync): deletions and conflicts` | ⬜ |
| 2.4 | **Spostamenti e rinomine** tramite hash, cambi di sole maiuscole | `feat(sync): moves` | ⬜ |
| 2.5 | **Journal** e ripresa delle operazioni interrotte | `feat(core): operation journal` | ⬜ |
| 2.6 | **Adozione**: primo allineamento senza base | `feat(sync): adopt existing disks` | ⬜ |
| 2.7 | Test end-to-end degli scenari di guasto, anche su volumi exFAT veri | `test: failure scenarios` | ⬜ |

## M3 — Controllo completo (senza UI) → `v0.3.0`

| # | Step | Commit | Stato |
|---|---|---|---|
| 3.1 | Verifica di tutti i file, lettura dei due dischi in parallelo, stima del tempo | `feat(check): full check` | ⬜ |
| 3.2 | Ripresa dopo un'interruzione (`verified`) | `feat(check): resume` | ⬜ |
| 3.3 | Riparazione dalla copia buona | `feat(check): repair` | ⬜ |

## M4 — App → `v0.4.0`

Vedi [flussi](flussi.md).

| # | Step | Commit | Stato |
|---|---|---|---|
| 4.1 | Design: tono caldo, Inter + Lucide come PrintPicker, tema chiaro/scuro, componenti, italiano e inglese | `feat(ui): design system` | ⬜ |
| 4.2 | Icona nella barra con gli stati, avvio con il computer | `feat(app): tray and autostart` | ⬜ |
| 4.3 | **Rilevamento dei dischi** e notifiche | `feat(app): disk detection` | ⬜ |
| 4.4 | **Allinea**: riepilogo, avanzamento, fine, espulsione | `feat(ui): sync flow` | ⬜ |
| 4.5 | Cancellazioni e conflitti | `feat(ui): questions flow` | ⬜ |
| 4.6 | Controllo completo con promemoria | `feat(ui): full check flow` | ⬜ |
| 4.7 | Impostazioni e storico | `feat(ui): settings and history` | ⬜ |

## M5 — Dischi → `v0.5.0`

| # | Step | Commit | Stato |
|---|---|---|---|
| 5.1 | Primo avvio e **preparazione dei dischi**: adozione e controllo del file system | `feat(ui): prepare disks` | ⬜ |
| 5.2 | **Formattazione** exFAT guidata sui tre sistemi | `feat(app): exfat formatting` | ⬜ |
| 5.3 | **Sostituzione** di un disco rotto | `feat(ui): replace disk` | ⬜ |
| 5.4 | `Scarica HddRaid.html` sui dischi | `feat(core): download link file` | ⬜ |

## M6 — Distribuzione → `v1.0.0`

| # | Step | Commit | Stato |
|---|---|---|---|
| 6.1 | Installer per i tre sistemi in CI, firma ad hoc su macOS | `ci: release builds` | ⬜ |
| 6.2 | Aggiornamenti automatici | `feat(app): auto update` | ⬜ |
| 6.3 | Pagina di download con istruzioni illustrate | `docs: download page` | ⬜ |
| 6.4 | Prova sul campo: migrazione dei miei dischi attuali | — | ⬜ |

## Dopo

- Eventi nativi per il rilevamento dei dischi (al posto del controllo periodico).
- Firma del codice su Windows e macOS.
- Terzo disco.
