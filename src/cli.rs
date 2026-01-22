use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand, ValueEnum};

use crate::bin_resolver::{self, ResolveCtx};
use crate::config;
use crate::demo::{self, BuildOptions};
use crate::dev_mode::{
    DevCliOverrides, DevMode, DevProfile, DevSettingsResolved, effective_dev_settings,
    merge_settings,
};
use crate::domains::{self, Domain, DomainAction};
use crate::gmap::{self, Policy};
use crate::project::{self, ScanFormat};
use crate::runner_exec;
use crate::settings;

mod dev_mode_cmd;
mod dev_secrets;

use dev_mode_cmd::{
    DevModeDetectArgs, DevModeMapCommand, DevModeOffArgs, DevModeOnArgs, DevModeStatusArgs,
};
use dev_secrets::DevSecretsCommand;
#[derive(Parser)]
#[command(name = "greentic-operator")]
#[command(about = "Greentic operator tooling", version)]
pub struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Dev(DevCommand),
    Demo(DemoCommand),
}

#[derive(Parser)]
struct DevCommand {
    #[command(subcommand)]
    command: DevSubcommand,
}

#[derive(Parser)]
struct DemoCommand {
    #[arg(long, global = true)]
    debug: bool,
    #[command(subcommand)]
    command: DemoSubcommand,
}

#[derive(Subcommand)]
enum DevSubcommand {
    On(DevModeOnArgs),
    Off(DevModeOffArgs),
    Status(DevModeStatusArgs),
    Detect(DevModeDetectArgs),
    Map(DevModeMapCommand),
    Init(DevInitArgs),
    Scan(DevScanArgs),
    Sync(DevSyncArgs),
    Tenant(TenantCommand),
    Team(TeamCommand),
    Allow(DevPolicyArgs),
    Forbid(DevPolicyArgs),
    Up(DevUpArgs),
    Down(DevDownArgs),
    #[command(name = "svc-status")]
    SvcStatus(DevStatusArgs),
    Logs(DevLogsArgs),
    Setup(DomainSetupArgs),
    Diagnostics(DomainDiagnosticsArgs),
    Verify(DomainVerifyArgs),
    Doctor(DevDoctorArgs),
    Secrets(DevSecretsCommand),
}

#[derive(Subcommand)]
enum DemoSubcommand {
    Build(DemoBuildArgs),
    Up(DemoUpArgs),
    Setup(DemoSetupArgs),
    Down(DemoDownArgs),
    Status(DemoStatusArgs),
    Logs(DemoLogsArgs),
    Doctor(DemoDoctorArgs),
}

#[derive(Parser)]
#[command(
    about = "Initialize a new Greentic project layout.",
    long_about = "Creates the standard project directory layout, greentic.yaml, and the default tenant gmap.",
    after_help = "Main options:\n  (none)\n\nOptional options:\n  --project-root <PATH> (default: current directory)"
)]
struct DevInitArgs {
    #[arg(long)]
    project_root: Option<PathBuf>,
}

#[derive(Parser)]
#[command(
    about = "Scan providers, packs, tenants, and teams.",
    long_about = "Scans the project layout and prints a summary or structured output.",
    after_help = "Main options:\n  (none)\n\nOptional options:\n  --format <text|json|yaml> (default: text)\n  --project-root <PATH> (default: current directory)"
)]
struct DevScanArgs {
    #[arg(long, value_enum, default_value_t = Format::Text)]
    format: Format,
    #[arg(long)]
    project_root: Option<PathBuf>,
}

#[derive(Parser)]
#[command(
    about = "Generate resolved manifests for tenants and teams.",
    long_about = "Writes state/resolved/<tenant>[.<team>].yaml from discovered packs, providers, and gmaps.",
    after_help = "Main options:\n  (none)\n\nOptional options:\n  --project-root <PATH> (default: current directory)"
)]
struct DevSyncArgs {
    #[arg(long)]
    project_root: Option<PathBuf>,
}

#[derive(Parser)]
struct TenantCommand {
    #[command(subcommand)]
    command: TenantSubcommand,
}

#[derive(Subcommand)]
enum TenantSubcommand {
    Add(TenantAddArgs),
    Rm(TenantRmArgs),
    List(TenantListArgs),
}

#[derive(Parser)]
#[command(
    about = "Add a tenant and initialize its gmap.",
    long_about = "Creates tenants/<tenant>/tenant.gmap with a default policy.",
    after_help = "Main options:\n  <TENANT>\n\nOptional options:\n  --project-root <PATH> (default: current directory)"
)]
struct TenantAddArgs {
    tenant: String,
    #[arg(long)]
    project_root: Option<PathBuf>,
}

#[derive(Parser)]
#[command(
    about = "Remove a tenant and its data.",
    long_about = "Deletes tenants/<tenant> and any team gmaps under it.",
    after_help = "Main options:\n  <TENANT>\n\nOptional options:\n  --project-root <PATH> (default: current directory)"
)]
struct TenantRmArgs {
    tenant: String,
    #[arg(long)]
    project_root: Option<PathBuf>,
}

#[derive(Parser)]
#[command(
    about = "List tenants in the project.",
    long_about = "Reads tenants/ and prints tenant ids.",
    after_help = "Main options:\n  (none)\n\nOptional options:\n  --project-root <PATH> (default: current directory)"
)]
struct TenantListArgs {
    #[arg(long)]
    project_root: Option<PathBuf>,
}

#[derive(Parser)]
struct TeamCommand {
    #[command(subcommand)]
    command: TeamSubcommand,
}

#[derive(Subcommand)]
enum TeamSubcommand {
    Add(TeamAddArgs),
    Rm(TeamRmArgs),
    List(TeamListArgs),
}

#[derive(Parser)]
#[command(
    about = "Add a team under a tenant and initialize its gmap.",
    long_about = "Creates tenants/<tenant>/teams/<team>/team.gmap with a default policy.",
    after_help = "Main options:\n  <TEAM>\n  --tenant <TENANT>\n\nOptional options:\n  --project-root <PATH> (default: current directory)"
)]
struct TeamAddArgs {
    #[arg(long)]
    tenant: String,
    team: String,
    #[arg(long)]
    project_root: Option<PathBuf>,
}

#[derive(Parser)]
#[command(
    about = "Remove a team from a tenant.",
    long_about = "Deletes tenants/<tenant>/teams/<team> and its gmap.",
    after_help = "Main options:\n  <TEAM>\n  --tenant <TENANT>\n\nOptional options:\n  --project-root <PATH> (default: current directory)"
)]
struct TeamRmArgs {
    #[arg(long)]
    tenant: String,
    team: String,
    #[arg(long)]
    project_root: Option<PathBuf>,
}

#[derive(Parser)]
#[command(
    about = "List teams for a tenant.",
    long_about = "Reads tenants/<tenant>/teams and prints team ids.",
    after_help = "Main options:\n  --tenant <TENANT>\n\nOptional options:\n  --project-root <PATH> (default: current directory)"
)]
struct TeamListArgs {
    #[arg(long)]
    tenant: String,
    #[arg(long)]
    project_root: Option<PathBuf>,
}

#[derive(Parser)]
#[command(
    about = "Start local messaging services (and NATS unless disabled).",
    long_about = "Uses state/resolved/<tenant>[.<team>].yaml and launches greentic-messaging and optional NATS.",
    after_help = "Main options:\n  --tenant <TENANT>\n\nOptional options:\n  --team <TEAM>\n  --no-nats\n  --nats-url <URL>\n  --project-root <PATH> (default: current directory)\n  --dev-mode <auto|on|off>\n  --dev-root <PATH>\n  --dev-profile <debug|release>\n  --dev-target-dir <PATH>"
)]
struct DevUpArgs {
    #[arg(long)]
    tenant: String,
    #[arg(long)]
    team: Option<String>,
    #[arg(long)]
    no_nats: bool,
    #[arg(long)]
    nats_url: Option<String>,
    #[arg(long)]
    project_root: Option<PathBuf>,
    #[command(flatten)]
    dev: DevModeArgs,
}

#[derive(Parser)]
#[command(
    about = "Stop local messaging services (and NATS unless disabled).",
    long_about = "Stops services started by dev up and removes pidfiles.",
    after_help = "Main options:\n  --tenant <TENANT>\n\nOptional options:\n  --team <TEAM>\n  --no-nats\n  --project-root <PATH> (default: current directory)"
)]
struct DevDownArgs {
    #[arg(long)]
    tenant: String,
    #[arg(long)]
    team: Option<String>,
    #[arg(long)]
    no_nats: bool,
    #[arg(long)]
    project_root: Option<PathBuf>,
}

#[derive(Parser)]
#[command(
    about = "Show running status for local messaging services.",
    long_about = "Checks pidfiles for messaging and optional NATS.",
    after_help = "Main options:\n  --tenant <TENANT>\n\nOptional options:\n  --team <TEAM>\n  --no-nats\n  --project-root <PATH> (default: current directory)"
)]
struct DevStatusArgs {
    #[arg(long)]
    tenant: String,
    #[arg(long)]
    team: Option<String>,
    #[arg(long)]
    no_nats: bool,
    #[arg(long)]
    project_root: Option<PathBuf>,
}

#[derive(Parser)]
#[command(
    about = "Tail logs for messaging or NATS.",
    long_about = "Streams the latest log output for the selected service.",
    after_help = "Main options:\n  --tenant <TENANT>\n\nOptional options:\n  --team <TEAM>\n  --service <messaging|nats>\n  --project-root <PATH> (default: current directory)"
)]
struct DevLogsArgs {
    #[arg(long)]
    tenant: String,
    #[arg(long)]
    team: Option<String>,
    #[arg(long)]
    service: Option<LogService>,
    #[arg(long)]
    project_root: Option<PathBuf>,
}

#[derive(Parser)]
#[command(
    about = "Run provider setup flows for a domain.",
    long_about = "Executes setup_default across providers and can auto-run secrets init for messaging.",
    after_help = "Main options:\n  <DOMAIN>\n  --tenant <TENANT>\n\nOptional options:\n  --team <TEAM>\n  --provider <FILTER>\n  --dry-run\n  --format <text|json|yaml> (default: text)\n  --parallel <N> (default: 1)\n  --allow-missing-setup\n  --online\n  --secrets-env <ENV>\n  --secrets-bin <PATH>\n  --project-root <PATH> (default: current directory)"
)]
struct DomainSetupArgs {
    domain: DomainArg,
    #[arg(long)]
    tenant: String,
    #[arg(long)]
    team: Option<String>,
    #[arg(long)]
    provider: Option<String>,
    #[arg(long)]
    dry_run: bool,
    #[arg(long, value_enum, default_value_t = PlanFormat::Text)]
    format: PlanFormat,
    #[arg(long, default_value_t = 1)]
    parallel: usize,
    #[arg(long)]
    allow_missing_setup: bool,
    #[arg(long)]
    online: bool,
    #[arg(long)]
    secrets_env: Option<String>,
    #[arg(long)]
    secrets_bin: Option<PathBuf>,
    #[arg(long)]
    project_root: Option<PathBuf>,
}

#[derive(Parser)]
#[command(
    about = "Run provider diagnostics flows for a domain.",
    long_about = "Executes diagnostics for each provider pack that defines it.",
    after_help = "Main options:\n  <DOMAIN>\n  --tenant <TENANT>\n\nOptional options:\n  --team <TEAM>\n  --provider <FILTER>\n  --dry-run\n  --format <text|json|yaml> (default: text)\n  --parallel <N> (default: 1)\n  --online\n  --project-root <PATH> (default: current directory)"
)]
struct DomainDiagnosticsArgs {
    domain: DomainArg,
    #[arg(long)]
    tenant: String,
    #[arg(long)]
    team: Option<String>,
    #[arg(long)]
    provider: Option<String>,
    #[arg(long)]
    dry_run: bool,
    #[arg(long, value_enum, default_value_t = PlanFormat::Text)]
    format: PlanFormat,
    #[arg(long, default_value_t = 1)]
    parallel: usize,
    #[arg(long)]
    online: bool,
    #[arg(long)]
    project_root: Option<PathBuf>,
}

#[derive(Parser)]
#[command(
    about = "Run provider verify flows for a domain.",
    long_about = "Executes verify_* flows where available for the selected domain.",
    after_help = "Main options:\n  <DOMAIN>\n  --tenant <TENANT>\n\nOptional options:\n  --team <TEAM>\n  --provider <FILTER>\n  --dry-run\n  --format <text|json|yaml> (default: text)\n  --parallel <N> (default: 1)\n  --online\n  --project-root <PATH> (default: current directory)"
)]
struct DomainVerifyArgs {
    domain: DomainArg,
    #[arg(long)]
    tenant: String,
    #[arg(long)]
    team: Option<String>,
    #[arg(long)]
    provider: Option<String>,
    #[arg(long)]
    dry_run: bool,
    #[arg(long, value_enum, default_value_t = PlanFormat::Text)]
    format: PlanFormat,
    #[arg(long, default_value_t = 1)]
    parallel: usize,
    #[arg(long)]
    online: bool,
    #[arg(long)]
    project_root: Option<PathBuf>,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum DomainArg {
    Messaging,
    Events,
    Secrets,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum DoctorDomainArg {
    Messaging,
    Events,
    Secrets,
    All,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum PlanFormat {
    Text,
    Json,
    Yaml,
}

#[derive(Parser)]
#[command(
    about = "Run domain doctor validation.",
    long_about = "Executes greentic-pack doctor with optional validators.",
    after_help = "Main options:\n  <DOMAIN>\n\nOptional options:\n  --tenant <TENANT>\n  --team <TEAM>\n  --strict\n  --validator-pack <PATH>...\n  --project-root <PATH> (default: current directory)\n  --dev-mode <auto|on|off>\n  --dev-root <PATH>\n  --dev-profile <debug|release>\n  --dev-target-dir <PATH>"
)]
struct DevDoctorArgs {
    domain: DoctorDomainArg,
    #[arg(long)]
    tenant: Option<String>,
    #[arg(long)]
    team: Option<String>,
    #[arg(long)]
    strict: bool,
    #[arg(long)]
    validator_pack: Vec<PathBuf>,
    #[arg(long)]
    project_root: Option<PathBuf>,
    #[command(flatten)]
    dev: DevModeArgs,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum LogService {
    Messaging,
    Nats,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum CloudflaredModeArg {
    On,
    Off,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum RestartTarget {
    All,
    Cloudflared,
    Nats,
    Gateway,
    Egress,
    Subscriptions,
}

#[derive(Parser)]
#[command(
    about = "Build a portable demo bundle.",
    long_about = "Copies packs/providers/tenants and writes resolved manifests under the output directory.",
    after_help = "Main options:\n  --out <DIR>\n\nOptional options:\n  --tenant <TENANT>\n  --team <TEAM>\n  --allow-pack-dirs\n  --only-used-providers\n  --doctor\n  --project-root <PATH> (default: current directory)\n  --dev-mode <auto|on|off>\n  --dev-root <PATH>\n  --dev-profile <debug|release>\n  --dev-target-dir <PATH>"
)]
struct DemoBuildArgs {
    #[arg(long)]
    out: PathBuf,
    #[arg(long)]
    tenant: Option<String>,
    #[arg(long)]
    team: Option<String>,
    #[arg(long)]
    allow_pack_dirs: bool,
    #[arg(long)]
    only_used_providers: bool,
    #[arg(long)]
    doctor: bool,
    #[arg(long)]
    project_root: Option<PathBuf>,
    #[command(flatten)]
    dev: DevModeArgs,
}

#[derive(Parser)]
#[command(
    about = "Start demo services from a bundle.",
    long_about = "Uses resolved manifests inside the bundle to start services and optional NATS.",
    after_help = "Main options:\n  --bundle <DIR> (bundle mode)\n  --config <PATH> (config mode)\n\nOptional options:\n  --tenant <TENANT>\n  --team <TEAM>\n  --no-nats\n  --nats-url <URL>\n  --cloudflared <on|off> (default: on)\n  --cloudflared-binary <PATH>\n  --restart <all|cloudflared|nats|gateway|egress|subscriptions>[,...]\n  --providers <csv>\n  --skip-setup\n  --skip-secrets-init\n  --verify-webhooks\n  --force-setup\n  --setup-input <PATH>\n  --runner-binary <PATH>\n  --dev-mode <auto|on|off>\n  --dev-root <PATH>\n  --dev-profile <debug|release>\n  --dev-target-dir <PATH>"
)]
struct DemoUpArgs {
    #[arg(long)]
    bundle: Option<PathBuf>,
    #[arg(long)]
    tenant: Option<String>,
    #[arg(long)]
    team: Option<String>,
    #[arg(long)]
    no_nats: bool,
    #[arg(long)]
    nats_url: Option<String>,
    #[arg(long)]
    config: Option<PathBuf>,
    #[arg(long, value_enum, default_value_t = CloudflaredModeArg::On)]
    cloudflared: CloudflaredModeArg,
    #[arg(long)]
    cloudflared_binary: Option<PathBuf>,
    #[arg(long, value_enum, value_delimiter = ',')]
    restart: Vec<RestartTarget>,
    #[arg(long, value_delimiter = ',')]
    providers: Vec<String>,
    #[arg(long)]
    skip_setup: bool,
    #[arg(long)]
    skip_secrets_init: bool,
    #[arg(long)]
    verify_webhooks: bool,
    #[arg(long)]
    force_setup: bool,
    #[arg(long)]
    setup_input: Option<PathBuf>,
    #[arg(long)]
    runner_binary: Option<PathBuf>,
    #[command(flatten)]
    dev: DevModeArgs,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum DemoSetupDomainArg {
    Messaging,
    Events,
    Secrets,
    All,
}

#[derive(Parser)]
#[command(
    about = "Run provider setup flows against a demo bundle.",
    long_about = "Executes setup flows for provider packs included in the bundle.",
    after_help = "Main options:\n  --bundle <DIR>\n  --tenant <TENANT>\n\nOptional options:\n  --team <TEAM>\n  --domain <messaging|events|secrets|all> (default: all)\n  --provider <FILTER>\n  --dry-run\n  --format <text|json|yaml> (default: text)\n  --parallel <N> (default: 1)\n  --allow-missing-setup\n  --online\n  --secrets-env <ENV>\n  --secrets-bin <PATH>\n  --skip-secrets-init"
)]
struct DemoSetupArgs {
    #[arg(long)]
    bundle: PathBuf,
    #[arg(long)]
    tenant: String,
    #[arg(long)]
    team: Option<String>,
    #[arg(long, value_enum, default_value_t = DemoSetupDomainArg::All)]
    domain: DemoSetupDomainArg,
    #[arg(long)]
    provider: Option<String>,
    #[arg(long)]
    dry_run: bool,
    #[arg(long, value_enum, default_value_t = Format::Text)]
    format: Format,
    #[arg(long, default_value_t = 1)]
    parallel: usize,
    #[arg(long)]
    allow_missing_setup: bool,
    #[arg(long)]
    online: bool,
    #[arg(long)]
    secrets_env: Option<String>,
    #[arg(long)]
    secrets_bin: Option<PathBuf>,
    #[arg(long)]
    skip_secrets_init: bool,
    #[arg(long)]
    state_dir: Option<PathBuf>,
}
#[derive(Parser)]
#[command(
    about = "Stop demo services using runtime state.",
    long_about = "Stops services recorded in state/pids for a tenant/team or all services when --all is set.",
    after_help = "Main options:\n  (none)\n\nOptional options:\n  --tenant <TENANT> (default: demo)\n  --team <TEAM> (default: default)\n  --state-dir <PATH> (default: ./state or <bundle>/state)\n  --bundle <DIR> (legacy mode if --state-dir omitted)\n  --all\n  --verbose\n  --no-nats"
)]
struct DemoDownArgs {
    #[arg(long)]
    bundle: Option<PathBuf>,
    #[arg(long, default_value = "demo")]
    tenant: String,
    #[arg(long, default_value = "default")]
    team: String,
    #[arg(long)]
    state_dir: Option<PathBuf>,
    #[arg(long)]
    all: bool,
    #[arg(long)]
    verbose: bool,
    #[arg(long)]
    no_nats: bool,
}

#[derive(Parser)]
#[command(
    about = "Show demo service status using runtime state.",
    long_about = "Lists pidfiles under state/pids for the selected tenant/team.",
    after_help = "Main options:\n  (none)\n\nOptional options:\n  --tenant <TENANT> (default: demo)\n  --team <TEAM> (default: default)\n  --state-dir <PATH> (default: ./state or <bundle>/state)\n  --bundle <DIR> (legacy mode if --state-dir omitted)\n  --verbose\n  --no-nats"
)]
struct DemoStatusArgs {
    #[arg(long)]
    bundle: Option<PathBuf>,
    #[arg(long, default_value = "demo")]
    tenant: String,
    #[arg(long, default_value = "default")]
    team: String,
    #[arg(long)]
    state_dir: Option<PathBuf>,
    #[arg(long)]
    verbose: bool,
    #[arg(long)]
    no_nats: bool,
}

#[derive(Parser)]
#[command(
    about = "Show demo service logs using runtime state.",
    long_about = "Prints or tails logs under state/logs for the selected service.",
    after_help = "Main options:\n  <SERVICE> (messaging|nats|cloudflared)\n\nOptional options:\n  --tail\n  --tenant <TENANT> (default: demo)\n  --team <TEAM> (default: default)\n  --state-dir <PATH> (default: ./state or <bundle>/state)\n  --bundle <DIR> (legacy mode if --state-dir omitted)\n  --verbose\n  --no-nats"
)]
struct DemoLogsArgs {
    #[arg(default_value = "messaging")]
    service: String,
    #[arg(long)]
    tail: bool,
    #[arg(long)]
    bundle: Option<PathBuf>,
    #[arg(long, default_value = "demo")]
    tenant: String,
    #[arg(long, default_value = "default")]
    team: String,
    #[arg(long)]
    state_dir: Option<PathBuf>,
    #[arg(long)]
    verbose: bool,
    #[arg(long)]
    no_nats: bool,
}

#[derive(Parser)]
#[command(
    about = "Run demo doctor validation from a bundle.",
    long_about = "Runs greentic-pack doctor against packs in the demo bundle.",
    after_help = "Main options:\n  --bundle <DIR>\n\nOptional options:\n  --dev-mode <auto|on|off>\n  --dev-root <PATH>\n  --dev-profile <debug|release>\n  --dev-target-dir <PATH>"
)]
struct DemoDoctorArgs {
    #[arg(long)]
    bundle: PathBuf,
    #[command(flatten)]
    dev: DevModeArgs,
}
#[derive(Parser)]
#[command(
    about = "Allow/forbid a gmap rule for tenant or team.",
    long_about = "Updates the appropriate gmap file with a deterministic ordering.",
    after_help = "Main options:\n  --tenant <TENANT>\n  --path <PACK[/FLOW[/NODE]]>\n\nOptional options:\n  --team <TEAM>\n  --project-root <PATH> (default: current directory)"
)]
struct DevPolicyArgs {
    #[arg(long)]
    tenant: String,
    #[arg(long)]
    team: Option<String>,
    #[arg(long)]
    path: String,
    #[arg(long)]
    project_root: Option<PathBuf>,
}

#[derive(Clone, Debug, Parser)]
struct DevModeArgs {
    #[arg(long, value_enum)]
    dev_mode: Option<DevModeArg>,
    #[arg(long)]
    dev_root: Option<PathBuf>,
    #[arg(long, value_enum)]
    dev_profile: Option<DevProfileArg>,
    #[arg(long)]
    dev_target_dir: Option<PathBuf>,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum DevModeArg {
    Auto,
    On,
    Off,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum DevProfileArg {
    Debug,
    Release,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum Format {
    Text,
    Json,
    Yaml,
}

impl Cli {
    pub fn run(self) -> anyhow::Result<()> {
        let settings = settings::load_settings()?;
        let mut ctx = AppCtx { settings };
        match self.command {
            Command::Dev(dev) => dev.run(&mut ctx),
            Command::Demo(demo) => demo.run(&ctx),
        }
    }
}

struct AppCtx {
    settings: settings::OperatorSettings,
}

impl DevCommand {
    fn run(self, ctx: &mut AppCtx) -> anyhow::Result<()> {
        match self.command {
            DevSubcommand::On(args) => {
                ctx.settings = args.run(ctx.settings.clone())?;
                Ok(())
            }
            DevSubcommand::Off(args) => {
                ctx.settings = args.run(ctx.settings.clone())?;
                Ok(())
            }
            DevSubcommand::Status(args) => {
                ctx.settings = args.run(ctx.settings.clone())?;
                Ok(())
            }
            DevSubcommand::Detect(args) => {
                ctx.settings = args.run(ctx.settings.clone())?;
                Ok(())
            }
            DevSubcommand::Map(args) => {
                ctx.settings = args.run(ctx.settings.clone())?;
                Ok(())
            }
            DevSubcommand::Init(args) => args.run(),
            DevSubcommand::Scan(args) => args.run(),
            DevSubcommand::Sync(args) => args.run(),
            DevSubcommand::Tenant(args) => args.run(),
            DevSubcommand::Team(args) => args.run(),
            DevSubcommand::Allow(args) => args.run(Policy::Public),
            DevSubcommand::Forbid(args) => args.run(Policy::Forbidden),
            DevSubcommand::Up(args) => args.run(ctx),
            DevSubcommand::Down(args) => args.run(),
            DevSubcommand::SvcStatus(args) => args.run(),
            DevSubcommand::Logs(args) => args.run(),
            DevSubcommand::Setup(args) => args.run(),
            DevSubcommand::Diagnostics(args) => args.run(),
            DevSubcommand::Verify(args) => args.run(),
            DevSubcommand::Doctor(args) => args.run(ctx),
            DevSubcommand::Secrets(args) => args.run(&ctx.settings),
        }
    }
}

impl DemoCommand {
    fn run(self, ctx: &AppCtx) -> anyhow::Result<()> {
        if self.debug {
            unsafe {
                std::env::set_var("GREENTIC_OPERATOR_DEMO_DEBUG", "1");
            }
        }
        match self.command {
            DemoSubcommand::Build(args) => args.run(ctx),
            DemoSubcommand::Up(args) => args.run(ctx),
            DemoSubcommand::Setup(args) => args.run(),
            DemoSubcommand::Down(args) => args.run(),
            DemoSubcommand::Status(args) => args.run(),
            DemoSubcommand::Logs(args) => args.run(),
            DemoSubcommand::Doctor(args) => args.run(ctx),
        }
    }
}

impl DevInitArgs {
    fn run(self) -> anyhow::Result<()> {
        let root = project_root(self.project_root)?;
        project::init_project(&root)
    }
}

impl DevScanArgs {
    fn run(self) -> anyhow::Result<()> {
        let root = project_root(self.project_root)?;
        let format = match self.format {
            Format::Text => ScanFormat::Text,
            Format::Json => ScanFormat::Json,
            Format::Yaml => ScanFormat::Yaml,
        };
        project::scan_project(&root, format)
    }
}

impl DevSyncArgs {
    fn run(self) -> anyhow::Result<()> {
        let root = project_root(self.project_root)?;
        project::sync_project(&root)
    }
}

impl TenantCommand {
    fn run(self) -> anyhow::Result<()> {
        match self.command {
            TenantSubcommand::Add(args) => args.run(),
            TenantSubcommand::Rm(args) => args.run(),
            TenantSubcommand::List(args) => args.run(),
        }
    }
}

impl TenantAddArgs {
    fn run(self) -> anyhow::Result<()> {
        let root = project_root(self.project_root)?;
        project::add_tenant(&root, &self.tenant)
    }
}

impl TenantRmArgs {
    fn run(self) -> anyhow::Result<()> {
        let root = project_root(self.project_root)?;
        project::remove_tenant(&root, &self.tenant)
    }
}

impl TenantListArgs {
    fn run(self) -> anyhow::Result<()> {
        let root = project_root(self.project_root)?;
        for tenant in project::list_tenants(&root)? {
            println!("{tenant}");
        }
        Ok(())
    }
}

impl TeamCommand {
    fn run(self) -> anyhow::Result<()> {
        match self.command {
            TeamSubcommand::Add(args) => args.run(),
            TeamSubcommand::Rm(args) => args.run(),
            TeamSubcommand::List(args) => args.run(),
        }
    }
}

impl TeamAddArgs {
    fn run(self) -> anyhow::Result<()> {
        let root = project_root(self.project_root)?;
        project::add_team(&root, &self.tenant, &self.team)
    }
}

impl TeamRmArgs {
    fn run(self) -> anyhow::Result<()> {
        let root = project_root(self.project_root)?;
        project::remove_team(&root, &self.tenant, &self.team)
    }
}

impl TeamListArgs {
    fn run(self) -> anyhow::Result<()> {
        let root = project_root(self.project_root)?;
        for team in project::list_teams(&root, &self.tenant)? {
            println!("{team}");
        }
        Ok(())
    }
}

impl DevPolicyArgs {
    fn run(self, policy: Policy) -> anyhow::Result<()> {
        let root = project_root(self.project_root)?;
        let gmap_path = match self.team {
            Some(team) => root
                .join("tenants")
                .join(&self.tenant)
                .join("teams")
                .join(team)
                .join("team.gmap"),
            None => root.join("tenants").join(&self.tenant).join("tenant.gmap"),
        };
        gmap::upsert_policy(&gmap_path, &self.path, policy)
    }
}

impl DevUpArgs {
    fn run(self, ctx: &AppCtx) -> anyhow::Result<()> {
        let root = project_root(self.project_root)?;
        let config = config::load_operator_config(&root)?;
        let dev_settings = resolve_dev_settings(&ctx.settings, config.as_ref(), &self.dev, &root)?;
        let messaging_binary = config::messaging_binary(config.as_ref(), &root);
        let command = bin_resolver::resolve_binary(
            &messaging_binary.name,
            &ResolveCtx {
                config_dir: root.clone(),
                dev: dev_settings.clone(),
                explicit_path: messaging_binary.explicit_path,
            },
        )?;

        let mut nats_url = self.nats_url.clone();
        if !self.no_nats && nats_url.is_none() {
            if let Err(err) = crate::services::start_nats(&root) {
                eprintln!("Warning: failed to start NATS: {err}");
            } else {
                nats_url = Some(crate::services::nats_url(&root));
            }
        }

        let state = crate::services::start_messaging_with_command(
            &root,
            &self.tenant,
            self.team.as_deref(),
            nats_url.as_deref(),
            &command.to_string_lossy(),
        )?;
        println!("messaging: {:?}", state);
        Ok(())
    }
}

impl DevDownArgs {
    fn run(self) -> anyhow::Result<()> {
        let root = project_root(self.project_root)?;
        let state = crate::services::stop_messaging(&root, &self.tenant, self.team.as_deref())?;
        println!("messaging: {:?}", state);

        if !self.no_nats {
            let nats = crate::services::stop_nats(&root)?;
            println!("nats: {:?}", nats);
        }
        Ok(())
    }
}

impl DevStatusArgs {
    fn run(self) -> anyhow::Result<()> {
        let root = project_root(self.project_root)?;
        let messaging =
            crate::services::messaging_status(&root, &self.tenant, self.team.as_deref())?;
        println!("messaging: {:?}", messaging);

        if !self.no_nats {
            let nats = crate::services::nats_status(&root)?;
            println!("nats: {:?}", nats);
        }
        Ok(())
    }
}

impl DevLogsArgs {
    fn run(self) -> anyhow::Result<()> {
        let root = project_root(self.project_root)?;
        match self.service.unwrap_or(LogService::Messaging) {
            LogService::Messaging => {
                crate::services::tail_messaging_logs(&root, &self.tenant, self.team.as_deref())
            }
            LogService::Nats => crate::services::tail_nats_logs(&root),
        }
    }
}

impl DomainSetupArgs {
    fn run(self) -> anyhow::Result<()> {
        let root = project_root(self.project_root)?;
        run_domain_command(DomainRunArgs {
            root,
            state_root: None,
            domain: self.domain.into(),
            action: DomainAction::Setup,
            tenant: self.tenant,
            team: self.team,
            provider_filter: self.provider,
            dry_run: self.dry_run,
            format: self.format,
            parallel: self.parallel,
            allow_missing_setup: self.allow_missing_setup,
            online: self.online,
            secrets_env: self.secrets_env,
            secrets_bin: self.secrets_bin,
        })
    }
}

impl DomainDiagnosticsArgs {
    fn run(self) -> anyhow::Result<()> {
        let root = project_root(self.project_root)?;
        run_domain_command(DomainRunArgs {
            root,
            state_root: None,
            domain: self.domain.into(),
            action: DomainAction::Diagnostics,
            tenant: self.tenant,
            team: self.team,
            provider_filter: self.provider,
            dry_run: self.dry_run,
            format: self.format,
            parallel: self.parallel,
            allow_missing_setup: true,
            online: self.online,
            secrets_env: None,
            secrets_bin: None,
        })
    }
}

impl DomainVerifyArgs {
    fn run(self) -> anyhow::Result<()> {
        let root = project_root(self.project_root)?;
        run_domain_command(DomainRunArgs {
            root,
            state_root: None,
            domain: self.domain.into(),
            action: DomainAction::Verify,
            tenant: self.tenant,
            team: self.team,
            provider_filter: self.provider,
            dry_run: self.dry_run,
            format: self.format,
            parallel: self.parallel,
            allow_missing_setup: true,
            online: self.online,
            secrets_env: None,
            secrets_bin: None,
        })
    }
}

impl DevDoctorArgs {
    fn run(self, ctx: &AppCtx) -> anyhow::Result<()> {
        let root = project_root(self.project_root)?;
        let config = config::load_operator_config(&root)?;
        let dev_settings = resolve_dev_settings(&ctx.settings, config.as_ref(), &self.dev, &root)?;
        let explicit = config::binary_override(config.as_ref(), "greentic-pack", &root);
        let pack_command = bin_resolver::resolve_binary(
            "greentic-pack",
            &ResolveCtx {
                config_dir: root.clone(),
                dev: dev_settings,
                explicit_path: explicit,
            },
        )?;

        let scope = match self.domain {
            DoctorDomainArg::Messaging => crate::doctor::DoctorScope::One(Domain::Messaging),
            DoctorDomainArg::Events => crate::doctor::DoctorScope::One(Domain::Events),
            DoctorDomainArg::Secrets => crate::doctor::DoctorScope::One(Domain::Secrets),
            DoctorDomainArg::All => crate::doctor::DoctorScope::All,
        };
        crate::doctor::run_doctor(
            &root,
            scope,
            crate::doctor::DoctorOptions {
                tenant: self.tenant,
                team: self.team,
                strict: self.strict,
                validator_packs: self.validator_pack,
            },
            &pack_command,
        )?;
        Ok(())
    }
}

impl DemoBuildArgs {
    fn run(self, ctx: &AppCtx) -> anyhow::Result<()> {
        let root = project_root(self.project_root)?;
        if demo_debug_enabled() {
            println!(
                "[demo] build root={} out={} tenant={:?} team={:?} doctor={}",
                root.display(),
                self.out.display(),
                self.tenant,
                self.team,
                self.doctor
            );
        }
        let options = BuildOptions {
            out_dir: self.out,
            tenant: self.tenant,
            team: self.team,
            allow_pack_dirs: self.allow_pack_dirs,
            only_used_providers: self.only_used_providers,
            run_doctor: self.doctor,
        };
        let config = config::load_operator_config(&root)?;
        let dev_settings = resolve_dev_settings(&ctx.settings, config.as_ref(), &self.dev, &root)?;
        let pack_command = if options.run_doctor {
            let explicit = config::binary_override(config.as_ref(), "greentic-pack", &root);
            Some(bin_resolver::resolve_binary(
                "greentic-pack",
                &ResolveCtx {
                    config_dir: root.clone(),
                    dev: dev_settings,
                    explicit_path: explicit,
                },
            )?)
        } else {
            None
        };
        demo::build_bundle(&root, options, pack_command.as_deref())
    }
}

impl DemoUpArgs {
    fn run(self, ctx: &AppCtx) -> anyhow::Result<()> {
        let restart: std::collections::BTreeSet<String> =
            self.restart.iter().map(restart_name).collect();
        if let Some(bundle) = self.bundle {
            if demo_debug_enabled() {
                println!(
                    "[demo] up bundle={} tenant={:?} team={:?} no_nats={} nats_url={:?} cloudflared={:?}",
                    bundle.display(),
                    self.tenant,
                    self.team,
                    self.no_nats,
                    self.nats_url,
                    self.cloudflared
                );
            }
            let tenant = self.tenant.unwrap_or_else(|| "demo".to_string());
            let config = config::load_operator_config(&bundle)?;
            let dev_settings =
                resolve_dev_settings(&ctx.settings, config.as_ref(), &self.dev, &bundle)?;
            let messaging_binary = config::messaging_binary(config.as_ref(), &bundle);
            let command = bin_resolver::resolve_binary(
                &messaging_binary.name,
                &ResolveCtx {
                    config_dir: bundle.clone(),
                    dev: dev_settings,
                    explicit_path: messaging_binary.explicit_path,
                },
            )?;
            let cloudflared = match self.cloudflared {
                CloudflaredModeArg::Off => None,
                CloudflaredModeArg::On => {
                    let explicit = self.cloudflared_binary.clone();
                    let binary = bin_resolver::resolve_binary(
                        "cloudflared",
                        &ResolveCtx {
                            config_dir: bundle.clone(),
                            dev: None,
                            explicit_path: explicit,
                        },
                    )?;
                    Some(crate::cloudflared::CloudflaredConfig {
                        binary,
                        local_port: 8080,
                        extra_args: Vec::new(),
                        restart: restart.contains("cloudflared"),
                    })
                }
            };

            return demo::demo_up(
                &bundle,
                &tenant,
                self.team.as_deref(),
                self.nats_url.as_deref(),
                self.no_nats,
                &command.to_string_lossy(),
                cloudflared,
            );
        }

        let config_path = resolve_demo_config_path(self.config)?;
        let demo_config = config::load_demo_config(&config_path)?;
        let operator_config =
            config::load_operator_config(config_path.parent().unwrap_or(Path::new(".")))?;
        let dev_settings = resolve_dev_settings(
            &ctx.settings,
            operator_config.as_ref(),
            &self.dev,
            config_path.parent().unwrap_or(Path::new(".")),
        )?;
        let cloudflared = match self.cloudflared {
            CloudflaredModeArg::Off => None,
            CloudflaredModeArg::On => {
                let explicit = self.cloudflared_binary.clone();
                let binary = bin_resolver::resolve_binary(
                    "cloudflared",
                    &ResolveCtx {
                        config_dir: config_path.parent().unwrap_or(Path::new(".")).to_path_buf(),
                        dev: None,
                        explicit_path: explicit,
                    },
                )?;
                Some(crate::cloudflared::CloudflaredConfig {
                    binary,
                    local_port: demo_config.services.gateway.port,
                    extra_args: Vec::new(),
                    restart: restart.contains("cloudflared"),
                })
            }
        };

        let provider_options = crate::providers::ProviderSetupOptions {
            providers: if self.providers.is_empty() {
                None
            } else {
                Some(self.providers)
            },
            verify_webhooks: self.verify_webhooks,
            force_setup: self.force_setup,
            skip_setup: self.skip_setup,
            skip_secrets_init: self.skip_secrets_init,
            setup_input: self.setup_input,
            runner_binary: self.runner_binary,
        };

        demo::demo_up_services(
            &config_path,
            &demo_config,
            dev_settings,
            cloudflared,
            &restart,
            provider_options,
        )
    }
}

impl DemoSetupArgs {
    fn run(self) -> anyhow::Result<()> {
        let domains: Vec<Domain> = match self.domain {
            DemoSetupDomainArg::Messaging => vec![Domain::Messaging],
            DemoSetupDomainArg::Events => vec![Domain::Events],
            DemoSetupDomainArg::Secrets => vec![Domain::Secrets],
            DemoSetupDomainArg::All => vec![Domain::Messaging, Domain::Events, Domain::Secrets],
        };
        if demo_debug_enabled() {
            println!(
                "[demo] setup bundle={} tenant={} team={:?} domains={:?} provider_filter={:?} dry_run={} parallel={} skip_secrets_init={}",
                self.bundle.display(),
                self.tenant,
                self.team,
                domains,
                self.provider,
                self.dry_run,
                self.parallel,
                self.skip_secrets_init
            );
        }
        let format = match self.format {
            Format::Text => PlanFormat::Text,
            Format::Json => PlanFormat::Json,
            Format::Yaml => PlanFormat::Yaml,
        };
        for domain in domains {
            run_domain_command(DomainRunArgs {
                root: self.bundle.clone(),
                state_root: self.state_dir.clone(),
                domain,
                action: DomainAction::Setup,
                tenant: self.tenant.clone(),
                team: self.team.clone(),
                provider_filter: self.provider.clone(),
                dry_run: self.dry_run,
                format,
                parallel: self.parallel,
                allow_missing_setup: self.allow_missing_setup,
                online: self.online,
                secrets_env: if self.skip_secrets_init {
                    None
                } else {
                    self.secrets_env.clone()
                },
                secrets_bin: if self.skip_secrets_init {
                    None
                } else {
                    self.secrets_bin.clone()
                },
            })?;
        }
        Ok(())
    }
}
fn restart_name(target: &RestartTarget) -> String {
    match target {
        RestartTarget::All => "all",
        RestartTarget::Cloudflared => "cloudflared",
        RestartTarget::Nats => "nats",
        RestartTarget::Gateway => "gateway",
        RestartTarget::Egress => "egress",
        RestartTarget::Subscriptions => "subscriptions",
    }
    .to_string()
}

fn resolve_demo_config_path(explicit: Option<PathBuf>) -> anyhow::Result<PathBuf> {
    if let Some(path) = explicit {
        return Ok(path);
    }
    let cwd = std::env::current_dir()?;
    let demo_path = cwd.join("demo").join("demo.yaml");
    if demo_path.exists() {
        return Ok(demo_path);
    }
    let fallback = cwd.join("greentic.operator.yaml");
    if fallback.exists() {
        return Ok(fallback);
    }
    Err(anyhow::anyhow!(
        "no demo config found; pass --config or create ./demo/demo.yaml"
    ))
}

impl DemoDownArgs {
    fn run(self) -> anyhow::Result<()> {
        if let Some(bundle) = self.bundle.as_ref()
            && self.state_dir.is_none()
        {
            if demo_debug_enabled() {
                println!(
                    "[demo] down bundle={} tenant={} team={} no_nats={}",
                    bundle.display(),
                    self.tenant,
                    self.team,
                    self.no_nats
                );
            }
            return demo::demo_down_legacy(bundle, &self.tenant, Some(&self.team), self.no_nats);
        }

        let state_dir = resolve_state_dir(self.state_dir, self.bundle.as_ref());
        if demo_debug_enabled() {
            println!(
                "[demo] down state_dir={} tenant={} team={} all={}",
                state_dir.display(),
                self.tenant,
                self.team,
                self.all
            );
        }
        demo::demo_down_runtime(&state_dir, &self.tenant, &self.team, self.all)
    }
}

impl DemoStatusArgs {
    fn run(self) -> anyhow::Result<()> {
        if let Some(bundle) = self.bundle.as_ref()
            && self.state_dir.is_none()
        {
            if demo_debug_enabled() {
                println!(
                    "[demo] status bundle={} tenant={} team={} no_nats={} verbose={}",
                    bundle.display(),
                    self.tenant,
                    self.team,
                    self.no_nats,
                    self.verbose
                );
            }
            return demo::demo_status_legacy(bundle, &self.tenant, Some(&self.team), self.no_nats);
        }

        let state_dir = resolve_state_dir(self.state_dir, self.bundle.as_ref());
        if demo_debug_enabled() {
            println!(
                "[demo] status state_dir={} tenant={} team={} verbose={}",
                state_dir.display(),
                self.tenant,
                self.team,
                self.verbose
            );
        }
        demo::demo_status_runtime(&state_dir, &self.tenant, &self.team, self.verbose)
    }
}

impl DemoLogsArgs {
    fn run(self) -> anyhow::Result<()> {
        if let Some(bundle) = self.bundle.as_ref()
            && self.state_dir.is_none()
        {
            if demo_debug_enabled() {
                println!(
                    "[demo] logs bundle={} tenant={} team={} service={} tail={}",
                    bundle.display(),
                    self.tenant,
                    self.team,
                    self.service,
                    self.tail
                );
            }
            return demo::demo_logs_legacy(
                bundle,
                &self.tenant,
                Some(&self.team),
                Some(self.service.as_str()),
            );
        }

        let state_dir = resolve_state_dir(self.state_dir, self.bundle.as_ref());
        if demo_debug_enabled() {
            println!(
                "[demo] logs state_dir={} tenant={} team={} service={} tail={}",
                state_dir.display(),
                self.tenant,
                self.team,
                self.service,
                self.tail
            );
        }
        demo::demo_logs_runtime(
            &state_dir,
            &self.tenant,
            &self.team,
            &self.service,
            self.tail,
        )
    }
}

impl DemoDoctorArgs {
    fn run(self, ctx: &AppCtx) -> anyhow::Result<()> {
        let config = config::load_operator_config(&self.bundle)?;
        let dev_settings =
            resolve_dev_settings(&ctx.settings, config.as_ref(), &self.dev, &self.bundle)?;
        let explicit = config::binary_override(config.as_ref(), "greentic-pack", &self.bundle);
        let pack_command = bin_resolver::resolve_binary(
            "greentic-pack",
            &ResolveCtx {
                config_dir: self.bundle.clone(),
                dev: dev_settings.clone(),
                explicit_path: explicit,
            },
        )?;
        if dev_settings.is_some() {
            println!("greentic-pack -> {} ✅", pack_command.display());
        }
        if demo_debug_enabled() {
            println!(
                "[demo] doctor bundle={} greentic-pack={}",
                self.bundle.display(),
                pack_command.display()
            );
        }
        demo::demo_doctor(&self.bundle, &pack_command)
    }
}

fn project_root(arg: Option<PathBuf>) -> anyhow::Result<PathBuf> {
    Ok(arg.unwrap_or(std::env::current_dir()?))
}

fn resolve_state_dir(state_dir: Option<PathBuf>, bundle: Option<&PathBuf>) -> PathBuf {
    if let Some(state_dir) = state_dir {
        return state_dir;
    }
    if let Some(bundle) = bundle {
        return bundle.join("state");
    }
    PathBuf::from("state")
}

fn demo_debug_enabled() -> bool {
    matches!(
        std::env::var("GREENTIC_OPERATOR_DEMO_DEBUG").as_deref(),
        Ok("1") | Ok("true") | Ok("yes")
    )
}

fn demo_provider_files(
    root: &Path,
    tenant: &str,
    team: Option<&str>,
    domain: Domain,
) -> anyhow::Result<Option<std::collections::BTreeSet<String>>> {
    let resolved = demo_resolved_manifest_path(root, tenant, team);
    if !resolved.exists() {
        return Ok(None);
    }
    let contents = std::fs::read_to_string(resolved)?;
    let manifest: DemoResolvedManifest = serde_yaml_bw::from_str(&contents)?;
    let key = match domain {
        Domain::Messaging => "messaging",
        Domain::Events => "events",
        Domain::Secrets => "secrets",
    };
    let Some(list) = manifest.providers.get(key) else {
        return Ok(Some(std::collections::BTreeSet::new()));
    };
    let mut files = std::collections::BTreeSet::new();
    for path in list {
        if let Some(name) = Path::new(path).file_name().and_then(|value| value.to_str()) {
            files.insert(name.to_string());
        }
    }
    Ok(Some(files))
}

fn demo_resolved_manifest_path(root: &Path, tenant: &str, team: Option<&str>) -> PathBuf {
    let filename = match team {
        Some(team) => format!("{tenant}.{team}.yaml"),
        None => format!("{tenant}.yaml"),
    };
    root.join("resolved").join(filename)
}

#[derive(serde::Deserialize)]
struct DemoResolvedManifest {
    #[serde(default)]
    providers: std::collections::BTreeMap<String, Vec<String>>,
}

fn resolve_dev_settings(
    settings: &settings::OperatorSettings,
    config: Option<&config::OperatorConfig>,
    overrides: &DevModeArgs,
    config_dir: &Path,
) -> anyhow::Result<Option<DevSettingsResolved>> {
    let global = settings.dev.to_dev_settings();
    let merged = merge_settings(config.and_then(|cfg| cfg.dev.clone()), Some(global));
    effective_dev_settings(overrides.to_overrides(), merged, config_dir)
}

impl DevModeArgs {
    fn to_overrides(&self) -> DevCliOverrides {
        DevCliOverrides {
            mode: self.dev_mode.map(Into::into),
            root: self.dev_root.clone(),
            profile: self.dev_profile.map(Into::into),
            target_dir: self.dev_target_dir.clone(),
        }
    }
}

impl From<DevModeArg> for DevMode {
    fn from(value: DevModeArg) -> Self {
        match value {
            DevModeArg::Auto => DevMode::Auto,
            DevModeArg::On => DevMode::On,
            DevModeArg::Off => DevMode::Off,
        }
    }
}

impl From<DevProfileArg> for DevProfile {
    fn from(value: DevProfileArg) -> Self {
        match value {
            DevProfileArg::Debug => DevProfile::Debug,
            DevProfileArg::Release => DevProfile::Release,
        }
    }
}

struct DomainRunArgs {
    root: PathBuf,
    state_root: Option<PathBuf>,
    domain: Domain,
    action: DomainAction,
    tenant: String,
    team: Option<String>,
    provider_filter: Option<String>,
    dry_run: bool,
    format: PlanFormat,
    parallel: usize,
    allow_missing_setup: bool,
    online: bool,
    secrets_env: Option<String>,
    secrets_bin: Option<PathBuf>,
}

fn run_domain_command(args: DomainRunArgs) -> anyhow::Result<()> {
    let is_demo_bundle = args.root.join("greentic.demo.yaml").exists();
    let mut packs = domains::discover_provider_packs(&args.root, args.domain)?;
    if is_demo_bundle {
        if let Some(allowed) =
            demo_provider_files(&args.root, &args.tenant, args.team.as_deref(), args.domain)?
        {
            packs.retain(|pack| allowed.contains(&pack.file_name));
        }
    }
    if packs.is_empty() {
        if is_demo_bundle {
            println!(
                "No provider packs found under {}. If you expected providers, rebuild the bundle.",
                args.root.display()
            );
        } else {
            println!(
                "No provider packs found under {} (use --project-root to point at your project).",
                args.root.display()
            );
        }
        return Ok(());
    }
    let plan = domains::plan_runs(
        args.domain,
        args.action,
        &packs,
        args.provider_filter.as_deref(),
        args.allow_missing_setup,
    )?;

    if plan.is_empty() {
        if is_demo_bundle {
            println!("No provider packs matched. Try --provider <pack_id>.");
        } else {
            println!(
                "No provider packs matched. Try --provider <pack_id> or --project-root."
            );
        }
        return Ok(());
    }

    if args.dry_run {
        render_plan(&plan, args.format)?;
        return Ok(());
    }

    let dist_offline = !args.online;
    let state_root = args.state_root.as_ref().unwrap_or(&args.root);
    run_plan(
        &args.root,
        state_root,
        args.domain,
        args.action,
        &args.tenant,
        args.team.as_deref(),
        plan,
        args.parallel,
        dist_offline,
        args.secrets_env.as_deref(),
        args.secrets_bin.as_deref(),
    )
}

#[allow(clippy::too_many_arguments)]
fn run_plan(
    root: &Path,
    state_root: &Path,
    domain: Domain,
    action: DomainAction,
    tenant: &str,
    team: Option<&str>,
    plan: Vec<domains::PlannedRun>,
    parallel: usize,
    dist_offline: bool,
    secrets_env: Option<&str>,
    secrets_bin: Option<&Path>,
) -> anyhow::Result<()> {
    if parallel <= 1 {
        for item in plan {
            run_plan_item(
                root,
                state_root,
                domain,
                action,
                tenant,
                team,
                &item,
                dist_offline,
                secrets_env,
                secrets_bin,
            )?;
        }
        return Ok(());
    }

    let mut handles = Vec::new();
    let plan = std::sync::Arc::new(std::sync::Mutex::new(plan));
    let errors = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));

    for _ in 0..parallel {
        let plan = plan.clone();
        let errors = errors.clone();
        let root = root.to_path_buf();
        let state_root = state_root.to_path_buf();
        let tenant = tenant.to_string();
        let team = team.map(|value| value.to_string());
        let secrets_env = secrets_env.map(|value| value.to_string());
        let secrets_bin = secrets_bin.map(|value| value.to_path_buf());
        handles.push(std::thread::spawn(move || {
            loop {
                let next = {
                    let mut queue = plan.lock().unwrap();
                    queue.pop()
                };
                let Some(item) = next else {
                    break;
                };
                let result = run_plan_item(
                    &root,
                    &state_root,
                    domain,
                    action,
                    &tenant,
                    team.as_deref(),
                    &item,
                    dist_offline,
                    secrets_env.as_deref(),
                    secrets_bin.as_deref(),
                );
                if let Err(err) = result {
                    errors.lock().unwrap().push(err);
                }
            }
        }));
    }

    for handle in handles {
        let _ = handle.join();
    }

    let errors = errors.lock().unwrap();
    if !errors.is_empty() {
        return Err(anyhow::anyhow!("{} flow(s) failed.", errors.len()));
    }
    Ok(())
}

fn render_plan(plan: &[domains::PlannedRun], format: PlanFormat) -> anyhow::Result<()> {
    match format {
        PlanFormat::Text => {
            println!("Plan:");
            for item in plan {
                println!("  {} -> {}", item.pack.file_name, item.flow_id);
            }
            Ok(())
        }
        PlanFormat::Json => {
            let json = serde_json::to_string_pretty(plan)?;
            println!("{json}");
            Ok(())
        }
        PlanFormat::Yaml => {
            let yaml = serde_yaml_bw::to_string(plan)?;
            print!("{yaml}");
            Ok(())
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn run_plan_item(
    _root: &Path,
    state_root: &Path,
    domain: Domain,
    action: DomainAction,
    tenant: &str,
    team: Option<&str>,
    item: &domains::PlannedRun,
    dist_offline: bool,
    secrets_env: Option<&str>,
    secrets_bin: Option<&Path>,
) -> anyhow::Result<()> {
    let input = build_input_payload(state_root, domain, tenant, team);
    let output = runner_exec::run_provider_pack_flow(runner_exec::RunRequest {
        root: state_root.to_path_buf(),
        domain,
        pack_path: item.pack.path.clone(),
        pack_label: item.pack.pack_id.clone(),
        flow_id: item.flow_id.clone(),
        tenant: tenant.to_string(),
        team: team.map(|value| value.to_string()),
        input,
        dist_offline,
    })?;
    println!(
        "{} {} -> {:?}",
        item.pack.file_name, item.flow_id, output.result.status
    );

    if domain == Domain::Messaging && action == DomainAction::Setup {
        let setup_flow = domains::config(domain).setup_flow;
        if item.flow_id == setup_flow {
            let Some(secrets_bin) = secrets_bin else {
                return Ok(());
            };
            let env = secrets_env
                .map(|value| value.to_string())
                .unwrap_or_else(crate::tools::secrets::resolve_env);
            let status = crate::tools::secrets::run_init(
                state_root,
                Some(secrets_bin),
                &env,
                tenant,
                team,
                &item.pack.path,
                true,
            )?;
            if !status.success() {
                let code = status.code().unwrap_or(1);
                return Err(anyhow::anyhow!(
                    "greentic-secrets init failed with exit code {code}"
                ));
            }
        }
    }
    Ok(())
}

fn build_input_payload(
    root: &Path,
    domain: Domain,
    tenant: &str,
    team: Option<&str>,
) -> serde_json::Value {
    let mut payload = serde_json::json!({
        "tenant": tenant,
    });
    if let Some(team) = team {
        payload["team"] = serde_json::Value::String(team.to_string());
    }

    match domain {
        Domain::Messaging | Domain::Events => {
            let mut config = serde_json::json!({});
            if let Some(url) = read_public_base_url(root, tenant, team) {
                payload["public_base_url"] = serde_json::Value::String(url);
                config["public_base_url"] = payload["public_base_url"].clone();
            }
            payload["config"] = config;
            payload
        }
        Domain::Secrets => payload,
    }
}

fn read_public_base_url(root: &Path, tenant: &str, team: Option<&str>) -> Option<String> {
    let team_id = team.unwrap_or("default");
    let paths = crate::runtime_state::RuntimePaths::new(root.join("state"), tenant, team_id);
    let path = crate::cloudflared::public_url_path(&paths);
    std::fs::read_to_string(path).ok().map(|value| value.trim().to_string()).filter(|value| !value.is_empty())
}

impl From<DomainArg> for Domain {
    fn from(value: DomainArg) -> Self {
        match value {
            DomainArg::Messaging => Domain::Messaging,
            DomainArg::Events => Domain::Events,
            DomainArg::Secrets => Domain::Secrets,
        }
    }
}
