//! Volumi collegati al computer: elenco, spazio ed espulsione.
//!
//! L'elenco si legge a ogni controllo periodico, quindi deve essere veloce:
//! non apre i dischi, guarda solo dove sono montati. In sviluppo, le cartelle
//! elencate in `HDDRAID_DEV_DISKS` (separate come in `PATH`) si comportano
//! come volumi collegati.

use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Variabile d'ambiente con i dischi simulati.
pub const DEV_DISKS_ENV: &str = "HDDRAID_DEV_DISKS";

/// Radici dei volumi esterni collegati, più i dischi simulati.
pub fn mounted() -> Vec<PathBuf> {
    let mut roots = system_volumes();
    roots.extend(dev_disks());
    roots
}

/// Dischi simulati di `HDDRAID_DEV_DISKS` che esistono.
pub fn dev_disks() -> Vec<PathBuf> {
    std::env::var_os(DEV_DISKS_ENV)
        .map(|v| std::env::split_paths(&v).filter(|p| p.is_dir()).collect())
        .unwrap_or_default()
}

/// `true` se `root` è un disco simulato.
pub fn is_dev_disk(root: &Path) -> bool {
    dev_disks().iter().any(|d| d == root)
}

/// Spazio totale e libero del volume che contiene `path`, in byte.
pub fn space(path: &Path) -> io::Result<(u64, u64)> {
    Ok((fs4::total_space(path)?, fs4::available_space(path)?))
}

#[cfg(target_os = "macos")]
fn system_volumes() -> Vec<PathBuf> {
    // In /Volumes c'è anche il disco di sistema, come collegamento a "/".
    let Ok(entries) = std::fs::read_dir("/Volumes") else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
        .map(|e| e.path())
        .collect()
}

#[cfg(target_os = "linux")]
fn system_volumes() -> Vec<PathBuf> {
    let Ok(mounts) = std::fs::read_to_string("/proc/self/mounts") else {
        return Vec::new();
    };
    linux_mounts(&mounts)
        .into_iter()
        .map(|(_, point)| point)
        .filter(|p| {
            ["/media/", "/run/media/", "/mnt/"]
                .iter()
                .any(|d| p.starts_with(d))
        })
        .collect()
}

#[cfg(windows)]
fn system_volumes() -> Vec<PathBuf> {
    // Il disco di sistema non si propone mai: non va né usato né formattato.
    let system = std::env::var("SystemDrive").unwrap_or_else(|_| "C:".into());
    (b'A'..=b'Z')
        .map(|letter| PathBuf::from(format!("{}:\\", letter as char)))
        .filter(|p| {
            !p.to_string_lossy()
                .eq_ignore_ascii_case(&format!("{system}\\"))
        })
        .filter(|p| p.is_dir())
        .collect()
}

#[cfg(not(any(target_os = "macos", target_os = "linux", windows)))]
fn system_volumes() -> Vec<PathBuf> {
    Vec::new()
}

/// Una partizione di un disco rimovibile, montata o no.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Removable {
    /// Il dispositivo, come `/dev/sdb1`.
    pub device: String,
    pub mounted: bool,
}

/// File system che si montano da soli: quelli dei dischi esterni.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
const MOUNTABLE: &[&str] = &["exfat", "vfat", "ntfs"];

/// Le partizioni dei dischi rimovibili che si possono montare. Solo su
/// Linux: macOS e Windows montano da soli i dischi collegati, molti desktop
/// Linux (KDE, per esempio) no.
pub fn removable() -> Vec<Removable> {
    #[cfg(target_os = "linux")]
    {
        let Ok(output) = Command::new("lsblk")
            .args(["-J", "-p", "-o", "PATH,FSTYPE,MOUNTPOINT,HOTPLUG"])
            .output()
        else {
            return Vec::new();
        };
        linux_removable(&String::from_utf8_lossy(&output.stdout))
    }
    #[cfg(not(target_os = "linux"))]
    Vec::new()
}

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn linux_removable(json: &str) -> Vec<Removable> {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(json) else {
        return Vec::new();
    };
    let devices = value["blockdevices"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    devices
        .iter()
        .filter(|d| {
            // `true` nelle versioni recenti di lsblk, "1" in quelle vecchie.
            d["hotplug"].as_bool() == Some(true) || d["hotplug"].as_str() == Some("1")
        })
        .filter(|d| {
            d["fstype"]
                .as_str()
                .is_some_and(|fs| MOUNTABLE.contains(&fs))
        })
        .filter_map(|d| {
            Some(Removable {
                device: d["path"].as_str()?.to_string(),
                mounted: d["mountpoint"].as_str().is_some(),
            })
        })
        .collect()
}

/// Monta il dispositivo `device` come farebbe il desktop, senza permessi di
/// amministratore, e restituisce la sua radice.
#[cfg(target_os = "linux")]
pub fn mount(device: &str) -> io::Result<PathBuf> {
    let output = Command::new("udisksctl")
        .args(["mount", "-b", device])
        .output()?;
    // "Mounted /dev/sdb1 at /media/giacomo/Disco B"
    match String::from_utf8_lossy(&output.stdout)
        .trim()
        .split_once(" at ")
    {
        Some((_, point)) => Ok(PathBuf::from(point.trim_end_matches('.'))),
        None => Err(io::Error::other(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        )),
    }
}

#[cfg(not(target_os = "linux"))]
pub fn mount(_device: &str) -> io::Result<PathBuf> {
    Err(io::Error::from(io::ErrorKind::Unsupported))
}

/// Nome e file system di un volume.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VolumeInfo {
    /// Il nome del volume, come lo mostrano Esplora file e Finder.
    pub name: String,
    /// "exfat", "ntfs", "apfs"…, in minuscolo; `None` se non si sa.
    pub filesystem: Option<String>,
}

/// Lunghezza massima del nome di un volume exFAT.
pub const MAX_LABEL: usize = 11;

/// Perché un nome non va bene per un volume exFAT.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LabelError {
    Empty,
    TooLong,
    BadChar(char),
}

/// Controlla che `name` vada bene come nome di un volume exFAT.
pub fn check_label(name: &str) -> Result<(), LabelError> {
    if name.trim().is_empty() {
        return Err(LabelError::Empty);
    }
    if name.encode_utf16().count() > MAX_LABEL {
        return Err(LabelError::TooLong);
    }
    match name
        .chars()
        .find(|c| c.is_control() || "\"*/:<>?\\|".contains(*c))
    {
        Some(c) => Err(LabelError::BadChar(c)),
        None => Ok(()),
    }
}

/// Nome e file system del volume con radice `root`. Per un disco simulato,
/// il nome è quello della cartella.
pub fn info(root: &Path) -> VolumeInfo {
    let folder_name = || {
        root.file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| root.to_string_lossy().into_owned())
    };
    if is_dev_disk(root) {
        return VolumeInfo {
            name: folder_name(),
            filesystem: None,
        };
    }
    #[cfg(windows)]
    {
        windows_info(root).unwrap_or_else(|| VolumeInfo {
            name: folder_name(),
            filesystem: None,
        })
    }
    #[cfg(not(windows))]
    {
        VolumeInfo {
            name: folder_name(),
            filesystem: unix_filesystem(root),
        }
    }
}

#[cfg(target_os = "macos")]
fn unix_filesystem(root: &Path) -> Option<String> {
    let output = Command::new("mount").output().ok()?;
    macos_mount_fs(&String::from_utf8_lossy(&output.stdout), root)
}

#[cfg(target_os = "linux")]
fn unix_filesystem(root: &Path) -> Option<String> {
    let text = std::fs::read_to_string("/proc/self/mounts").ok()?;
    text.lines().find_map(|line| {
        let mut parts = line.split(' ');
        let _device = parts.next()?;
        let point = parts.next()?;
        let fs = parts.next()?;
        (linux_mounts(line).first()?.1 == root && !point.is_empty()).then(|| fs.to_lowercase())
    })
}

#[cfg(not(any(target_os = "macos", target_os = "linux", windows)))]
fn unix_filesystem(_root: &Path) -> Option<String> {
    None
}

/// File system dall'output di `mount` su macOS:
/// `/dev/disk4s1 on /Volumes/Disco A (exfat, local, nodev)`.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
fn macos_mount_fs(text: &str, root: &Path) -> Option<String> {
    text.lines().find_map(|line| {
        let (_, rest) = line.split_once(" on ")?;
        let (point, options) = rest.rsplit_once(" (")?;
        (Path::new(point) == root).then(|| {
            options
                .split([',', ')'])
                .next()
                .unwrap_or("")
                .trim()
                .to_lowercase()
        })
    })
}

#[cfg(windows)]
fn windows_info(root: &Path) -> Option<VolumeInfo> {
    let letter = root.to_string_lossy().chars().next()?;
    let script = format!(
        "$v = Get-Volume -DriveLetter {letter}; \"$($v.FileSystemLabel)|$($v.FileSystemType)\""
    );
    let output = powershell(&script, false).ok()?;
    let (name, fs) = output.trim().split_once('|')?;
    Some(VolumeInfo {
        name: if name.is_empty() {
            format!("{letter}:")
        } else {
            name.to_string()
        },
        filesystem: (!fs.is_empty()).then(|| fs.to_lowercase()),
    })
}

/// Rinomina il volume. Su Windows e Linux il sistema chiede la password di
/// amministratore. Un disco simulato non si rinomina: il nome resta in
/// `disk.json`.
pub fn rename(root: &Path, name: &str) -> io::Result<()> {
    check_label(name).map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, format!("{e:?}")))?;
    if is_dev_disk(root) {
        return Err(io::Error::from(io::ErrorKind::Unsupported));
    }
    rename_system(root, name)
}

#[cfg(target_os = "macos")]
fn rename_system(root: &Path, name: &str) -> io::Result<()> {
    run(Command::new("diskutil").arg("rename").arg(root).arg(name))
}

#[cfg(target_os = "linux")]
fn rename_system(root: &Path, name: &str) -> io::Result<()> {
    let device = linux_device(root)?;
    run(Command::new("pkexec").args(["exfatlabel", &device, name]))
}

#[cfg(windows)]
fn rename_system(root: &Path, name: &str) -> io::Result<()> {
    let letter = root.to_string_lossy().chars().next().unwrap_or('?');
    let name = name.replace('\'', "''");
    powershell(
        &format!("Set-Volume -DriveLetter {letter} -NewFileSystemLabel '{name}'"),
        true,
    )
    .map(|_| ())
}

#[cfg(not(any(target_os = "macos", target_os = "linux", windows)))]
fn rename_system(_root: &Path, _name: &str) -> io::Result<()> {
    Err(io::Error::from(io::ErrorKind::Unsupported))
}

/// Formatta il volume in exFAT con il nome `name` e restituisce la sua nuova
/// radice (su macOS e Linux cambia con il nome). **Cancella tutto.** Un disco
/// simulato viene svuotato.
pub fn format_exfat(root: &Path, name: &str) -> io::Result<PathBuf> {
    check_label(name).map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, format!("{e:?}")))?;
    if is_dev_disk(root) {
        for entry in std::fs::read_dir(root)? {
            let path = entry?.path();
            if path.is_dir() {
                std::fs::remove_dir_all(&path)?;
            } else {
                std::fs::remove_file(&path)?;
            }
        }
        return Ok(root.to_path_buf());
    }
    format_system(root, name)
}

#[cfg(target_os = "macos")]
fn format_system(root: &Path, name: &str) -> io::Result<PathBuf> {
    let device = diskutil_field(&root.to_string_lossy(), "Device Identifier")?;
    run(Command::new("diskutil")
        .args(["eraseVolume", "ExFAT", name])
        .arg(root))?;
    diskutil_field(&device, "Mount Point").map(PathBuf::from)
}

/// Un campo di `diskutil info`, come "Device Identifier" o "Mount Point".
#[cfg(target_os = "macos")]
fn diskutil_field(target: &str, field: &str) -> io::Result<String> {
    let output = Command::new("diskutil").args(["info", target]).output()?;
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .find_map(|line| {
            let (key, value) = line.split_once(':')?;
            (key.trim() == field).then(|| value.trim().to_string())
        })
        .filter(|v| !v.is_empty())
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, format!("{field}: non trovato")))
}

#[cfg(target_os = "linux")]
fn format_system(root: &Path, name: &str) -> io::Result<PathBuf> {
    let device = linux_device(root)?;
    run(Command::new("udisksctl").args(["unmount", "-b", &device]))?;
    run(Command::new("pkexec").args(["mkfs.exfat", "-n", name, &device]))?;
    // Subito dopo mkfs udisks non conosce ancora il nuovo filesystem e il
    // mount fallisce: si aspetta udev e si riprova per qualche secondo.
    let _ = Command::new("udevadm").arg("settle").status();
    let mut error = String::new();
    for _ in 0..10 {
        match mount(&device) {
            Ok(root) => return Ok(root),
            Err(e) => error = e.to_string(),
        }
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
    Err(io::Error::other(format!(
        "disco formattato ma non montato: {error}"
    )))
}

#[cfg(windows)]
fn format_system(root: &Path, name: &str) -> io::Result<PathBuf> {
    let letter = root.to_string_lossy().chars().next().unwrap_or('?');
    let name = name.replace('\'', "''");
    powershell(
        &format!(
            "Format-Volume -DriveLetter {letter} -FileSystem exFAT -NewFileSystemLabel '{name}' -Confirm:$false -Force"
        ),
        true,
    )?;
    Ok(root.to_path_buf())
}

#[cfg(not(any(target_os = "macos", target_os = "linux", windows)))]
fn format_system(_root: &Path, _name: &str) -> io::Result<PathBuf> {
    Err(io::Error::from(io::ErrorKind::Unsupported))
}

#[cfg(target_os = "linux")]
fn linux_device(root: &Path) -> io::Result<String> {
    let mounts = std::fs::read_to_string("/proc/self/mounts")?;
    linux_mounts(&mounts)
        .into_iter()
        .find(|(_, point)| point == root)
        .map(|(device, _)| device)
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "volume non montato"))
}

/// Esegue uno script PowerShell e ne restituisce l'output. Con `elevated`
/// lo lancia con i permessi di amministratore (Windows chiede conferma).
#[cfg(windows)]
fn powershell(script: &str, elevated: bool) -> io::Result<String> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let script = if elevated {
        let inner = script.replace('\'', "''");
        format!(
            "$p = Start-Process powershell -Verb RunAs -Wait -PassThru -WindowStyle Hidden \
             -ArgumentList '-NoProfile','-Command','{inner}'; exit $p.ExitCode"
        )
    } else {
        script.to_string()
    };
    let output = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .creation_flags(CREATE_NO_WINDOW)
        .output()?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        Err(io::Error::other(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ))
    }
}

/// Righe di `/proc/self/mounts`: dispositivo e punto di montaggio, con gli
/// spazi decodificati (`\040`).
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn linux_mounts(text: &str) -> Vec<(String, PathBuf)> {
    fn unescape(s: &str) -> String {
        let mut out = String::new();
        let mut rest = s;
        while let Some(i) = rest.find('\\') {
            out.push_str(&rest[..i]);
            let code = rest.get(i + 1..i + 4);
            match code.and_then(|c| u8::from_str_radix(c, 8).ok()) {
                Some(b) => {
                    out.push(b as char);
                    rest = &rest[i + 4..];
                }
                None => {
                    out.push('\\');
                    rest = &rest[i + 1..];
                }
            }
        }
        out.push_str(rest);
        out
    }
    text.lines()
        .filter_map(|line| {
            let mut parts = line.split(' ');
            let device = parts.next()?;
            let point = parts.next()?;
            Some((unescape(device), PathBuf::from(unescape(point))))
        })
        .collect()
}

/// Espelle il volume con radice `root`, così si può scollegare. Un disco
/// simulato non si espelle.
pub fn eject(root: &Path) -> io::Result<()> {
    if is_dev_disk(root) {
        return Ok(());
    }
    eject_system(root)
}

#[cfg(target_os = "macos")]
fn eject_system(root: &Path) -> io::Result<()> {
    run(Command::new("diskutil").arg("eject").arg(root))
}

#[cfg(target_os = "linux")]
fn eject_system(root: &Path) -> io::Result<()> {
    let mounts = std::fs::read_to_string("/proc/self/mounts")?;
    let device = linux_mounts(&mounts)
        .into_iter()
        .find(|(_, point)| point == root)
        .map(|(device, _)| device)
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "volume non montato"))?;
    run(Command::new("udisksctl").args(["unmount", "-b", &device]))?;
    // Spegnere il disco è facoltativo: non tutti i dispositivi lo permettono.
    let _ = run(Command::new("udisksctl").args(["power-off", "-b", &device]));
    Ok(())
}

#[cfg(windows)]
fn eject_system(root: &Path) -> io::Result<()> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let drive = root.to_string_lossy().trim_end_matches('\\').to_string();
    let script = format!(
        "(New-Object -ComObject Shell.Application).Namespace(17).ParseName('{drive}').InvokeVerb('Eject')"
    );
    run(Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .creation_flags(CREATE_NO_WINDOW))?;
    // Il comando non segnala gli errori: si controlla che il volume sia sparito.
    for _ in 0..20 {
        if !root.exists() {
            return Ok(());
        }
        std::thread::sleep(std::time::Duration::from_millis(250));
    }
    Err(io::Error::other("il disco è ancora in uso"))
}

#[cfg(not(any(target_os = "macos", target_os = "linux", windows)))]
fn eject_system(_root: &Path) -> io::Result<()> {
    Err(io::Error::from(io::ErrorKind::Unsupported))
}

#[cfg_attr(
    not(any(target_os = "macos", target_os = "linux", windows)),
    allow(dead_code)
)]
fn run(command: &mut Command) -> io::Result<()> {
    let output = command.output()?;
    if output.status.success() {
        Ok(())
    } else {
        let message = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(io::Error::other(if message.is_empty() {
            format!("{:?} non è riuscito", command.get_program())
        } else {
            message
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_linux_mounts() {
        let text = "/dev/sda1 / ext4 rw 0 0\n\
                    /dev/sdb1 /media/giacomo/Disco\\040A exfat rw 0 0\n";
        let mounts = linux_mounts(text);
        assert_eq!(mounts.len(), 2);
        assert_eq!(mounts[1].0, "/dev/sdb1");
        assert_eq!(mounts[1].1, Path::new("/media/giacomo/Disco A"));
    }

    #[test]
    fn parses_linux_removable() {
        let json = r#"{"blockdevices": [
            {"path": "/dev/nvme0n1p2", "fstype": "ext4", "mountpoint": "/", "hotplug": false},
            {"path": "/dev/sdc", "fstype": null, "mountpoint": null, "hotplug": true},
            {"path": "/dev/sdc1", "fstype": "exfat", "mountpoint": null, "hotplug": true},
            {"path": "/dev/sdd1", "fstype": "exfat", "mountpoint": "/media/g/B", "hotplug": "1"},
            {"path": "/dev/sdd2", "fstype": "ext4", "mountpoint": null, "hotplug": true}
        ]}"#;
        let devices = linux_removable(json);
        assert_eq!(
            devices,
            [
                Removable {
                    device: "/dev/sdc1".into(),
                    mounted: false
                },
                Removable {
                    device: "/dev/sdd1".into(),
                    mounted: true
                },
            ]
        );
    }

    #[test]
    fn labels() {
        assert_eq!(check_label("Disco rosso"), Ok(()));
        assert_eq!(check_label("Perché sì"), Ok(()));
        assert_eq!(check_label(" "), Err(LabelError::Empty));
        assert_eq!(check_label("Disco di casa"), Err(LabelError::TooLong));
        assert_eq!(check_label("Foto/2024"), Err(LabelError::BadChar('/')));
    }

    #[test]
    fn parses_macos_mount() {
        let text = "/dev/disk3s1s1 on / (apfs, sealed, local, read-only)\n\
                    /dev/disk4s1 on /Volumes/Disco A (exfat, local, nodev, nosuid)\n";
        assert_eq!(
            macos_mount_fs(text, Path::new("/Volumes/Disco A")).as_deref(),
            Some("exfat")
        );
        assert_eq!(macos_mount_fs(text, Path::new("/Volumes/Altro")), None);
    }

    #[test]
    fn space_of_a_folder() {
        let dir = tempfile::tempdir().unwrap();
        let (total, free) = space(dir.path()).unwrap();
        assert!(total > 0 && free <= total);
    }
}
