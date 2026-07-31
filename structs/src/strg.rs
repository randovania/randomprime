use auto_struct_macros::auto_struct;
use reader_writer::{
    FourCC, IteratorArray, LCow, LazyArray, LazyUtf16beStr, Readable, RoArray, RoArrayIter,
};

static SUPPORTED_LANGUAGES: &[&[u8; 4]] = &[
    b"ENGL", b"DUTC", b"FREN", b"GERM", b"ITAL", b"JAPN", b"SPAN",
];

pub static NON_JPN_LANGUAGES: &[&[u8; 4]] = &[b"ENGL", b"DUTC", b"FREN", b"GERM", b"ITAL", b"SPAN"];

const EMPTY_STRING: &str = "\u{0}";
const JPN_FONT_PREFIX: &str = "&line-extra-space=4;&font=C29C51F1;";

pub enum Languages {
    All,
    Some(&'static [&'static [u8; 4]]),
}

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct Strg<'r> {
    #[auto_struct(expect = 0x87654321)]
    magic: u32,
    #[auto_struct(expect = 0)]
    version: u32,

    #[auto_struct(derive = string_tables.len() as u32)]
    lang_count: u32,

    #[auto_struct(derive = {
        let mut lengths = string_tables.iter().map(|table| table.strings.len());
        let count = lengths.next().unwrap();
        assert!(
            lengths.all(|len| len == count),
            "STRG language tables hold differing numbers of strings. Fix: Call `pad_string_tables after` \
             appending to a subset of the languages"
        );
        count as u32
    })]
    string_count: u32,

    #[auto_struct(derive_from_iter = string_tables.iter()
        .scan(0usize, &|sum: &mut usize, t: LCow<StrgStringTable>| {
            let r = StrgLang { lang: t.lang, offset: *sum as u32, };
            *sum += t.size();
            Some(r)
        }))]
    #[auto_struct(init = (lang_count as usize, ()))]
    langs: RoArray<'r, StrgLang>,
    #[auto_struct(init = StrgLangIter(string_count as usize, langs.iter()))]
    pub string_tables: IteratorArray<'r, StrgStringTable<'r>, StrgLangIter<'r>>,

    #[auto_struct(pad_align = 32)]
    _pad: (),
}

impl<'r> Strg<'r> {
    // Grow every language table to the length of the longest, so the file-wide string count in
    // the header stays accurate. Only needed after pushing onto tables directly.
    pub fn pad_string_tables(&mut self) {
        let tables = self.string_tables.as_mut_vec();
        let longest = tables
            .iter()
            .map(|table| table.strings.len())
            .max()
            .unwrap_or(0);

        for table in tables.iter_mut() {
            let strings = table.strings.as_mut_vec();
            while strings.len() < longest {
                strings.push(EMPTY_STRING.to_string().into());
            }
        }
    }

    pub fn add_strings(&mut self, strings: &[String], languages: Languages) {
        let languages = match languages {
            Languages::All => SUPPORTED_LANGUAGES,
            Languages::Some(value) => value,
        };

        for table in self.string_tables.as_mut_vec().iter_mut() {
            let selected = languages.contains(&table.lang.as_bytes());
            for string in strings.iter() {
                let string = match selected {
                    true => string.to_string(),
                    false => EMPTY_STRING.to_string(),
                };
                table.strings.as_mut_vec().push(string.into());
            }
        }
    }

    pub fn edit_strings(&mut self, (from, to): (String, String), languages: Languages) {
        let languages = match languages {
            Languages::All => SUPPORTED_LANGUAGES,
            Languages::Some(value) => value,
        };
        for table in self.string_tables.as_mut_vec().iter_mut() {
            if languages.contains(&table.lang.as_bytes()) {
                for string in table.strings.iter_mut() {
                    if string.contains(&from) {
                        string.replace(&from, &to);
                    }
                }
            }
        }
    }

    pub fn string_at(&self, index: usize, language: &[u8; 4]) -> Option<String> {
        self.string_tables
            .iter()
            .find(|table| table.lang == language.into())?
            .strings
            .iter()
            .nth(index)
            .map(|string| string.into_owned().into_string())
    }

    pub fn set_string(&mut self, index: usize, string: &str, languages: Languages) {
        self.edit_string_at(index, languages, |_| string.to_string());
    }

    // Keeps each language's own text and adds to it, for suffixes that read the same everywhere.
    pub fn append_to_string(&mut self, index: usize, suffix: &str, languages: Languages) {
        self.edit_string_at(index, languages, |string| {
            format!(
                "{}{}{}",
                string.trim_end_matches('\u{0}'),
                suffix,
                EMPTY_STRING
            )
        });
    }

    fn edit_string_at(
        &mut self,
        index: usize,
        languages: Languages,
        edit: impl Fn(String) -> String,
    ) {
        let languages = match languages {
            Languages::All => SUPPORTED_LANGUAGES,
            Languages::Some(value) => value,
        };

        for table in self.string_tables.as_mut_vec().iter_mut() {
            if !languages.contains(&table.lang.as_bytes()) {
                continue;
            }
            let strings = table.strings.as_mut_vec();
            strings[index] = edit(strings[index].clone().into_string()).into();
        }
    }

    pub fn from_strings(strings: Vec<String>) -> Strg<'r> {
        Strg {
            string_tables: vec![StrgStringTable {
                lang: b"ENGL".into(),
                strings: strings
                    .into_iter()
                    .map(|i| i.into())
                    .collect::<Vec<_>>()
                    .into(),
            }]
            .into(),
        }
    }

    pub fn from_strings_jpn(strings: Vec<String>) -> Strg<'r> {
        let strings: LazyArray<LazyUtf16beStr> = strings
            .into_iter()
            .map(|i| format!("{}{}", JPN_FONT_PREFIX, i).into())
            .collect::<Vec<_>>()
            .into();
        Strg {
            string_tables: vec![
                StrgStringTable {
                    lang: b"ENGL".into(),
                    strings: strings.clone(),
                },
                StrgStringTable {
                    lang: b"JAPN".into(),
                    strings,
                },
            ]
            .into(),
        }
    }

    pub fn from_strings_pal(strings: Vec<String>) -> Strg<'r> {
        let strings: LazyArray<LazyUtf16beStr> = strings
            .into_iter()
            .map(|i| i.into())
            .collect::<Vec<_>>()
            .into();
        Strg {
            string_tables: vec![
                StrgStringTable {
                    lang: b"ENGL".into(),
                    strings: strings.clone(),
                },
                StrgStringTable {
                    lang: b"FREN".into(),
                    strings: strings.clone(),
                },
                StrgStringTable {
                    lang: b"GERM".into(),
                    strings: strings.clone(),
                },
                StrgStringTable {
                    lang: b"SPAN".into(),
                    strings: strings.clone(),
                },
                StrgStringTable {
                    lang: b"ITAL".into(),
                    strings: strings.clone(),
                },
                StrgStringTable {
                    lang: b"JAPN".into(),
                    strings,
                },
            ]
            .into(),
        }
    }
}

#[doc(hidden)]
#[derive(Clone, Debug)]
pub struct StrgLangIter<'r>(usize, RoArrayIter<'r, StrgLang>);
impl Iterator for StrgLangIter<'_> {
    type Item = (usize, FourCC);
    fn next(&mut self) -> Option<Self::Item> {
        self.1.next().map(|i| (self.0, i.lang))
    }
}
impl ExactSizeIterator for StrgLangIter<'_> {
    fn len(&self) -> usize {
        self.1.len()
    }
}

#[auto_struct(Readable, Writable, FixedSize)]
#[derive(Debug, Clone)]
struct StrgLang {
    pub lang: FourCC,
    pub offset: u32,
}

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct StrgStringTable<'r> {
    #[auto_struct(args = (string_count, lang))]
    _args: (usize, FourCC),

    #[auto_struct(literal = lang)]
    pub lang: FourCC,

    #[auto_struct(derive = (strings.len() * 4 + strings.iter()
        .map(&|i: LCow<LazyUtf16beStr>| i.size())
        .sum::<usize>()) as u32)]
    _size: u32,

    #[auto_struct(derive_from_iter = strings.iter()
        .scan(strings.len() as u32 * 4, &|st: &mut u32, i: LCow<LazyUtf16beStr>| {
            let r = *st;
            *st += i.size() as u32;
            Some(r)
        }))]
    #[auto_struct(init = (string_count, ()))]
    _offsets: RoArray<'r, u32>,
    #[auto_struct(init = (string_count, ()))]
    pub strings: LazyArray<'r, LazyUtf16beStr<'r>>,
}
