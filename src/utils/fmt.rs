use std::{cmp, io};
use std::{fmt::Display, time::Duration};

use size::Size;

use super::terminal::terminal_width;


pub trait Formattable: Display {
    const MAX_WIDTH: usize;

    fn left_pad(&self) -> String {
        format!("{:>width$}", self.to_string(), width = Self::MAX_WIDTH)
    }

    fn right_pad(&self) -> String {
        format!("{:<width$}", self.to_string(), width = Self::MAX_WIDTH)
    }

    fn bracketed(self) -> FmtBracketed<Self> where Self: Sized {
        FmtBracketed::new(self)
    }

    fn with_prefix<const PREF_LEN: usize>(self, prefix: String) -> FmtPrefix<PREF_LEN, Self> where Self: Sized {
        FmtPrefix::new(self, prefix)
    }

    fn with_suffix<const SUFF_LEN: usize>(self, suffix: String) -> FmtSuffix<SUFF_LEN, Self> where Self: Sized {
        FmtSuffix::new(self, suffix)
    }
}



pub struct FmtSize(Size);
pub struct FmtPercentage(u64);
pub struct FmtBracketed<T: Formattable>(Box<T>, [char; 2]);
pub struct FmtOrNA<T: Formattable>(Option<T>, bool);
pub struct FmtAge(Duration);
pub struct FmtWithEllipsis(String, usize, bool);
pub struct FmtPrefix<const ADD: usize, T: Formattable>(Box<T>, String);
pub struct FmtSuffix<const ADD: usize, T: Formattable>(Box<T>, String);


impl FmtSize {
    pub fn new(bytes: u64) -> Self {
        FmtSize(Size::from_bytes(bytes))
    }
}

impl FmtPercentage {
    pub fn new(amount: u64, total: u64) -> Self {
        FmtPercentage(amount * 100 / total)
    }
}

impl FmtWithEllipsis {
    pub fn fitting_terminal(s: String, preferred_width: usize, leave_space: usize) -> Self {
        let actual_width = match terminal_width(io::stdout()).ok() {
            Some(tw) => cmp::min(tw.saturating_sub(leave_space), preferred_width),
            None => preferred_width,
        };
        FmtWithEllipsis(s, actual_width, true)
    }

    pub fn truncate_if(mut self, trunc: bool) -> Self {
        self.2 = trunc;
        self
    }

    pub fn right_pad(&self) -> String {
        format!("{:<width$}", self.to_string(), width = self.1)
    }
}

impl<T: Formattable> FmtBracketed<T> {
    pub fn new(obj: T) -> Self {
        FmtBracketed(Box::new(obj), ['(', ')'])
    }

    pub fn with_square_brackets(mut self) -> Self {
        self.1 = ['[', ']'];
        self
    }
}

impl<T: Formattable> FmtOrNA<T> {
    pub fn mapped<S>(option: Option<S>, fun: impl Fn(S) -> T) -> Self {
        match option {
            Some(val) => Self::with(fun(val)),
            None => Self::na(),
        }
    }

    pub fn with(obj: T) -> Self {
        FmtOrNA(Some(obj), true)
    }

    pub fn na() -> Self {
        FmtOrNA(None, true)
    }

    pub fn or_empty(mut self) -> Self {
        self.1 = false;
        self
    }
}

impl FmtAge {
    pub fn new(age: Duration) -> Self {
        FmtAge(age)
    }
}

impl<const ADD: usize, T: Formattable> FmtPrefix<ADD, T> {
    pub fn new(obj: T, prefix: String) -> Self {
        FmtPrefix(Box::new(obj), prefix)
    }
}

impl<const ADD: usize, T: Formattable> FmtSuffix<ADD, T> {
    pub fn new(obj: T, suffix: String) -> Self {
        FmtSuffix(Box::new(obj), suffix)
    }
}



impl Formattable for FmtSize {
    const MAX_WIDTH: usize = 11;
}

impl Formattable for FmtPercentage {
    const MAX_WIDTH: usize = 3;
}

impl<T: Formattable> Formattable for FmtBracketed<T> {
    const MAX_WIDTH: usize = T::MAX_WIDTH + 2;
}

impl<T: Formattable> Formattable for FmtOrNA<T> {
    const MAX_WIDTH: usize = [3, T::MAX_WIDTH][(3 < T::MAX_WIDTH) as usize];
}

impl Formattable for FmtAge {
    const MAX_WIDTH: usize = 9;
}

impl<const ADD: usize, T: Formattable> Formattable for FmtPrefix<ADD, T> {
    const MAX_WIDTH: usize = T::MAX_WIDTH + ADD;
}

impl<const ADD: usize, T: Formattable> Formattable for FmtSuffix<ADD, T> {
    const MAX_WIDTH: usize = T::MAX_WIDTH + ADD;
}



impl Display for FmtSize {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl Display for FmtPercentage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}%", self.0)
    }
}

impl<T: Formattable> Display for FmtBracketed<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}{}", self.1[0], self.0, self.1[1])
    }
}

impl<T: Formattable> Display for FmtOrNA<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.0 {
            Some(val) => write!(f, "{val}"),
            None => write!(f, "{}", if self.1 { "n/a" } else { "" }),
        }
    }
}

impl Display for FmtAge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let seconds = self.0.as_secs();
        let minutes = seconds / 60;
        let hours = minutes / 60;
        let days = hours / 24;
        let weeks = days / 7;
        let years = days / 365;

        if minutes < 1 {
            write!(f, "{seconds} sec")
        } else if hours < 1 {
            write!(f, "{minutes} min")
        } else if days < 1 {
            if hours == 1 {
                write!(f, "1 hour")
            } else {
                write!(f, "{hours} hours")
            }
        } else if years < 1 {
            if days == 1 {
                write!(f, "1 day")
            } else {
                write!(f, "{days} days")
            }
        } else if years < 3 {
            if weeks == 1 {
                write!(f, "1 week")
            } else {
                write!(f, "{weeks} weeks")
            }
        } else if years == 1 {
            write!(f, "1 year")
        } else {
            write!(f, "{years} years")
        }

    }
}

impl<const ADD: usize, T: Formattable> Display for FmtPrefix<ADD, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", self.1, self.0)
    }
}

impl<const ADD: usize, T: Formattable> Display for FmtSuffix<ADD, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", self.0, self.1)
    }
}

impl Display for FmtWithEllipsis {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let FmtWithEllipsis(s, width, trunc) = self;
        let s = if *trunc && s.len() > *width {
            format!("{}...", &s[..width.saturating_sub(3)])
        } else {
            s.to_owned()
        };

        write!(f, "{s}")
    }
}
