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
crates/hddraid-cli/       CLI di debug del motore (`hddraid`)
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

## CLI di debug

Prova il motore sui dischi simulati, senza interfaccia. La copia locale
dell'indice va in `$HDDRAID_WORK` (di default nella cartella temporanea).

```sh
export HDDRAID_WORK=$PWD/dev-disks/work
cargo run -p hddraid-cli -- init dev-disks/disk-a dev-disks/disk-b
cargo run -p hddraid-cli -- status dev-disks/disk-a dev-disks/disk-b
cargo run -p hddraid-cli -- scan dev-disks/disk-a
cargo run -p hddraid-cli -- sync dev-disks/disk-a dev-disks/disk-b
cargo run -p hddraid-cli -- check dev-disks/disk-a dev-disks/disk-b --repair
```

`sync` mostra il piano, chiede cosa fare per cancellazioni e conflitti
(oppure `--answer delete|restore|keep-a|keep-b|both|skip` per tutte) e
conferma prima di eseguire (`--yes` per saltare la conferma). `check` fa il
controllo completo; con `--repair` ripara i file rovinati dalla copia buona.

Un giro completo:

```sh
cargo run -p hddraid-devtools --bin dev-disks -- setup
cargo run -p hddraid-cli -- init dev-disks/disk-a dev-disks/disk-b
cargo run -p hddraid-cli -- sync dev-disks/disk-a dev-disks/disk-b --yes
cargo run -p hddraid-devtools --bin dev-disks -- break all
cargo run -p hddraid-cli -- sync dev-disks/disk-a dev-disks/disk-b
```

## Prova su volumi exFAT veri

Su macOS, `scripts/exfat-e2e.sh` crea due immagini disco exFAT in
`dev-disks/exfat/`, le monta (senza permessi di amministratore), ci fa un
giro completo (preparazione, allineamento, tutti gli scenari, controllo
completo con riparazione) e verifica che alla fine i dischi siano uguali.
Gira anche in CI sul runner macOS.

```sh
scripts/exfat-e2e.sh
```
