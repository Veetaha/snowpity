use crate::util::{fs, repo_abs_path};
use clap::Parser;
use itertools::Itertools;

/// Deduplicated aliases.csv file
#[derive(Parser, Debug)]
pub struct FmtAliases {}

impl crate::cmd::Cmd for FmtAliases {
    fn run(self) -> anyhow::Result<()> {
        let file = fs::read_to_string(repo_abs_path(["censy", "src", "aliases.csv"]))?;

        let aliases = file
            .split_terminator("\n")
            .filter(|line| !line.is_empty())
            .map(|line| {
                line.split_terminator(' ')
                    .map(|alias| alias.to_lowercase())
                    .unique()
                    .sorted_by(|a, b| {
                        // Make single-character aliases appear first
                        // Then make cyrillic characters appear first
                        use std::cmp::Ordering::*;

                        let Ok(a) = a.chars().exactly_one() else {
                            return if b.chars().count() == 1 {
                                Greater
                            } else {
                                a.cmp(b)
                            }
                        };

                        let Ok(b) = b.chars().exactly_one() else {
                            return Less
                        };

                        if is_cyrillic(a) {
                            if is_cyrillic(b) {
                                return a.cmp(&b);
                            }
                            return Less;
                        }

                        if is_cyrillic(b) {
                            return Greater;
                        }

                        a.cmp(&b)
                    })
                    .format(" ")
            })
            .join("\n");

        fs::write(
            repo_abs_path(["censy", "src", "aliases-formatted.csv"]),
            aliases,
        )?;

        Ok(())
    }
}

fn is_cyrillic(suspect: char) -> bool {
    ('а'..='я').contains(&suspect)
}
