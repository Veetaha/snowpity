// This was inspired by `censor` crate
// https://github.com/kaikalii/censor/blob/3fc7d5ae2b707cb621a58c48fbff064223890e6b/src/lib.rs#L1

mod error;
mod template_phrase;

use itertools::Itertools;
use once_cell::unsync::OnceCell as UnsyncOnceCell;
use std::collections::{HashMap, HashSet};

pub use error::*;
pub use template_phrase::*;

fn _aliases<R>(read: impl FnOnce(&HashMap<&str, Vec<&str>>) -> R) -> R {
    thread_local! {
        static ALIASES: UnsyncOnceCell<HashMap<&'static str, Vec<&'static str>>> = UnsyncOnceCell::new();
    }

    ALIASES.with(|aliases| {
        let aliases = aliases.get_or_init(|| {
            include_str!("aliases.csv")
                .split_terminator('\n')
                .filter(|line| !line.is_empty())
                .enumerate()
                .map(|(i, line)| {
                    let i = i + 1;
                    let mut symbols = line.split_terminator(' ');
                    let char = symbols.next().unwrap_or_else(|| {
                        panic!("BUG: in aliases.csv:{i} doesn't have the root character");
                    });

                    let _char = char.chars().exactly_one().unwrap_or_else(|_| {
                        panic!("BUG: line aliases.csv:{i} has root symbol with more than one character");
                    });

                    ("", vec![])
                })
                .collect::<HashMap<&'static str, Vec<&'static str>>>()
        });

        read(aliases)
    });

    todo!()
}

pub struct ValidationCtx {
    _include_banned_phrases: HashSet<TemplatePhrase>,
}

pub struct ValidationInput<'a> {
    _text: &'a str,
}

pub struct ValidationOutput {
    _banned_phrases: Vec<TemplatePhrase>,
}

pub fn validate(_input: ValidationInput<'_>) -> ValidationOutput {
    todo!()
}
