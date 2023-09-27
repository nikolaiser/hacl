mod cli;
mod config;

use std::fmt::Display;

use error_stack::{IntoReport, Report, Result, ResultExt};

use crate::cli::create_app;
use crate::cli::handle_sub_commands;
use crate::cli::SubcommandGiven;
use crate::config::Area;
use crate::config::Config;
use crate::config::ConfigError;
use reqwest::blocking::{Client, Request, RequestBuilder};
use reqwest::{Method, Url};
use skim::prelude::*;
use std::io::Cursor;

#[derive(Debug)]
pub(crate) enum HaclError {
    IntenalError,
    CliError,
    ConfigError,
    HaError,
    FuzzyFindError(String),
    IOError,
}
impl Display for HaclError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HaclError::CliError => write!(f, "Cli Error"),
            HaclError::HaError => write!(f, "Home Assistant Error"),
            HaclError::ConfigError => write!(f, "Config Error"),
            HaclError::FuzzyFindError(inner) => write!(f, "Fuzzy Find Error: {inner}"),
            HaclError::IntenalError => write!(f, "Intenal Error"),
            HaclError::IOError => write!(f, "IO Error"),
        }
    }
}
impl std::error::Error for HaclError {}

fn main() -> Result<(), HaclError> {
    // Install debug hooks for formatting of error handling
    Report::install_debug_hook::<Suggestion>(|value, context| {
        context.push_body(format!("{value}"));
    });
    #[cfg(any(not(debug_assertions), test))]
    Report::install_debug_hook::<std::panic::Location>(|_value, _context| {});

    // Use CLAP to parse the command line arguments
    let cli_args = create_app();
    let config = match handle_sub_commands(cli_args)? {
        SubcommandGiven::Yes => return Ok(()),
        SubcommandGiven::No(config) => config, // continue
    };

    if config.base_url.is_empty() {
        return Err(ConfigError::NoBaseUrl).attach("You must configure base Home Assistant url in the configuration file or with the config subcommand. E.g. `hacl config`").change_context(HaclError::ConfigError);
    };

    if config.token.is_empty() {
        return Err(ConfigError::NoApiToken).attach("You must configure Home Assistant API token in the configuration file or with the config subcommand. E.g. `hacl config`").change_context(HaclError::ConfigError);
    };

    let areas = collect_areas(&config)?;

    let areas_string: String = areas.iter().map(|x| format!("{}\n", &x.id)).collect();

    let selected_id = get_single_selection(areas_string, None)?;

    let selected = areas
        .into_iter()
        .find(|x| x.id == selected_id)
        .ok_or(HaclError::IntenalError)
        .attach("Unexpected area id")
        .change_context(HaclError::IntenalError)?;

    let light_to_toggle: Vec<String> = selected
        .entities
        .into_iter()
        .filter(|x| x.starts_with("light."))
        .collect();

    let _ = toggle_light(&config, light_to_toggle);

    Ok(())
}

fn toggle_light(config: &Config, entities: Vec<String>) -> Result<(), HaclError> {
    let client = Client::new();

    let base_url = Url::parse(&config.base_url)
        .attach("Error parsing base url")
        .change_context(HaclError::ConfigError)?;

    entities
        .into_iter()
        .map(|x| {
            let toggle_url = base_url
                .join("api/services/light/toggle")
                .attach("Internal url parsing error")
                .change_context(HaclError::IntenalError)?;

            let toggle_response = client
                .post(toggle_url)
                .body(format!("{{\"entity_id\": \"{}\"}}", x))
                .header("Authorization", format!("Bearer {}", &config.token))
                .send()
                .attach("Error sending request to Home Assistant")
                .change_context(HaclError::IOError)?;
            Ok(())
        })
        .collect()
}

fn collect_areas(config: &Config) -> Result<Vec<Area>, HaclError> {
    let client = Client::new();

    let base_url = Url::parse(&config.base_url)
        .attach("Error parsing base url")
        .change_context(HaclError::ConfigError)?;

    let template_url = base_url
        .join("api/template")
        .attach("Internal url parsing error")
        .change_context(HaclError::IntenalError)?;

    let areas_response = client
        .post(template_url)
        .body("{\"template\": \"{{ areas() }}\"}")
        .header("Authorization", format!("Bearer {}", &config.token))
        .send()
        .attach("Error sending request to Home Assistant")
        .change_context(HaclError::IOError)?;

    let areas_text = areas_response
        .text()
        .change_context(HaclError::IOError)?
        .replace("'", "\"");

    let area_ids: Vec<String> = serde_json::from_str(&areas_text)
        .attach("Unable to decode HA response as json")
        .change_context(HaclError::HaError)?;

    area_ids
        .into_iter()
        .map(|x| {
            let template_url = base_url
                .join("api/template")
                .attach("Internal url parsing error")
                .change_context(HaclError::IntenalError)?;

            let request_body = format!("{{\"template\": \"{{{{ area_entities('{}') }}}}\"}}", x);

            let area_entities_response = client
                .post(template_url.clone())
                .body(request_body)
                .header("Authorization", format!("Bearer {}", &config.token))
                .send()
                .attach("Error sending request to Home Assistant")
                .change_context(HaclError::IOError)?;

            let area_entities_text = area_entities_response
                .text()
                .change_context(HaclError::IOError)?
                .replace("'", "\"");

            let area_entities_ids: Vec<String> = serde_json::from_str(&area_entities_text)
                .attach("Unable to decode HA response as json")
                .change_context(HaclError::HaError)?;

            Ok(Area {
                id: x,
                entities: area_entities_ids,
            })
        })
        .collect()
}

fn get_single_selection(list: String, preview: Option<&str>) -> Result<String, HaclError> {
    let options = SkimOptionsBuilder::default()
        .height(Some("50%"))
        .preview(preview)
        .multi(false)
        .color(Some("dark"))
        .build()
        .map_err(|x| x.to_string())
        .map_err(HaclError::FuzzyFindError)?;
    let item_reader = SkimItemReader::default();
    let item = item_reader.of_bufread(Cursor::new(list));
    let skim_output = Skim::run_with(&options, Some(item))
        .ok_or_else(|| HaclError::FuzzyFindError("Fuzzy finder internal errors".into()))?;
    if skim_output.is_abort {
        return Err(Report::new(HaclError::CliError).attach_printable("No selection made"));
    }
    Ok(skim_output
        .selected_items
        .get(0)
        .ok_or(HaclError::CliError)
        .attach_printable("No selection made")?
        .output()
        .to_string())
}

#[derive(Debug)]
pub struct Suggestion(&'static str);
impl Display for Suggestion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use owo_colors::OwoColorize;
        f.write_str(
            &owo_colors::OwoColorize::bold(&format!("Suggestion: {}", self.0))
                .green()
                .to_string(),
        )
    }
}
