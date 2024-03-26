pub mod alloc;

use clap::Parser;

#[cfg(feature = "terminal")]
pub mod inter;
#[cfg(feature = "terminal")]
pub mod store;

mod args;
mod daemon;
mod parse;
mod update;
mod utils;

fn main() -> anyhow::Result<()> {
    let opt = args::cmd::Opt::parse();

    #[cfg(all(feature = "serve", not(feature = "terminal")))]
    if let Some(command) = opt.command {
        match command {
            args::ServeSubcommand::Run(args) => daemon::serve(args, false)?,
            #[cfg(target_family = "unix")]
            args::ServeSubcommand::Stop => daemon::serve_stop()?,
            #[cfg(target_family = "unix")]
            args::ServeSubcommand::Start(args) => daemon::serve_start(args)?,
            #[cfg(target_family = "unix")]
            args::ServeSubcommand::Restart(args) => daemon::serve_restart(args)?,
            #[cfg(target_family = "unix")]
            args::ServeSubcommand::Status => daemon::serve_status()?,
            #[cfg(target_family = "unix")]
            args::ServeSubcommand::Log => daemon::serve_log()?,
            args::ServeSubcommand::Genca => {
                let _ = mitm::cagen::gen_ca();
            }
            args::ServeSubcommand::UA => print_ua_help(),
            args::ServeSubcommand::GT { out } => daemon::generate_template(out)?,
            args::ServeSubcommand::Update => update::update()?,
        }
    }

    #[cfg(all(feature = "serve", feature = "terminal"))]
    if let Some(command) = opt.command {
        use args::cmd::SubCommands;
        match command {
            SubCommands::Serve(commands) => match commands {
                args::ServeSubcommand::Run(args) => daemon::serve(args, true)?,
                #[cfg(target_family = "unix")]
                args::ServeSubcommand::Stop => daemon::serve_stop()?,
                #[cfg(target_family = "unix")]
                args::ServeSubcommand::Start(args) => daemon::serve_start(args)?,
                #[cfg(target_family = "unix")]
                args::ServeSubcommand::Restart(args) => daemon::serve_restart(args)?,
                #[cfg(target_family = "unix")]
                args::ServeSubcommand::Status => daemon::serve_status()?,
                #[cfg(target_family = "unix")]
                args::ServeSubcommand::Log => daemon::serve_log()?,
                args::ServeSubcommand::Genca => {
                    let _ = openai::serve::preauth::cagen::gen_ca();
                }
                args::ServeSubcommand::UA => print_ua_help(),
                args::ServeSubcommand::GT { out } => daemon::generate_template(out)?,
                args::ServeSubcommand::Update => update::update()?,
            },
            SubCommands::Terminal => {
                let runtime = tokio::runtime::Builder::new_multi_thread()
                    .enable_all()
                    .worker_threads(1)
                    .max_blocking_threads(1)
                    .build()?;

                runtime.block_on(inter::prompt())?;
            }
        }
    }

    Ok(())
}

use reqwest::impersonate::Impersonate;

struct AgentImpersonate(Impersonate);

impl std::fmt::Debug for AgentImpersonate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match &self.0 {
            Impersonate::Chrome99 => "chrome99",
            Impersonate::Chrome100 => "chrome100",
            Impersonate::Chrome101 => "chrome101",
            Impersonate::Chrome104 => "chrome104",
            Impersonate::Chrome105 => "chrome105",
            Impersonate::Chrome106 => "chrome106",
            Impersonate::Chrome108 => "chrome108",
            Impersonate::Chrome107 => "chrome107",
            Impersonate::Chrome109 => "chrome109",
            Impersonate::Chrome114 => "chrome114",
            Impersonate::Chrome116 => "chrome116",
            Impersonate::Chrome117 => "chrome117",
            Impersonate::Chrome118 => "chrome118",
            Impersonate::Chrome119 => "chrome119",
            Impersonate::Chrome120 => "chrome120",
            Impersonate::Safari12 => "safari12",
            Impersonate::Safari15_3 => "safari15_3",
            Impersonate::Safari15_5 => "safari15_5",
            Impersonate::Safari15_6_1 => "safari15_6_1",
            Impersonate::Safari16 => "safari16",
            Impersonate::Safari16_5 => "safari16_5",
            Impersonate::Safari17_2_1 => "safari17_2_1",
            Impersonate::OkHttp3_9 => "okhttp3_9",
            Impersonate::OkHttp3_11 => "okhttp3_11",
            Impersonate::OkHttp3_13 => "okhttp3_13",
            Impersonate::OkHttp3_14 => "okhttp3_14",
            Impersonate::OkHttp4_9 => "okhttp4_9",
            Impersonate::OkHttp4_10 => "okhttp4_10",
            Impersonate::OkHttp5 => "okhttp5",
            Impersonate::Edge99 => "edge99",
            Impersonate::Edge101 => "edge101",
            Impersonate::Edge122 => "edge122",
        };
        f.write_str(name)
    }
}

/// Print impersonate user agent support help
fn print_ua_help() {
    // Edge user agent group
    let edge_group = [
        AgentImpersonate(Impersonate::Edge99),
        AgentImpersonate(Impersonate::Edge101),
        AgentImpersonate(Impersonate::Edge122),
    ]
    .into_iter()
    .map(|x| format!("{:?}", x))
    .collect::<Vec<_>>();
    println!("Edge: {}", edge_group.join(", "));

    // Safari user agent group
    let safari_group = [
        AgentImpersonate(Impersonate::Safari12),
        AgentImpersonate(Impersonate::Safari15_3),
        AgentImpersonate(Impersonate::Safari15_5),
        AgentImpersonate(Impersonate::Safari15_6_1),
        AgentImpersonate(Impersonate::Safari16),
        AgentImpersonate(Impersonate::Safari16_5),
        AgentImpersonate(Impersonate::Safari17_2_1),
    ]
    .into_iter()
    .map(|x| format!("{:?}", x))
    .collect::<Vec<_>>();
    println!("Safari: {}", safari_group.join(","));

    // OkHttp user agent group
    let okhttp_group = [
        AgentImpersonate(Impersonate::OkHttp3_9),
        AgentImpersonate(Impersonate::OkHttp3_11),
        AgentImpersonate(Impersonate::OkHttp3_13),
        AgentImpersonate(Impersonate::OkHttp3_14),
        AgentImpersonate(Impersonate::OkHttp4_9),
        AgentImpersonate(Impersonate::OkHttp4_10),
        AgentImpersonate(Impersonate::OkHttp5),
    ]
    .into_iter()
    .map(|x| format!("{:?}", x))
    .collect::<Vec<_>>();
    println!("OkHttp: {}", okhttp_group.join(","));

    // Chrome user agent group
    let chrome_group = [
        AgentImpersonate(Impersonate::Chrome99),
        AgentImpersonate(Impersonate::Chrome100),
        AgentImpersonate(Impersonate::Chrome101),
        AgentImpersonate(Impersonate::Chrome104),
        AgentImpersonate(Impersonate::Chrome105),
        AgentImpersonate(Impersonate::Chrome106),
        AgentImpersonate(Impersonate::Chrome107),
        AgentImpersonate(Impersonate::Chrome108),
        AgentImpersonate(Impersonate::Chrome109),
        AgentImpersonate(Impersonate::Chrome114),
        AgentImpersonate(Impersonate::Chrome116),
        AgentImpersonate(Impersonate::Chrome117),
        AgentImpersonate(Impersonate::Chrome118),
        AgentImpersonate(Impersonate::Chrome119),
        AgentImpersonate(Impersonate::Chrome120),
    ]
    .into_iter()
    .map(|x| format!("{:?}", x))
    .collect::<Vec<_>>();
    println!("Chrome: {}", chrome_group.join(","));
}
