# Stack (proposta)

## Scelta: Tauri 2

| Esigenza | Come si copre |
|---|---|
| Grafica curata | Interfaccia web (HTML/CSS) dentro una finestra nativa: libertà totale su design e animazioni |
| Windows, macOS, Linux | Tauri usa la webview del sistema (WebView2, WKWebView, WebKitGTK): installer piccoli (~10 MB) |
| Icona nella barra, notifiche, avvio con il computer | Funzioni e plugin ufficiali di Tauri (tray, `notification`, `autostart`) |
| Aggiornamenti automatici | Plugin `updater`, con una chiave di firma propria e gratuita (diversa dalla firma del codice) |
| Motore di sincronizzazione | **Rust**, in un crate separato senza dipendenze dalla UI, testabile da solo |
| Indice | SQLite con `rusqlite` (SQLite incluso nel binario) |
| Hash | SHA-256 (crate `sha2`, accelerato in hardware): verificabile anche senza l'app (`shasum`, `certutil`). Il collo di bottiglia è comunque il disco |

**Frontend**: Svelte + TypeScript, con traduzioni in file JSON (italiano e
inglese). Font **Inter** e icone **Lucide**, come in
[PrintPicker](https://github.com/GiacomoGuaresi/PrintPicker), incluse nell'app
(niente caricamenti da internet).

**Log**: un file di log locale sul computer (cartella dati dell'app, ruotato,
pochi MB) per capire cosa è successo in caso di problemi. Nessun dato viene
inviato da nessuna parte.

### Perché non Python + Qt come PhotoSaver

Con PySide6 e PyInstaller si ottengono eseguibili da 100+ MB, una grafica più
difficile da curare e niente aggiornamenti automatici o icona nella barra
pronti all'uso. Con Electron la grafica è la stessa di Tauri, ma l'app pesa
~100 MB e usa molta più memoria: male per un'app che resta sempre in
background.

## Funzioni che dipendono dal sistema

| Funzione | Windows | macOS | Linux |
|---|---|---|---|
| Rilevare i dischi collegati | elenco dei volumi ogni 2 secondi; poi eventi `WM_DEVICECHANGE` | `/Volumes` ogni 2 secondi; poi DiskArbitration | `/proc/self/mounts` ogni 2 secondi; poi udisks2 su D-Bus |
| Formattare in exFAT | `Format-Volume` in PowerShell, con permessi di amministratore | `diskutil eraseDisk ExFAT` | `mkfs.exfat` tramite `pkexec` / udisks2 |
| Espellere | `CM_Request_Device_Eject` | `diskutil eject` | `udisksctl unmount` + `power-off` |
| Riconoscere il file system | `GetVolumeInformationW` | `statfs` | `statfs` / `/proc/self/mounts` |

Il rilevamento parte con un controllo periodico, semplice e uguale ovunque; gli
eventi nativi sono un'ottimizzazione successiva. In ogni volume collegato
l'app cerca `.hddraid/disk.json`.

## Distribuzione

- **Installer**: `.msi` / `.exe` (NSIS) per Windows; `.dmg` universal (Intel e
  Apple Silicon) per macOS; `.AppImage` e `.deb` per Linux.
- **Repository pubblico** su GitHub, licenza **MIT**.
- **Build**: GitHub Actions, una build per sistema, pubblicate su GitHub
  Releases.
- **Pagina di download**: GitHub Pages. Riconosce il sistema e propone il file
  giusto, con le istruzioni illustrate per l'installazione. È la pagina aperta
  da `Scarica HddRaid.html`.

## Firma del codice

Per ora **niente firma** (costa ~99 $/anno per Apple e ~10 $/mese per Windows).
Le conseguenze, da spiegare con immagini nella pagina di download:

- **Windows**: SmartScreen mostra "PC protetto da Windows". Si supera con
  **Ulteriori informazioni → Esegui comunque**. Solo al primo avvio
  dell'installer.
- **macOS**: l'app va comunque firmata *ad hoc* (`codesign -s -`, gratuito),
  altrimenti su Apple Silicon risulta "danneggiata". Al primo avvio macOS la
  blocca; si sblocca da **Impostazioni di Sistema → Privacy e sicurezza → Apri
  comunque**.
- **Linux**: nessun avviso.

È il punto più debole per chi usa poco il computer: la firma va rivalutata
prima di dare l'app ad altri.

## Test

- Il motore (scansione, confronto a tre vie, copia sicura, journal) si testa
  in Rust su cartelle temporanee che fanno da dischi simulati, con scenari di
  guasto (interruzione, file rovinato, disco scollegato).
- Test su **veri volumi exFAT** in CI: immagini disco create con `hdiutil` su
  macOS e con un loop device su Linux, per maiuscole, Unicode e date.
- CI su Windows, macOS e Linux.
