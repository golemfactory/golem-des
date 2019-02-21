use std::error::Error;
use std::fs::File;
use std::path::Path;

use csv::Writer;
use docopt::Docopt;
use gd_world::logger;
use gd_world::prelude::*;
use gd_world::provider::Stats as PStats;
use gd_world::requestor::DefenceMechanismType;
use gd_world::requestor::Stats as RStats;
use rand::prelude::*;
use rand_chacha::ChaChaRng;
use rayon::prelude::*;
use serde_derive::Deserialize;

mod params;

use crate::params::*;

const USAGE: &'static str = "
Golem marketplace agent-based DES simulator

Usage:
    golem_des <json> [--defence=<defence>] [--repetitions=<repetitions>] [--output-dir=<output-dir>] [--verbose]
    golem_des (-h | --help)

Options:
    json                            JSON file with simulation parameters.
    -v --verbose                    Show debug logs.
    -h --help                       Show this screen.
    --defence=<defence>             Defence mechanism (ctasks, lgrola, or redundancy) [default: redundancy].
    --repetitions=<repetitions>     Number of repetitions [default: 100].
    --output-dir=<output-dir>       Output directory for statistics.
";

#[derive(Debug, Deserialize)]
struct Args {
    arg_json: String,
    flag_defence: DefenceMechanismType,
    flag_repetitions: usize,
    flag_verbose: bool,
    flag_output_dir: Option<String>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Args = Docopt::new(USAGE)
        .and_then(|d| d.deserialize())
        .unwrap_or_else(|e| e.exit());

    logger::init()?;

    if args.flag_verbose {
        log::set_max_level(log::LevelFilter::Debug);
    }

    let file = File::open(Path::new(&args.arg_json))?;
    let params: SimulationParams = serde_json::from_reader(file)?;

    let results: Vec<(Vec<RStats>, Vec<PStats>)> = (0..args.flag_repetitions)
        .into_par_iter()
        .map(|run_num| {
            let mut rng = match params.seed {
                Some(seed) => ChaChaRng::seed_from_u64(seed + run_num as u64),
                None => ChaChaRng::from_entropy(),
            };

            let mut requestors: Vec<Requestor> = Vec::new();
            let mut providers: Vec<Box<dyn Provider>> = Vec::new();

            // create pre-specified actors
            if let Some(rs) = &params.requestors {
                for spec in rs {
                    requestors.push(spec.into_requestor(&mut rng, args.flag_defence));
                }
            }

            if let Some(ps) = &params.providers {
                for spec in ps {
                    providers.push(spec.into_provider());
                }
            }

            // create random actors
            if let Some(sources) = &params.requestor_sources {
                for source in sources {
                    for requestor in source.iter(&mut rng, args.flag_defence) {
                        requestors.push(requestor);
                    }
                }
            }

            if let Some(sources) = &params.provider_sources {
                for source in sources {
                    for provider in source.iter(&mut rng) {
                        providers.push(provider);
                    }
                }
            }

            // create the simulation world; aka the marketplace
            let mut world = World::new(rng);

            // append actors
            world.append_requestors(requestors);
            world.append_providers(providers);

            // run the simulation
            world.run(params.duration);

            // gather statistics
            world.into_stats(run_num as u64)
        })
        .collect();

    let create_path = |fname: &str, id: Option<u64>| {
        let path = Path::new(match &args.flag_output_dir {
            None => ".",
            Some(path) => &path,
        });

        let name =
            String::from(fname) + "_" + &id.map(|value| value.to_string()).unwrap_or(String::new());
        path.join(name).with_extension("csv")
    };

    let mut requestors_wtr = Writer::from_path(create_path("requestors_stats", params.seed))?;
    let mut providers_wtr = Writer::from_path(create_path("providers_stats", params.seed))?;

    for (requestors, providers) in results {
        for requestor in requestors {
            requestors_wtr.serialize(requestor)?;
        }

        for provider in providers {
            providers_wtr.serialize(provider)?;
        }
    }

    Ok(())
}
