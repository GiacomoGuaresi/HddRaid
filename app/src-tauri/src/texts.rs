//! Testi mostrati fuori dalla finestra: icona nella barra e notifiche. Quelli
//! della finestra sono nei file JSON dell'interfaccia.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    It,
    En,
}

impl Lang {
    /// Dalla scelta dell'utente o, se manca, dalla lingua del sistema.
    pub fn pick(choice: Option<&str>) -> Lang {
        let lang = choice
            .map(str::to_string)
            .or_else(sys_locale::get_locale)
            .unwrap_or_default();
        if lang.to_lowercase().starts_with("it") {
            Lang::It
        } else {
            Lang::En
        }
    }
}

pub struct Texts {
    lang: Lang,
}

impl Texts {
    pub fn new(lang: Lang) -> Texts {
        Texts { lang }
    }

    fn pick(&self, it: String, en: String) -> String {
        match self.lang {
            Lang::It => it,
            Lang::En => en,
        }
    }

    pub fn open(&self) -> String {
        self.pick("Apri HddRaid".into(), "Open HddRaid".into())
    }

    pub fn eject(&self) -> String {
        self.pick("Espelli i dischi".into(), "Eject the drives".into())
    }

    pub fn quit(&self) -> String {
        self.pick("Esci".into(), "Quit".into())
    }

    pub fn no_disks(&self) -> String {
        self.pick("Collega i tuoi dischi".into(), "Connect your drives".into())
    }

    pub fn connect_other(&self, missing: &str) -> String {
        self.pick(
            format!("Collega anche {missing}"),
            format!("Connect {missing} too"),
        )
    }

    pub fn copying(&self, percent: u32) -> String {
        self.pick(
            format!("Sto copiando… {percent}%"),
            format!("Copying… {percent}%"),
        )
    }

    pub fn checking(&self, percent: u32) -> String {
        self.pick(
            format!("Sto controllando i file… {percent}%"),
            format!("Checking files… {percent}%"),
        )
    }

    pub fn comparing(&self) -> String {
        self.pick(
            "Sto confrontando i dischi…".into(),
            "Comparing your drives…".into(),
        )
    }

    pub fn in_sync(&self) -> String {
        self.pick(
            "I dischi sono uguali".into(),
            "Your drives are the same".into(),
        )
    }

    pub fn questions(&self, count: usize) -> String {
        match count {
            1 => self.pick(
                "C'è una domanda per te".into(),
                "I have a question for you".into(),
            ),
            n => self.pick(
                format!("Ci sono {n} domande per te"),
                format!("I have {n} questions for you"),
            ),
        }
    }

    pub fn to_align(&self, files: usize) -> String {
        match files {
            1 => self.pick("1 file da allineare".into(), "1 file to align".into()),
            n => self.pick(
                format!("{n} file da allineare"),
                format!("{n} files to align"),
            ),
        }
    }

    pub fn one_connected(&self, present: &str, missing: &str) -> (String, String) {
        (
            self.pick(
                format!("{present} è collegato"),
                format!("{present} is connected"),
            ),
            self.pick(
                format!("Collega anche {missing} per renderli uguali."),
                format!("Connect {missing} too, so I can make them the same."),
            ),
        )
    }

    pub fn ready(&self, files: usize, questions: usize) -> (String, String) {
        let body = match (files, questions) {
            (0, q) => self.questions(q),
            (f, 0) => self.to_align(f),
            (f, q) => format!("{} · {}", self.to_align(f), self.questions(q)),
        };
        (
            self.pick(
                "I tuoi dischi sono pronti".into(),
                "Your drives are ready".into(),
            ),
            body,
        )
    }

    pub fn already_same(&self) -> (String, String) {
        (
            self.in_sync(),
            self.pick(
                "Non c'è niente da copiare.".into(),
                "There's nothing to copy.".into(),
            ),
        )
    }

    pub fn synced(&self) -> (String, String) {
        (
            self.in_sync(),
            self.pick(
                "Puoi espellerli e scollegarli.".into(),
                "You can eject and unplug them.".into(),
            ),
        )
    }

    pub fn check_due(&self) -> (String, String) {
        (
            self.pick(
                "È ora di controllare i tuoi file".into(),
                "Time to check your files".into(),
            ),
            self.pick(
                "Apri HddRaid per controllare che siano in salute.".into(),
                "Open HddRaid to check that they're healthy.".into(),
            ),
        )
    }

    pub fn checked(&self) -> String {
        self.pick(
            "Controllo completo finito".into(),
            "Full check finished".into(),
        )
    }

    pub fn ejected(&self) -> (String, String) {
        (
            self.pick(
                "Puoi scollegare i dischi".into(),
                "You can unplug the drives".into(),
            ),
            self.pick("Alla prossima!".into(), "See you next time!".into()),
        )
    }
}
