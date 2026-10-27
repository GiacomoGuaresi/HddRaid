# Flussi e schermate

L'app è pensata per chi usa pochissimo il computer.

## Principi

- **Si fa viva da sola**: parte con il computer e resta nella barra di sistema
  (Windows, Linux) o nella barra dei menu (macOS). La finestra si apre quando
  c'è qualcosa da fare.
- **Una cosa alla volta**: ogni schermata ha una domanda e uno o due pulsanti
  grandi.
- **Nessuna parola tecnica**: "i dischi sono uguali", non "sincronizzazione
  completata"; "file rovinato", non "checksum non valido".
- **Dice sempre cosa farà** prima di farlo, e non cancella niente senza
  chiedere.
- **Grafica calda e rassicurante**: deve trasmettere "i tuoi ricordi sono al
  sicuro". Colori morbidi, angoli arrotondati, illustrazioni semplici dei due
  dischi, verde per "tutto a posto". Stessa base di
  [PrintPicker](https://github.com/GiacomoGuaresi/PrintPicker): font Inter,
  icone Lucide. Testi grandi, contrasto alto, tema chiaro e scuro dal sistema,
  animazioni leggere per l'avanzamento.
- Italiano e inglese, dalla lingua del sistema.

## Icona nella barra

| Stato | Icona | Testo |
|---|---|---|
| Nessun disco | grigia | "Collega i tuoi dischi" |
| Un disco | gialla | "Collega anche il disco B" |
| Allineamento in corso | animata | "Sto copiando… 45%" |
| Dischi uguali | verde | "I dischi sono uguali" |
| Serve una risposta | rossa | "Ci sono 3 domande per te" |

Menu dell'icona: **Apri HddRaid**, **Espelli i dischi**, **Esci**.

## 1. Installazione e primo avvio

1. Scarico l'installer dalla pagina di download (o dal file
   `Scarica HddRaid.html` su un disco già preparato).
2. Installo. La pagina di download spiega con immagini come superare l'avviso
   di sicurezza di Windows / macOS (vedi [stack.md](stack.md#firma-del-codice)).
3. Al primo avvio: **benvenuto** in una schermata, poi "Hai già dei dischi
   preparati con HddRaid?"
   - **Sì** → "Collega i tuoi dischi" e si passa all'uso normale.
   - **No, prepariamoli** → flusso 2.

## 2. Preparare i dischi

"Collega i due dischi che vuoi usare." L'app li elenca con nome, dimensione e
contenuto ("contiene 120 GB di file" / "vuoto").

Per ogni disco:

- **disco vuoto o da cancellare** → **formatta**:
  1. "Tutto quello che c'è sul disco *Seagate 2TB* verrà cancellato." Elenco
     delle cartelle principali, per riconoscere il disco.
  2. Per confermare si scrive il nome del disco (o si tiene premuto il
     pulsante per 3 secondi).
  3. Formattazione in exFAT (richiede
     la password di amministratore).
- **disco con file da tenere**:
  - se è exFAT → viene adottato così com'è;
  - se è NTFS o APFS → "Questo disco funziona solo su Windows / solo su Mac.
    Per usarlo su entrambi va formattato, ma prima copia i file da un'altra
    parte." Niente formattazione automatica di un disco con dati.

Poi l'app chiede **come chiamare i dischi**: propone "Disco A" e "Disco B",
ma si può scrivere un nome che aiuti a riconoscerli ("Disco rosso", "Disco di
casa"). Il nome si vede nei messaggi dell'app e in Esplora file / Finder.

Alla fine l'app scrive `.hddraid/` e `Scarica HddRaid.html` su entrambi. Se
almeno un disco aveva già dei file, parte il **primo allineamento** (flusso 3,
con le regole di [adozione](sincronizzazione.md#primo-allineamento-adozione)).

## 3. Uso normale: allineare

1. Collego i due dischi. Notifica: "I tuoi dischi sono collegati". L'app apre
   la coppia e legge solo le date dell'indice: **non confronta i file da
   sola**, così resta leggera e non lavora mentre sto ancora copiando.
2. Copio le foto nuove su uno dei due dischi con Esplora file / Finder, come
   sempre.
3. Schermata **Dischi collegati**: i due dischi, la data dell'ultimo
   allineamento e dell'ultimo controllo, e tre pulsanti: **Allinea**,
   **Controlla** (il controllo completo, flusso 6) ed **Espelli i dischi**.
   Se il controllo completo è scaduto, la schermata lo dice e mette in
   evidenza Controlla.
4. Premo **Allinea**: l'app confronta i dischi.
   - Se sono già uguali, la stessa schermata dice "I dischi sono uguali".
   - Altrimenti mostra il **riepilogo**, a parole semplici e con numeri
     grandi:
     - "34 file nuovi sul disco A → li copio sul disco B (1,2 GB)"
     - "2 file spostati sul disco B → li sposto anche sul disco A"
     - "3 file modificati sul disco A → sostituisco quelli del disco B"
     - "Ci sono 3 domande per te" (cancellazioni e conflitti, flussi 4 e 5)

     Pulsante grande **Allinea** per eseguire. Un link "Mostra i file" apre
     l'elenco dettagliato.
5. **Avanzamento**: barra, file attuale, tempo stimato. Si può chiudere la
   finestra: continua in background.
6. **Fine**: "I dischi sono uguali." Pulsante **Espelli i dischi**: dopo
   l'espulsione, "Puoi scollegare i dischi."

Con un solo disco collegato, notifica: "Disco A collegato. Collega anche il
disco B per renderli uguali."

## 4. Cancellazioni

"Hai cancellato 3 file dal disco A. Cosa faccio?"

Per ogni file (o cartella intera, come voce sola): nome, anteprima per le
immagini, dimensione.

- **Cancella anche dal disco B**
- **Rimettilo sul disco A**

Casella "Fai lo stesso per tutti". Prima di eseguire, una conferma finale:
"Cancello 3 file dal disco B. Non si potranno recuperare." I file da
rimettere vengono copiati dall'altro disco.

## 5. Conflitti

"Questo file è stato cambiato su entrambi i dischi in modo diverso."

Le due versioni affiancate: anteprima, data, dimensione, disco.

- **Tieni questa** (sotto ciascuna versione)
- **Tienile entrambe**: la versione del disco B diventa `nome (disco B).ext`

Si può rimandare ("Decido dopo"): il file resta in sospeso e viene riproposto
al prossimo collegamento; il resto dell'allineamento procede.

## 6. Controllo completo

1. Collego i dischi e sono passati più di 3 mesi dall'ultimo controllo.
   Notifica: "È ora di controllare che i tuoi file siano in salute. Ci vorranno
   circa 3 ore."
2. **Controlla ora** / **Più tardi** (ricorda tra una settimana).
3. Avanzamento con tempo stimato. "Puoi usare il computer normalmente. Puoi
   anche scollegare i dischi: riprenderò da qui la prossima volta."
4. Risultato:
   - tutto bene → "Tutti i 58.214 file sono in salute.";
   - file rovinati su un disco → "2 file si erano rovinati sul disco B. Li
     riparo usando il disco A?" **Ripara**;
   - file rovinati su entrambi → "1 file è rovinato su entrambi i dischi e non
     si può riparare." con il nome del file.

Il controllo si può lanciare anche a mano, da **Impostazioni → Controlla i
dischi adesso**.

## 7. Sostituire un disco rotto

Non c'è una procedura a parte: basta collegare il disco sano e un disco
nuovo.

1. Il disco nuovo compare al posto di quello mancante. Cliccandolo si apre
   il popup del disco: nome, formato, spazio.
2. **Formatta e usa al posto di Disco B**: si sceglie il nome del disco e
   si conferma scrivendo il nome attuale. Il disco viene formattato in exFAT
   e preparato come Disco B.
3. Al prossimo **Allinea** l'app ci ricopia tutto dal disco sano, senza
   domande.

Lo stesso popup, su un disco della coppia, permette di rinominarlo e di
formattarlo: dopo la formattazione torna nella coppia e viene riempito di
nuovo dall'altro disco.

## 8. Casi particolari

| Situazione | Cosa fa l'app |
|---|---|
| Disco di un'altra coppia | "Questo disco appartiene a un'altra coppia." Non lo tocca. |
| Disco scollegato durante l'allineamento | "Il disco B è stato scollegato. Ricollegalo per continuare." Riprende da dove era rimasto. |
| Spazio insufficiente | Prima di iniziare: "Sul disco B mancano 4 GB." Non inizia. |
| Disco in sola lettura | "Non riesco a scrivere sul disco B." con un suggerimento (es. NTFS su Mac). |
| Operazione interrotta (journal) | Al collegamento la completa da sola, poi prosegue. |

## Impostazioni

Una sola schermata, poche voci:

- Avvia HddRaid con il computer (sì di default)
- Controllo completo ogni: 1 / 3 / 6 mesi
- **Storico**: cosa è successo e quando
- Controlla i dischi adesso
- Nomi dei dischi
- Lingua (automatica di default)
