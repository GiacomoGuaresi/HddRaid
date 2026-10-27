# Sviluppo

## Requisiti

- [Rust](https://rustup.rs/) stabile
- Node.js 24 e [pnpm](https://pnpm.io/)
- Su Linux, le dipendenze di Tauri: `libwebkit2gtk-4.1-dev`,
  `libappindicator3-dev`, `librsvg2-dev`

## Struttura

```
crates/hddraid-core/      motore, senza UI
crates/hddraid-devtools/  dischi simulati
app/                      app Tauri 2 + SvelteKit
app/src-tauri/            parte Rust dell'app
```

## Comandi

```sh
cargo test --workspace                       # test
cargo clippy --workspace --all-targets       # lint
cargo fmt --all                              # formattazione
cd app && pnpm install && pnpm tauri dev     # app in sviluppo
```

## Dischi simulati

Due cartelle in `dev-disks/` (ignorata da git) fanno da disco A e disco B, con
lo stesso contenuto finto: foto per anno ed evento, documenti, nomi con accenti,
un file vuoto e una cartella vuota.

```sh
cargo run -p hddraid-devtools --bin dev-disks -- setup
cargo run -p hddraid-devtools --bin dev-disks -- break <scenario>
```

| Scenario | Cosa fa |
|---|---|
| `new` | file nuovi sul disco A |
| `modify` | file modificato sul disco A |
| `delete` | file cancellato dal disco B |
| `move` | cartella rinominata sul disco A |
| `conflict` | stesso file modificato in modo diverso su A e B |
| `same-content` | stesso file nuovo su A e B, con date diverse (non è un conflitto) |
| `corrupt` | un byte cambiato sul disco B, stessa dimensione e data (bit rot) |
| `empty-dir` | cartella vuota sul disco A |
| `case` | file rinominato cambiando solo le maiuscole sul disco A |
| `system` | file di sistema da ignorare (`.DS_Store`, `Thumbs.db`, `$RECYCLE.BIN`…) |
| `all` | tutti insieme |

Gli stessi dischi si usano nei test, tramite la libreria `hddraid-devtools`.
