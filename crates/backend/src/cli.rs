use clap::Parser;

#[derive(Parser, Debug)]
#[clap(version, about, long_about = None)]
pub struct CliArgs {
    #[clap(long, value_parser, default_value_t = 8084)]
    pub port: usize,

    #[clap(long, value_parser, default_value_t = default_host())]
    pub host: String,
}

fn default_host() -> String {
    String::from("127.0.0.1")
}
