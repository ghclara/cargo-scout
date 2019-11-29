use structopt::StructOpt;

mod clippy;
mod error;
mod git;
mod intersections;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "cargo-scout",
    author,
    about = "Leave the codebase better than when you found it."
)]
struct Options {
    #[structopt(short = "v", long = "verbose")]
    /// Set the verbosity level
    verbose: bool,

    #[structopt(
        short = "b",
        long = "branch",
        value_name = "branch",
        default_value = "master"
    )]
    /// Set the target branch
    branch: String,
}

fn display_warnings(warnings: &[clippy::Lint]) {
    for w in warnings {
        if let Some(m) = &w.message {
            for l in m.rendered.split('\n') {
                println!("{}", l);
            }
        }
    }
}

fn main() -> Result<(), error::Error> {
    let opts = Options::from_args();

    println!("Getting diff against target {}", opts.branch);
    let diff_sections = git::Parser::new()
        .set_verbose(opts.verbose)
        .get_sections(&opts.branch)?;
    println!("Running clippy");
    let clippy_lints = clippy::Linter::new()
        .set_verbose(opts.verbose)
        .get_lints()?;

    let warnings_caused_by_diff =
        intersections::get_lints_from_diff(&clippy_lints, &diff_sections, opts.verbose);
    if warnings_caused_by_diff.is_empty() {
        println!("No warnings raised by clippy::pedantic in your diff, you're good to go!");
        Ok(())
    } else {
        display_warnings(&warnings_caused_by_diff);
        println!(
            "Clippy::pedantic found {} warnings",
            warnings_caused_by_diff.len()
        );
        Err(error::Error::NotClean)
    }
}
