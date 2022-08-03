use clap::Parser;

#[derive(Parser, Debug)]
#[clap(version, about, long_about = None)]
pub struct CliArgs {
    #[clap(short, long, value_parser, default_value_t = 8084)]
    pub port: usize,
}