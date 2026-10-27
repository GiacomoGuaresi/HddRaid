# Documentazione

- [Obiettivo](obiettivo.md): problema, requisiti, vincoli, cosa è fuori scope
- [Flussi](flussi.md): schermate e flussi principali
- [Sincronizzazione](sincronizzazione.md): come l'app capisce cosa è cambiato e cosa fare
- [Formato dei dati](formato-dati.md): cosa scrive l'app sui dischi
- [Stack](stack.md): tecnologie, installazione, distribuzione
- [Roadmap](roadmap.md): milestone e step
- [Sviluppo](sviluppo.md): comandi e dischi simulati

## Da PhotoSaver

HddRaid nasce da [PhotoSaver](https://github.com/GiacomoGuaresi/PhotoSaver),
che non è mai arrivato in produzione perché faceva troppe cose (caricamento
guidato, visualizzatore, miniature, mappa…).

| Riprendiamo | Togliamo |
|---|---|
| Mirroring a livello di app su dischi exFAT normali | Caricamento dall'app: i file si copiano con Esplora file / Finder |
| Indice SQLite sui dischi, senza WAL | Visualizzatore, miniature, mappa, ricerca |
| Copia sicura: file temporaneo → `fsync` → verifica hash → rinomina | Filtro per tipo di file: si allinea tutto |
| Journal per le operazioni interrotte | Scelta della destinazione, doppioni |
| Check completo con ripresa e riconoscimento del bit rot | App portabile: ora è installata sul computer |
| Inizializzazione e sostituzione dei dischi | Cestino dell'app: i file si cancellano solo dopo una conferma esplicita |
