use std::io;

use clap::Command;
use clap_complete::{Generator, generate};

#[derive(Debug, Clone)]
pub struct ShellCompletionGenerator {
    cmd: Command,
}

impl ShellCompletionGenerator {
    #[inline]
    #[must_use]
    pub const fn new(cmd: Command) -> Self {
        Self { cmd }
    }

    #[inline]
    pub fn generate<G: Generator>(&mut self, generator: G) {
        let name = self.cmd.get_name().to_owned();
        generate(generator, &mut self.cmd, name, &mut io::stdout());
    }
}
