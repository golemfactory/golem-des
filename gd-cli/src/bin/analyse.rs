use std::collections::{BTreeMap, HashMap};
use std::error::Error;
use std::io::Read;
use std::path::Path;

use csv;
use docopt::Docopt;
use gnuplot::AxesCommon;
use serde_derive::Deserialize;
use statrs::statistics::Statistics;

use gd_tools::prelude::*;
use gd_world::provider as provider;
use gd_world::requestor as requestor;

const USAGE: &'static str = "
Golem marketplace agent-based DES simulator.
Analysis tool

Usage:
    analyse (providers|requestors) <csv-file>
    golem_des (-h | --help)

Options:
    csv_file                        CSV file with simulation output.
    -h --help                       Show this screen.
";

#[derive(Debug, Deserialize)]
struct Args {
    cmd_providers: bool,
    arg_csv_file: String,
}

pub fn plot<'a, P: AsRef<Path>>(
    values: BTreeMap<String, (f64, f64)>,
    x_label: &'a str,
    y_label: &'a str,
    title: &'a str,
    output_path: P,
) {
    let mut figure = gnuplot::Figure::new();
    figure.set_terminal(
        "pngcairo size 1024,768 enhanced font 'Verdana,12'",
        output_path
            .as_ref()
            .with_extension("png")
            .to_str()
            .expect("could not create output path for the graph"),
    );

    let count = values.len();
    let widths = (0..count + 1).map(|_| 0.5);
    let mut means: Vec<f64> = Vec::new();
    let mut cis: Vec<f64> = Vec::new();

    for (_, value) in &values {
        means.push(value.0);
        cis.push(value.1);
    }

    let xticks: Vec<gnuplot::Tick<usize>> = values
        .into_iter()
        .enumerate()
        .map(|(i, (k, _))| gnuplot::Tick::Major(i, gnuplot::Fix(k)))
        .collect();

    figure
        .axes2d()
        .boxes_set_width(
            0..count + 1,
            &means,
            widths,
            &[
                gnuplot::PlotOption::Color("#8b1a0e"),
                gnuplot::PlotOption::BorderColor("black"),
            ],
        )
        .y_error_bars(
            0..count + 1,
            &means,
            &cis,
            &[
                gnuplot::PlotOption::PointSymbol('.'),
                gnuplot::PlotOption::LineStyle(gnuplot::DashType::Solid),
                gnuplot::PlotOption::Color("black"),
            ],
        )
        .set_grid_options(
            false,
            &[
                gnuplot::PlotOption::LineStyle(gnuplot::DashType::SmallDot),
                gnuplot::PlotOption::LineWidth(1.0),
                gnuplot::PlotOption::Color("#808080"),
            ],
        )
        .set_x_grid(true)
        .set_y_grid(true)
        .set_x_ticks_custom(xticks, &[gnuplot::TickOption::Mirror(false)], &[])
        .set_y_ticks(
            Some((gnuplot::AutoOption::Auto, 1)),
            &[gnuplot::TickOption::Mirror(false)],
            &[],
        )
        .set_y_range(gnuplot::AutoOption::Fix(0.0), gnuplot::AutoOption::Auto)
        .set_x_label(x_label, &[])
        .set_y_label(y_label, &[])
        .set_border(
            true,
            &[
                gnuplot::BorderLocation2D::Bottom,
                gnuplot::BorderLocation2D::Left,
            ],
            &[],
        )
        .set_title(title, &[]);

    figure.show();
}

fn analyse_providers<R, P>(mut rdr: csv::Reader<R>, output_path: P) -> Result<(), Box<dyn Error>>
where
    R: Read,
    P: AsRef<Path>,
{
    let mut runs: HashMap<u64, Vec<provider::Stats>> = HashMap::new();
    for result in rdr.deserialize() {
        let result: provider::Stats = result?;
        runs.entry(result.run_id).or_insert(Vec::new()).push(result);
    }

    let mut subtasks_computed: BTreeMap<provider::Behaviour, Vec<f64>> = BTreeMap::new();
    let mut revenue_ratio: Vec<f64> = Vec::new();

    let mut prices: BTreeMap<provider::Behaviour, BTreeMap<String, Vec<f64>>> = BTreeMap::new();
    let mut effective_prices: BTreeMap<provider::Behaviour, BTreeMap<String, Vec<f64>>> =
        BTreeMap::new();
    let mut revenues: BTreeMap<provider::Behaviour, BTreeMap<String, Vec<f64>>> = BTreeMap::new();

    for (_, results) in runs {
        let all_subtasks_computed: usize = results.iter().map(|p| p.num_subtasks_computed).sum();

        let mut by_behaviour: BTreeMap<provider::Behaviour, Vec<provider::Stats>> = BTreeMap::new();
        for result in results {
            by_behaviour
                .entry(result.behaviour)
                .or_insert(Vec::new())
                .push(result);
        }

        let dishonest_revenue: f64 = by_behaviour
            .iter()
            .filter_map(|(&b, res)| {
                if b == provider::Behaviour::Regular {
                    None
                } else {
                    Some(res)
                }
            })
            .flatten()
            .map(|p| p.revenue)
            .sum();
        let honest_revenue: f64 = by_behaviour
            .iter()
            .filter_map(|(&b, res)| {
                if b == provider::Behaviour::Regular {
                    Some(res)
                } else {
                    None
                }
            })
            .flatten()
            .map(|p| p.revenue)
            .sum();
        revenue_ratio.push(dishonest_revenue / honest_revenue);

        for (behaviour, results) in by_behaviour {
            let num_subtasks_computed: usize =
                results.iter().map(|p| p.num_subtasks_computed).sum();

            subtasks_computed
                .entry(behaviour)
                .or_insert(Vec::new())
                .push(num_subtasks_computed as f64 / all_subtasks_computed as f64 * 100.0);

            for partition in partition_by(results, &[0.25, 0.5, 0.75, 1.0], |p| p.usage_factor) {
                let key = match partition.boundaries() {
                    (Some(lower), Some(upper)) => format!("{} - {}", lower, upper),
                    (None, Some(upper)) => format!("-inf - {}", upper),
                    (Some(lower), None) => format!("{} - inf", lower),
                    (None, None) => format!("-inf - inf"),
                };

                let prices = prices.entry(behaviour).or_insert(BTreeMap::new());
                prices
                    .entry(key.clone())
                    .or_insert(Vec::new())
                    .push(partition.iter().map(|p| p.price).mean());

                let effective_prices = effective_prices.entry(behaviour).or_insert(BTreeMap::new());
                effective_prices
                    .entry(key.clone())
                    .or_insert(Vec::new())
                    .push(partition.iter().map(|p| p.price * p.usage_factor).mean());

                let revenues = revenues.entry(behaviour).or_insert(BTreeMap::new());
                revenues
                    .entry(key)
                    .or_insert(Vec::new())
                    .push(partition.into_iter().map(|p| p.revenue).mean());
            }
        }
    }

    println!("\nMean percentage subtasks computed");
    for (behaviour, subtasks_computed) in subtasks_computed {
        let mean: f64 = subtasks_computed.iter().mean();
        let ci: f64 = subtasks_computed
            .into_iter()
            .confidence_interval_for_mean(0.99);

        if !(mean.is_nan() || ci.is_nan()) {
            println!("\t{} => {:.3} +/- {:.3}", behaviour, mean, ci);
        }
    }

    println!("\nRatio of mean revenue of dishonest and honest providers");
    let mean_revenue_ratio: f64 = revenue_ratio.iter().mean();
    let ci_revenue_ratio: f64 = revenue_ratio.iter().confidence_interval_for_mean(0.99);
    println!("\t{:.3} +/- {:.3}", mean_revenue_ratio, ci_revenue_ratio);

    let plot_helper = |map: BTreeMap<provider::Behaviour, BTreeMap<String, Vec<f64>>>,
                       y_label,
                       title,
                       output_file,
                       output_path: &Path| {
        println!("\n{}", y_label);

        for (behaviour, values) in map {
            println!("\t{}", behaviour);

            let mut mean_ci: BTreeMap<String, (f64, f64)> = BTreeMap::new();
            for (key, avg_values) in values {
                let mean: f64 = avg_values.iter().filter(|x| !x.is_nan()).mean();
                let ci: f64 = avg_values
                    .iter()
                    .filter(|x| !x.is_nan())
                    .confidence_interval_for_mean(0.99);

                if !(mean.is_nan() || ci.is_nan()) {
                    println!("\t\t{} => {:.9} +/- {:.9}", key, mean, ci);
                }

                mean_ci.insert(key, (mean, ci));
            }

            let output_file = &(behaviour
                .to_string()
                .to_lowercase()
                .trim()
                .replace(" ", "_")
                + "_"
                + output_file);

            plot(
                mean_ci,
                "Usage factor",
                y_label,
                &(behaviour.to_string() + title),
                output_path.join(Path::new(output_file)),
            );
        }
    };

    plot_helper(
        prices,
        "Mean average end price, [GNT / CPU second]",
        ": Mean average end price",
        "mean_average_end_price",
        output_path.as_ref(),
    );

    plot_helper(
        effective_prices,
        "Mean average effective price, [GNT]",
        ": Mean average effective price",
        "mean_average_effective_price",
        output_path.as_ref(),
    );

    plot_helper(
        revenues,
        "Mean average revenue, [GNT]",
        ": Mean average revenue",
        "mean_average_revenue",
        output_path.as_ref(),
    );

    Ok(())
}

fn analyse_requestors<R, P>(mut rdr: csv::Reader<R>, output_path: P) -> Result<(), Box<dyn Error>>
where
    R: Read,
    P: AsRef<Path>,
{
    let mut runs: HashMap<u64, Vec<requestor::Stats>> = HashMap::new();
    for result in rdr.deserialize() {
        let result: requestor::Stats = result?;
        runs.entry(result.run_id).or_insert(Vec::new()).push(result);
    }

    let mut subtasks_cancelled: BTreeMap<String, Vec<f64>> = BTreeMap::new();
    let mut mean_cost: BTreeMap<String, Vec<f64>> = BTreeMap::new();

    for (_, results) in runs {
        for partition in partition_by(results, &[0.5, 1.5], |r| r.budget_factor) {
            let key = match partition.boundaries() {
                (Some(lower), Some(upper)) => format!("{} - {}", lower, upper),
                (None, Some(upper)) => format!("-inf - {}", upper),
                (Some(lower), None) => format!("{} - inf", lower),
                (None, None) => format!("-inf - inf"),
            };

            mean_cost
                .entry(key.clone())
                .or_insert(Vec::new())
                .push(partition.iter().map(|r| r.mean_cost).mean());

            subtasks_cancelled.entry(key).or_insert(Vec::new()).push(
                partition
                    .into_iter()
                    .map(|r| {
                        let cancelled = r.num_subtasks_cancelled as f64;
                        cancelled / (cancelled + r.num_subtasks_computed as f64) * 100.0
                    })
                    .mean(),
            );
        }
    }

    let plot_helper = |map: BTreeMap<String, Vec<f64>>, y_label, title, output_path| {
        println!("\n{}", y_label);

        let mut mean_ci: BTreeMap<String, (f64, f64)> = BTreeMap::new();
        for (key, avg_values) in map {
            let mean: f64 = avg_values.iter().filter(|x| !x.is_nan()).mean();
            let ci: f64 = avg_values
                .iter()
                .filter(|x| !x.is_nan())
                .confidence_interval_for_mean(0.99);

            if !(mean.is_nan() || ci.is_nan()) {
                println!("\t{} => {:.9} +/- {:.9}", key, mean, ci);
            }

            mean_ci.insert(key, (mean, ci));
        }

        plot(mean_ci, "Budget factor", y_label, title, output_path);
    };

    plot_helper(
        mean_cost,
        "Mean cost wrt budget in %",
        "Mean cost wrt budget",
        output_path.as_ref().join("mean_cost_wrt_budget"),
    );

    plot_helper(
        subtasks_cancelled,
        "Mean number of subtasks cancelled in %",
        "Mean number of subtasks cancelled",
        output_path
            .as_ref()
            .join("mean_number_of_subtasks_cancelled"),
    );

    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Args = Docopt::new(USAGE)
        .and_then(|d| d.deserialize())
        .unwrap_or_else(|e| e.exit());

    let path = Path::new(&args.arg_csv_file);
    let rdr = csv::Reader::from_path(path)?;

    let output_path = path.parent().unwrap_or(Path::new("."));

    if args.cmd_providers {
        analyse_providers(rdr, output_path)?;
    } else {
        analyse_requestors(rdr, output_path)?;
    }

    Ok(())
}
