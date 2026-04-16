use std::{
    char::{decode_utf16, DecodeUtf16, REPLACEMENT_CHARACTER},
    cmp, fmt, io,
    str::Chars,
};

use crate::{
    reader::{Readable, Reader},
    writer::Writable,
};

#[derive(Clone)]
pub struct Utf16beStr<'r>(Reader<'r>);

impl<'r> Utf16beStr<'r> {
    pub fn chars(&self) -> DecodeUtf16<U16beIter<'r>> {
        decode_utf16(U16beIter(self.0.clone()))
    }
}

impl<'r> Readable<'r> for Utf16beStr<'r> {
    type Args = ();
    fn read_from(reader: &mut Reader<'r>, (): ()) -> Self {
        let start_reader = reader.clone();
        loop {
            if reader.read::<u16>(()) == 0 {
                break;
            }
        }
        let read_len = start_reader.len() - reader.len();
        Utf16beStr(start_reader.truncated(read_len))
    }

    fn size(&self) -> usize {
        self.0.len()
    }
}

impl<'r2> cmp::PartialEq<Utf16beStr<'r2>> for Utf16beStr<'_> {
    fn eq(&self, other: &Utf16beStr<'r2>) -> bool {
        self.chars().eq(other.chars())
    }
}

impl cmp::PartialEq<str> for Utf16beStr<'_> {
    fn eq(&self, other: &str) -> bool {
        self.chars()
            .map(|r| r.unwrap_or(REPLACEMENT_CHARACTER))
            .eq(other.chars())
    }
}

impl fmt::Debug for Utf16beStr<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        fmt::Debug::fmt(&self.chars().map(|i| i.unwrap()).collect::<String>(), f)
    }
}

impl Writable for Utf16beStr<'_> {
    fn write_to<W: io::Write>(&self, w: &mut W) -> io::Result<u64> {
        w.write_all(&self.0)?;
        Ok(self.0.len() as u64)
    }
}

#[derive(Clone, Debug)]
pub struct U16beIter<'r>(Reader<'r>);

impl Iterator for U16beIter<'_> {
    type Item = u16;
    fn next(&mut self) -> Option<Self::Item> {
        if self.0.len() > 0 {
            Some(self.0.read(()))
        } else {
            None
        }
    }
}

#[derive(Clone, Debug)]
pub enum LazyUtf16beStr<'r> {
    Owned(String),
    Borrowed(Utf16beStr<'r>),
}

impl<'r> LazyUtf16beStr<'r> {
    pub fn contains(&self, text: &String) -> bool {
        match *self {
            LazyUtf16beStr::Owned(ref s) => (*s).contains(text),
            LazyUtf16beStr::Borrowed(ref s) => {
                let string = s.chars().map(|i| i.unwrap()).collect::<String>();
                LazyUtf16beStr::Owned(string).contains(text)
            }
        }
    }

    pub fn replace(&mut self, from: &String, to: &str) -> &mut Self {
        *self = match *self {
            LazyUtf16beStr::Owned(ref mut s) => LazyUtf16beStr::from((*s).replace(from, to)),
            LazyUtf16beStr::Borrowed(ref s) => {
                let string = s.chars().map(|i| i.unwrap()).collect::<String>();
                LazyUtf16beStr::Owned(string.replace(from, to))
            }
        };
        self
    }

    pub fn as_mut_string(&mut self) -> &mut String {
        *self = match *self {
            LazyUtf16beStr::Owned(ref mut s) => return s,
            LazyUtf16beStr::Borrowed(ref s) => {
                LazyUtf16beStr::Owned(s.chars().map(|i| i.unwrap()).collect())
            }
        };
        self.as_mut_string()
    }

    pub fn into_string(self) -> String {
        match self {
            LazyUtf16beStr::Owned(s) => s,
            LazyUtf16beStr::Borrowed(s) => s.chars().map(|i| i.unwrap()).collect(),
        }
    }

    pub fn chars<'s>(&'s self) -> LazyUtf16beStrChars<'r, 's> {
        match *self {
            LazyUtf16beStr::Owned(ref s) => LazyUtf16beStrChars::Owned(s.chars()),
            LazyUtf16beStr::Borrowed(ref s) => LazyUtf16beStrChars::Borrowed(s.chars()),
        }
    }
}

impl cmp::PartialEq<str> for LazyUtf16beStr<'_> {
    fn eq(&self, other: &str) -> bool {
        self.chars().eq(other.chars())
    }
}

impl<'r2> cmp::PartialEq<Utf16beStr<'r2>> for LazyUtf16beStr<'_> {
    fn eq(&self, other: &Utf16beStr<'r2>) -> bool {
        self.chars()
            .eq(other.chars().map(|r| r.unwrap_or(REPLACEMENT_CHARACTER)))
    }
}

impl<'r2> cmp::PartialEq<LazyUtf16beStr<'r2>> for LazyUtf16beStr<'_> {
    fn eq(&self, other: &LazyUtf16beStr<'r2>) -> bool {
        self.chars().eq(other.chars())
    }
}

impl<'r> Readable<'r> for LazyUtf16beStr<'r> {
    type Args = ();
    fn read_from(reader: &mut Reader<'r>, (): ()) -> Self {
        let s = reader.read(());
        LazyUtf16beStr::Borrowed(s)
    }

    fn size(&self) -> usize {
        match *self {
            LazyUtf16beStr::Owned(ref s) => s.chars().map(|c| c.len_utf16()).sum::<usize>() * 2,
            LazyUtf16beStr::Borrowed(ref s) => s.size(),
        }
    }
}

impl Writable for LazyUtf16beStr<'_> {
    fn write_to<W: io::Write>(&self, w: &mut W) -> io::Result<u64> {
        match *self {
            LazyUtf16beStr::Borrowed(ref s) => {
                w.write_all(&s.0)?;
                Ok(s.0[..].len() as u64)
            }
            LazyUtf16beStr::Owned(ref s) => {
                let mut sum = 0;
                for i in s.encode_utf16() {
                    sum += i.write_to(w)?
                }
                Ok(sum)
            }
        }
    }
}

#[derive(Clone)]
pub enum LazyUtf16beStrChars<'r, 's> {
    Owned(Chars<'s>),
    Borrowed(DecodeUtf16<U16beIter<'r>>),
}

impl Iterator for LazyUtf16beStrChars<'_, '_> {
    type Item = char;
    fn next(&mut self) -> Option<Self::Item> {
        match *self {
            LazyUtf16beStrChars::Owned(ref mut c) => c.next(),
            LazyUtf16beStrChars::Borrowed(ref mut c) => {
                c.next().map(|r| r.unwrap_or(REPLACEMENT_CHARACTER))
            }
        }
    }
}

impl From<String> for LazyUtf16beStr<'_> {
    fn from(s: String) -> Self {
        // Verify null-terminator
        assert!(s.ends_with('\u{0}'));
        LazyUtf16beStr::Owned(s)
    }
}
