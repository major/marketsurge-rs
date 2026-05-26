fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/cli/args.rs");

    if std::env::var_os("CARGO_FEATURE_CLI").is_some() {
        use std::io::Write;

        let cmd = clap::Command::new("marketsurge-agent")
            .about("Query MarketSurge data as compact JSON")
            .version(env!("CARGO_PKG_VERSION"))
            .long_about("Query MarketSurge data as compact JSON. Auth reads browser cookies, so log in at https://marketsurge.investors.com first. Use --fields to limit top-level JSON fields in command output.")
            .after_help("Examples:\n  marketsurge-agent ratings AAPL MSFT\n  marketsurge-agent --fields symbol,rs_rating ratings AAPL\n  marketsurge-agent completions zsh > _marketsurge-agent")
            .arg(
                clap::Arg::new("fields")
                    .long("fields")
                    .global(true)
                    .value_delimiter(',')
                    .value_name("FIELD")
                    .help("Comma-delimited top-level JSON fields to include in output."),
            )
            .subcommand(clap::Command::new("adhoc-screen").about("Run an ad-hoc screener query and return matching rows"))
            .subcommand(clap::Command::new("chart").about("Fetch daily or weekly OHLCV bars for symbols"))
            .subcommand(clap::Command::new("fundamentals").about("Fetch EPS, sales, and estimate fundamentals for symbols"))
            .subcommand(clap::Command::new("industry").about("Fetch industry group RS and overview data"))
            .subcommand(clap::Command::new("market-data").about("Fetch broad rating, price, industry, and fundamental snapshot data"))
            .subcommand(clap::Command::new("ownership").about("Fetch fund ownership summaries and fund holdings"))
            .subcommand(clap::Command::new("ratings").about("Fetch relative strength ratings for symbols"))
            .subcommand(clap::Command::new("screen").about("List or run stock screens, including coach screens"))
            .subcommand(clap::Command::new("tree").about("Fetch coach or navigation trees"))
            .subcommand(clap::Command::new("watchlist").about("List watchlists, read symbols, or screen symbols"))
            .subcommand(clap::Command::new("completions").about("Generate shell completion scripts"))
            .subcommand(clap::Command::new("schema").about("Dump the CLI surface as machine-readable JSON"));

        let man = clap_mangen::Man::new(cmd);

        let mut buf = vec![];
        man.render(&mut buf).expect("failed to render man page");

        let out_dir = std::path::PathBuf::from(
            std::env::var_os("OUT_DIR").expect("OUT_DIR is set by Cargo for build scripts"),
        )
        .join("man");
        std::fs::create_dir_all(&out_dir).expect("failed to create man/ directory");

        let path = out_dir.join("marketsurge-agent.1");
        let mut file = std::fs::File::create(&path).expect("failed to create man page file");
        file.write_all(&buf).expect("failed to write man page");
    }
}
