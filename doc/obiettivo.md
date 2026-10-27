# HddRaid — Obiettivo del progetto

## In una frase

Un'app che tiene due hard disk esterni con lo stesso contenuto, come un RAID 1:
carico i file su uno qualsiasi dei due e l'app li allinea.

## Il problema

Uso due hard disk come archivio per foto e documenti importanti. Il secondo è
la copia di riserva, se il primo si rompe. Ogni tanto li collego al computer e
ci carico i file nuovi.

Oggi devo copiare tutto due volte, a mano, e non ho un modo semplice per sapere
se i due dischi sono davvero uguali o se un file si è rovinato.

## Per chi è

Per chi usa **pochissimo il computer**. Ne segue che:

- l'app deve farsi viva da sola: non serve ricordarsi di aprirla;
- grafica curata, testi grandi, pochi pulsanti, **nessuna parola tecnica**;
- una cosa alla volta, e l'app dice sempre cosa sta per fare;
- nessun file viene cancellato o sovrascritto senza chiedere.

## Cosa deve fare

- **Restare in background** (icona nella barra di sistema / barra dei menu) e
  partire con il computer.
- **Riconoscere i dischi** appena collegati.
  - Un solo disco: avvisa e aspetta l'altro ("Collega anche il disco B"). Il
    disco si può usare normalmente: le novità passano sull'altro al prossimo
    collegamento di entrambi.
  - Entrambi: confronta i dischi e mostra un **riepilogo** semplice ("34 file
    nuovi da copiare sul disco B").
- **Allineare** i dischi con un pulsante, dopo il riepilogo:
  - file nuovi o modificati su un disco → copiati sull'altro, con verifica;
  - file spostati o rinominati su un disco → spostati anche sull'altro, senza
    ricopiarli;
  - file cancellati da un disco → l'app **chiede ogni volta**, file per file
    (con "applica a tutti"), se cancellarli anche dall'altro disco o rimetterli;
  - **conflitti** (stesso file cambiato in modo diverso sui due dischi, o
    cambiato su uno e cancellato sull'altro) → decido io.
- **Controllo completo** (checksum di tutti i file) per trovare i file rovinati
  (bit rot) e ripararli dalla copia buona. È separato dall'allineamento perché
  dura ore; l'app lo propone ogni 3 mesi (configurabile), si può interrompere e
  riprende da dove era rimasto.
- **Preparare i dischi**:
  - dischi nuovi: formattazione guidata in exFAT, con conferme esplicite;
  - dischi che uso già: l'app li **adotta** e il primo allineamento unisce i
    contenuti senza cancellare niente.
- **Sostituire un disco rotto**: si collega un disco nuovo, lo si formatta dal
  suo popup e al prossimo allineamento l'app ci copia tutto dal disco sano.
- **Espellere i dischi** a fine lavoro, con un pulsante.

Le regole precise sono in [sincronizzazione.md](sincronizzazione.md), le
schermate in [flussi.md](flussi.md).

## Vincoli

- **Entrambi i dischi collegati** per allineare e per il controllo completo.
- **Tutto il disco** viene allineato, tranne le cartelle di sistema (cestino di
  Windows e macOS, indicizzazione di Spotlight, `Thumbs.db`, `.DS_Store`…) e la
  cartella dell'app. Elenco completo in [sincronizzazione.md](sincronizzazione.md#cosa-viene-ignorato).
- **File system exFAT**, leggibile e scrivibile da Windows, macOS e Linux. Un
  disco NTFS (sul Mac in sola lettura) o APFS (illeggibile su Windows) non viene
  accettato: l'app spiega perché e propone la formattazione.
- **Più computer**: indice e storico stanno **sui dischi**, quindi i dischi si
  possono collegare a qualsiasi computer con l'app installata.
- **App installata sul computer**, non sul disco. Sul disco c'è solo un file
  `Scarica HddRaid.html` che apre la pagina di download, per chi collega i
  dischi a un computer senza l'app.
- **Piattaforme**: Windows, macOS, Linux.
- **Lingue**: italiano e inglese, scelta dalla lingua del sistema.
- **Dimensioni**: fino a qualche TB e qualche centinaio di migliaia di file.
  Allineare quando non è cambiato niente deve richiedere pochi secondi.
- **Senza firma del codice**, per ora: all'installazione Windows e macOS
  mostrano un avviso. La pagina di download spiega come superarlo, con
  immagini (vedi [stack.md](stack.md#firma-del-codice)).

## Fuori scope

- Copiare file dal computer ai dischi: si fa con Esplora file / Finder.
- Visualizzare foto, miniature, ricerca, mappa.
- Organizzare, rinominare o togliere i doppioni.
- Più di due dischi. I dati però sono pensati per N dischi (i dischi sono righe
  dell'indice), così una terza copia si potrà aggiungere in futuro.
- Backup nel cloud, app per il telefono.
