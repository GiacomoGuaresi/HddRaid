# Formato dei dati sui dischi

Cosa scrive HddRaid su ogni disco. I file dell'utente restano file normali,
dove li ha messi lui, leggibili anche senza l'app.

```
<disco>/
├── .hddraid/
│   ├── disk.json          identità del disco
│   └── index.sqlite       indice: la base dell'ultimo allineamento
├── Scarica HddRaid.html   apre la pagina di download dell'app
└── …                      i file dell'utente
```

Su Windows la cartella `.hddraid/` viene anche marcata come nascosta.

## `disk.json`

```json
{
  "disk_id": "3f2a…",
  "archive_id": "9c41…",
  "side": "A",
  "label": "Disco A",
  "created": "2026-10-27T19:10:00+00:00",
  "format": 1
}
```

| Campo | Significato |
|---|---|
| `disk_id` | identificativo unico del disco (UUID) |
| `archive_id` | identificativo della coppia di dischi, uguale su entrambi |
| `side` | `A` o `B`: il lato del disco nella coppia, uguale su qualsiasi computer |
| `label` | nome scelto dall'utente ("Disco A" e "Disco B" di default), usato in tutti i messaggi; modificabile dalle impostazioni |
| `created` | data di preparazione (UTC) |
| `format` | versione del formato: un'app più vecchia rifiuta un disco più nuovo |

Anche l'**etichetta del volume** (il nome che si vede in Esplora file / Finder)
viene impostata dal nome, ridotto se serve agli 11 caratteri ammessi da exFAT
(es. "Disco rosso" → `DISCO ROSSO`).

L'app riconosce i suoi dischi da questo file, non dalla lettera di unità o dal
punto di montaggio, che cambiano da computer a computer. Un disco con un
`archive_id` sconosciuto è di un'altra coppia: l'app lo mostra ma non lo tocca.

Il file si scrive in modo atomico (file temporaneo → `fsync` → rinomina), così
un'interruzione lascia il file vecchio o quello nuovo, mai uno a metà.

## `index.sqlite`

Database SQLite, **uguale su entrambi i dischi** dopo ogni allineamento.

| Tabella | Contenuto |
|---|---|
| `meta` | `archive_id`, `hash_algorithm` (`sha256`), `revision`, `created`, `last_sync`, `last_full_check`, `full_check_started` (controllo completo in corso, da riprendere) |
| `disks` | un record per disco: id, nome, data di aggiunta, data di rimozione (dischi sostituiti) |
| `files` | la base: percorso (relativo alla radice, con `/`, in NFC), tipo (`file` o `cartella vuota`), dimensione, data di modifica (ns, UTC), hash, data di aggiunta |
| `verified` | per ogni file e disco: data dell'ultima verifica del contenuto (controllo completo) |
| `events` | storico: data, tipo (copiato, spostato, cancellato, riparato, conflitto…), percorso, disco, dettagli JSON |
| `journal` | operazioni in corso: tipo e parametri JSON; una riga rimasta indica un'operazione interrotta |

- I dischi sono **righe, non colonne**: un terzo disco non cambia lo schema.
- **Versione dello schema** in `PRAGMA user_version`; all'apertura l'app applica
  le migrazioni mancanti e rifiuta un indice scritto da un'app più nuova.
- Ogni scrittura avviene in una **transazione** che incrementa `revision`.
  Confrontando le revisioni si capisce quale disco ha l'indice più recente dopo
  un'interruzione.
- Niente WAL (`journal_mode = DELETE`, `synchronous = FULL`): il WAL usa memoria
  condivisa, poco affidabile su dischi rimovibili exFAT.
- L'indice si copia sull'altro disco con l'API di backup di SQLite su un file
  temporaneo, seguita da una rinomina atomica.
- L'app lavora su una **copia locale** dell'indice e la riscrive sui dischi a
  intervalli e a fine operazione: così un disco scollegato non lascia un
  database aperto a metà.

## Copia sicura

Durante una copia, il file viene scritto come `.<nome>.hddraid-partial` nella
cartella di destinazione. Prende il nome definitivo solo dopo:

1. scrittura forzata sul disco (`fsync`);
2. rilettura dal disco e confronto dell'hash con quello della sorgente;
3. impostazione della data di modifica originale.

Un file `.hddraid-partial` rimasto sul disco indica una copia interrotta: viene
ignorato e sostituito dalla copia successiva. Quando la copia sostituisce un
file esistente, la rinomina finale prende il posto del vecchio file in un solo
passo: fino a quel momento il vecchio file resta intatto.

## Operazioni interrotte

Se il computer si spegne o un disco si stacca a metà:

- **Indice**: vince quello con la `revision` più alta; l'altro viene
  riallineato all'apertura.
- **Azioni** (copia, sposta, cancella, ripara): prima di toccare i file,
  l'azione viene scritta nella tabella `journal`. Quando è finita, la riga
  del journal viene tolta e la base aggiornata **nella stessa transazione**.
  Se resta una riga, all'apertura l'app segnala l'operazione interrotta.
- **Ripresa**: la base cambia solo per le azioni finite, quindi al giro
  successivo il confronto ritrova le stesse differenze, meno quelle già
  sistemate. Un'azione a metà si risolve da sola: un file già copiato ma non
  registrato risulta "nuovo su entrambi con lo stesso checksum", un file già
  cancellato su entrambi esce dalla base, e così via.
- **Copie e rinomine a metà**: restano come `.hddraid-partial` o
  `.hddraid-rename`, ignorati e sostituiti.
- **Domande**: le risposte non si salvano. Una domanda senza risposta
  ("Decido dopo") viene ricalcolata e riproposta al giro successivo.

## `Scarica HddRaid.html`

Una pagina HTML statica, scritta dall'app su entrambi i dischi, che si apre con
un doppio clic su qualsiasi sistema. Dice in poche righe cos'è il disco e
rimanda alla pagina di download (con un redirect automatico). Funziona anche
come promemoria: "questo disco è gestito da HddRaid".
