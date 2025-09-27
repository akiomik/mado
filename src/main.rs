#[cfg(all(
    not(target_os = "windows"),
    not(target_os = "openbsd"),
    not(target_os = "aix"),
    any(
        target_arch = "x86_64",
        target_arch = "aarch64",
        target_arch = "powerpc64"
    )
))]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

#[cfg(target_os = "windows")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use std::process::ExitCode;

use clap::CommandFactory as _;
use clap::Parser as _;
use mado::command::check::Options;
use miette::Result;

use mado::command::check::Checker;
use mado::command::generate_shell_completion::ShellCompletionGenerator;
use mado::command::CompletionShell;
use mado::Cli;
use mado::Command;

fn main() -> Result<ExitCode> {
    let cli = Cli::parse();

    match &cli.command {
        Command::Check {
            files,
            output_format,
            quiet,
            exclude,
        } => {
            let options = Options {
                output_format: output_format.clone(),
                config_path: cli.config,
                quiet: *quiet,
                exclude: exclude.clone(),
            };
            let config = options.to_config()?;
            let checker = Checker::new(files, config)?;
            checker.check()
        }
        Command::GenerateShellCompletion { shell } => {
            let cmd = Cli::command();
            let mut generator = ShellCompletionGenerator::new(cmd);
            match shell {
                CompletionShell::Bash => {
                    generator.generate(clap_complete::Shell::Bash);
                }
                CompletionShell::Elvish => {
                    generator.generate(clap_complete::Shell::Elvish);
                }
                CompletionShell::Fish => {
                    generator.generate(clap_complete::Shell::Fish);
                }
                CompletionShell::Powershell => {
                    generator.generate(clap_complete::Shell::PowerShell);
                }
                CompletionShell::Zsh => {
                    generator.generate(clap_complete::Shell::Zsh);
                }
                CompletionShell::Nushell => {
                    generator.generate(clap_complete_nushell::Nushell);
                }
                CompletionShell::Fig => {
                    generator.generate(clap_complete_fig::Fig);
                }
            }
            Ok(ExitCode::SUCCESS)
        }
    }
}
