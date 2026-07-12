pub mod add;
pub mod doctor;
pub mod list;
pub mod resolve;
pub mod schema;

use crate::cli::{Cli, Command};
use crate::error::{AppError, AppResult};
use crate::output;
use crate::policy::{self, Operation};

pub fn run(cli: Cli) -> AppResult<i32> {
    let Cli {
        file,
        pretty,
        profile,
        read_only,
        sensitive_policy,
        command,
    } = cli;
    match command {
        Command::Add(args) => {
            let context = policy::resolve(
                file,
                profile,
                read_only,
                sensitive_policy,
                Operation::Add {
                    dry_run: args.dry_run,
                    agent: args.agent.clone(),
                    allow_sensitive: args.allow_sensitive.clone(),
                },
            )?;
            add::run(args, &context, pretty).map_err(|error| error.with_policy(&context))
        }
        Command::List(args) => {
            let context =
                policy::resolve(file, profile, read_only, sensitive_policy, Operation::List)?;
            list::run(args, &context, pretty).map_err(|error| error.with_policy(&context))
        }
        Command::Resolve(args) => {
            let context = policy::resolve_with_preflight(
                file,
                profile,
                read_only,
                sensitive_policy,
                Operation::Resolve {
                    dry_run: args.dry_run,
                    agent: args.agent.clone(),
                    allow_sensitive: args.allow_sensitive.clone(),
                },
                || resolve::validate_id(&args.id),
            )?;
            resolve::run(args, &context, pretty).map_err(|error| error.with_policy(&context))
        }
        Command::Schema { target } => {
            output::write_success(schema::contract(target), pretty, output::Meta::new())
                .map_err(|error| AppError::from_io(error, std::path::Path::new("stdout")))?;
            Ok(0)
        }
        Command::Doctor => {
            let context = policy::resolve(
                file,
                profile,
                read_only,
                sensitive_policy,
                Operation::Doctor,
            )?;
            doctor::run(&context, pretty).map_err(|error| error.with_policy(&context))
        }
    }
}
