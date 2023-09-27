use crate::config::Config;
use crate::config::ConfigError;
use crate::HaclError;
use clap::{Arg, ArgMatches, Command};
use error_stack::IntoReport;
use error_stack::Report;
use error_stack::Result;
use error_stack::ResultExt;

pub(crate) fn create_app() -> ArgMatches {
    Command::new("hcl")
        .author("nikolaiser")
        .about("Simple home assistant cli to control lights")
        .subcommand(
            Command::new("config")
                .arg_required_else_help(true)
                .arg(
                    Arg::new("base url")
                        .short('u')
                        .long("url")
                        .required(false)
                        .num_args(1)
                        .help("Base Home Assistant url"),
                )
                .arg(
                    Arg::new("API token")
                        .short('t')
                        .long("token")
                        .required(false)
                        .num_args(1)
                        .help("Home Assistant API token"),
                ),
        )
        .get_matches()
}

pub(crate) fn handle_sub_commands(cli_args: ArgMatches) -> Result<SubcommandGiven, HaclError> {
    match cli_args.subcommand() {
        Some(("config", sub_cmd_matches)) => {
            let mut defaults = confy::load::<Config>("hacl", None)
                .attach("Failed to load the config file")
                .change_context(HaclError::ConfigError)?;
            defaults.base_url = sub_cmd_matches
                .get_one::<String>("base url")
                .unwrap_or(&defaults.base_url)
                .into();
            defaults.token = sub_cmd_matches
                .get_one::<String>("API token")
                .unwrap_or(&defaults.token)
                .into();

            let config = Config {
                base_url: defaults.base_url,
                token: defaults.token,
            };

            confy::store("hacl", None, config)
                .map_err(Report::from)
                .attach("Failed to write the config file")
                .change_context(HaclError::ConfigError)?;
            println!("Configuration has been stored");
            Ok(SubcommandGiven::Yes)
        }

        _ => {
            let config = confy::load::<Config>("hacl", None)
                .attach("Failed to load the config file")
                .change_context(HaclError::ConfigError)?;
            Ok(SubcommandGiven::No(config))
        }
    }
}

pub enum SubcommandGiven {
    Yes,
    No(Config),
}
