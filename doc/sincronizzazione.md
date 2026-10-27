# Sincronizzazione

Come l'app capisce cosa è cambiato sui dischi e cosa fare.

## Perché serve un indice

Se un file c'è sul disco A e manca sul disco B, guardando solo i dischi non si
può sapere se è stato **aggiunto su A** o **cancellato da B**: nel primo caso va
copiato, nel secondo va chiesto se cancellarlo anche da A.

Per questo l'app salva sui dischi un **indice**: l'elenco dei file com'erano
all'ultimo allineamento riuscito (la *base*). Ogni disco si confronta con la
base, non con l'altro disco: è un confronto a tre vie, come fanno git o
Syncthing.

## Le fasi di un allineamento

1. **Lettura dell'indice** di entrambi i dischi (vince quello con la
   `revision` più alta, vedi [formato-dati.md](formato-dati.md#indexsqlite)).
2. **Scansione** di entrambi i dischi: per ogni file percorso, dimensione e data
   di modifica. Nessuna lettura del contenuto: su un disco già allineato
   richiede pochi secondi.
3. **Confronto con la base**: ogni file, su ogni disco, è *invariato*, *nuovo*,
   *modificato* o *cancellato* (vedi sotto).
4. **Checksum dove serve per decidere**: file modificati, file nuovi nello
   stesso percorso su entrambi i dischi (possibile conflitto) e file nuovi con
   la stessa dimensione di un file cancellato (possibile spostamento). Gli
   altri file nuovi si leggono una volta sola, durante la copia, che ne calcola
   il checksum: così 50 GB di foto nuove non vengono letti due volte.
5. **Piano**: l'elenco delle azioni, ricavato dalla [tabella dei
   casi](#tabella-dei-casi). Le azioni ovvie vanno nel riepilogo, cancellazioni
   e conflitti diventano domande.
6. **Riepilogo** all'utente e pulsante **Allinea**.
7. **Esecuzione**, con controllo dello spazio libero prima di iniziare e il
   [journal](formato-dati.md#operazioni-interrotte) per le interruzioni.
8. **Nuova base**: l'indice aggiornato viene scritto su entrambi i dischi.

Se tra il riepilogo e l'esecuzione un file cambia ancora (l'utente sta copiando
qualcosa), oppure è aperto e bloccato da un altro programma, l'azione su quel
file viene saltata e il file si riprende al giro successivo. A fine lavoro
l'app dice quanti file ha saltato.

## Stato di un file su un disco

Confronto tra il file sul disco e la riga della base:

| Stato | Condizione |
|---|---|
| invariato | c'è nella base, stessa dimensione e stessa data di modifica |
| modificato | c'è nella base, dimensione o data di modifica diverse |
| nuovo | non c'è nella base |
| cancellato | c'è nella base ma non sul disco |

Le date si confrontano in UTC con una tolleranza di 2 secondi: exFAT salva le
date con risoluzione di 10 ms e alcuni sistemi le arrotondano.

Un file *modificato* ha sempre il suo hash ricalcolato (fase 4). Se l'hash è
uguale a quello della base, il contenuto non è cambiato (es. è cambiata solo la
data): il file torna *invariato* e si aggiorna solo la data nella base.

## Tabella dei casi

Per ogni percorso, lo stato sul disco A e sul disco B. Il caso con A e B
scambiati è simmetrico.

| A | B | Azione | Chiede? |
|---|---|---|---|
| invariato | invariato | niente | — |
| nuovo | assente | copia A → B | riepilogo |
| nuovo | nuovo, stesso checksum | niente, entra nella base | — |
| nuovo | nuovo, checksum diverso | **conflitto** | sì |
| modificato | invariato | copia A → B, sostituendo la versione di B | riepilogo |
| modificato | modificato, stesso checksum | niente, aggiorna la base | — |
| modificato | modificato, checksum diverso | **conflitto** | sì |
| cancellato | invariato | **cancellazione**: cancella anche da B, oppure rimetti su A | sì |
| cancellato | cancellato | niente, esce dalla base | — |
| cancellato | modificato | **conflitto**: tieni la versione di B (e rimettila su A) o cancella | sì |

Le **cancellazioni** si chiedono sempre, file per file, con la possibilità di
applicare la stessa risposta a tutti. Cartelle intere cancellate si mostrano
come una voce sola ("Cartella *Mare 2024*, 212 file").

### Conflitti

**Un conflitto si dichiara solo dopo aver confrontato il checksum** (SHA-256)
delle due versioni. Dimensione e data di modifica non bastano: lo stesso file
copiato due volte, da computer diversi o con programmi diversi, può avere date
diverse ma contenuto identico.

- **Checksum uguale** → i file sono identici: nessun conflitto, nessuna
  domanda. Le date sui due dischi vengono allineate e il file entra nella base.
- **Checksum diverso** → conflitto vero, e l'app chiede.
- **Dimensione diversa** → i file sono sicuramente diversi; il checksum serve
  comunque per la base e per verificare la copia che risolverà il conflitto.

Il checksum viene sempre letto dal disco al momento del confronto, mai preso
dall'indice, così un file rovinato non viene scambiato per uno identico.

Per ogni conflitto vero l'app mostra le due versioni (data, dimensione, anteprima
per le immagini) e propone:

- **Tieni quella del disco A** (l'altra viene sostituita);
- **Tieni quella del disco B** (l'altra viene sostituita);
- **Tienile entrambe**: la versione di B viene rinominata
  `nome (disco B).ext` su entrambi i dischi.

Finché un conflitto non è risolto, quel file resta fuori dalla base e viene
riproposto al giro successivo; il resto dell'allineamento procede.

## Spostamenti e rinomine

Uno spostamento appare come un file *cancellato* in un percorso e un file
*nuovo* in un altro, sullo stesso disco. Se il file nuovo ha lo **stesso
hash** di quello cancellato (che la base conosce) e sull'altro disco il file
vecchio è *invariato*, l'azione diventa **sposta** sull'altro disco, invece di
"copia + chiedi se cancellare". Lo stesso vale per cartelle intere.

Se l'hash corrisponde a più file (doppioni), lo spostamento si applica solo
quando l'abbinamento è univoco; altrimenti si ricade nei casi normali.

## Nomi dei file

- **Maiuscole**: exFAT non distingue `Foto.jpg` da `foto.jpg`. I percorsi si
  confrontano senza distinguere maiuscole e minuscole. Un cambio solo di
  maiuscole su un disco viene propagato in due passi (`foto.jpg` →
  nome temporaneo → `Foto.jpg`).
- **Unicode**: macOS scrive gli accenti in forma decomposta (NFD), Windows
  in forma composta (NFC). I percorsi si confrontano in NFC, così `perché.jpg`
  scritto dal Mac e da Windows è lo stesso file.
- **Percorsi lunghi** su Windows: l'app usa i percorsi estesi (`\\?\`) oltre i
  260 caratteri.
- **Link simbolici**: ignorati (exFAT non li supporta).

## Cartelle vuote

Anche le cartelle vuote si allineano, con le stesse regole dei file: una
cartella vuota creata su un disco viene creata sull'altro, una cancellata
diventa una domanda. Una cartella che contiene file non serve tracciarla: esiste
finché esistono i suoi file.

Quando l'utente conferma la cancellazione di tutti i file di una cartella,
viene cancellata anche la cartella, se è rimasta vuota.

## Primo allineamento (adozione)

Quando l'app adotta due dischi che hanno già dei file, la base non c'è ancora.
Il primo allineamento quindi **unisce** i contenuti e **non cancella mai
niente**:

| A | B | Azione |
|---|---|---|
| c'è | manca | copia A → B |
| c'è | c'è, stesso checksum | niente (date allineate) |
| c'è | c'è, checksum diverso | **conflitto** |

Il primo allineamento calcola l'hash di tutti i file presenti su entrambi i
dischi, quindi può durare ore: mostra l'avanzamento e riprende se interrotto.
Alla fine c'è la prima base.

### Indice perso

Se l'indice manca o è illeggibile su entrambi i dischi (ma `disk.json` c'è),
l'app avvisa e riparte come un'adozione: unisce i contenuti senza cancellare
niente. Le cancellazioni fatte dall'ultimo allineamento vanno perse (i file
cancellati tornano dall'altro disco), ma nessun file viene perso.

Se l'indice è illeggibile su un disco solo, si usa quello dell'altro disco.

## Controllo completo

L'allineamento normale si fida di dimensione e data di modifica. Un file che
si rovina sul disco (bit rot) le mantiene uguali, quindi serve un controllo a
parte, che **legge tutti i file** di entrambi i dischi e ne confronta l'hash
con quello della base.

| A | B | Azione |
|---|---|---|
| hash giusto | hash giusto | niente; aggiorna la data dell'ultima verifica |
| hash giusto | hash sbagliato, stessa data | **ripara** B da A (proposto nel riepilogo) |
| hash sbagliato | hash sbagliato | file perso su entrambi: avvisa, non tocca niente |

Un hash diverso con data di modifica diversa non è corruzione ma una modifica:
se ne occupa il normale allineamento.

- L'app lo **propone** quando colleghi i dischi e l'ultimo controllo completo è
  più vecchio di 3 mesi (configurabile). Si può rimandare.
- Mostra l'avanzamento e una stima del tempo. I due dischi si leggono in
  parallelo.
- **Si può interrompere** (anche scollegando i dischi): la data dell'ultima
  verifica è salvata file per file, quindi il controllo successivo riprende dai
  file non ancora verificati.

## Niente cestino

L'app non ha un cestino: un file cancellato o sostituito dall'app è eliminato
definitivamente. Per questo:

- le cancellazioni si chiedono **sempre**, file per file;
- i file sostituiti (modificati su un disco, o il perdente di un conflitto)
  sono elencati nel riepilogo prima di premere **Allinea**;
- una sostituzione avviene solo quando la nuova versione è già scritta e
  verificata (vedi [copia sicura](formato-dati.md#copia-sicura)).

## Cosa viene ignorato

Non vengono allineati, né contano come differenze:

| Cosa | Perché |
|---|---|
| `.hddraid/`, `Scarica HddRaid.html` | file dell'app, gestiti a parte |
| `$RECYCLE.BIN/`, `System Volume Information/` | Windows |
| `.Trashes/`, `.Spotlight-V100/`, `.fseventsd/`, `.TemporaryItems/`, `.DocumentRevisions-V100/` | macOS |
| `.DS_Store`, `._*` (file AppleDouble), `Thumbs.db`, `desktop.ini` | file di servizio di Finder ed Esplora file |
| `.Trash-*/` | cestino di Linux |
| `*.hddraid-partial`, `*.hddraid-rename` | copie e rinomine interrotte dell'app |
| link simbolici | non supportati da exFAT |
