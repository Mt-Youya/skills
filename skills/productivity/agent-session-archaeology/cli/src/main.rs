mod cli;
mod guide;
mod sessions;
mod storage;

fn main() -> std::process::ExitCode {
    cli::main()
}
