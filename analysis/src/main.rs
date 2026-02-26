#![forbid(unsafe_code)]
#![feature(iterator_try_collect, result_option_map_or_default)]

mod examples;
mod formula;
mod load;
mod plot;
mod schema;
mod table;

use anyhow::Result;
use examples::{check_all_tests, fetch_examples};
use load::{
    Database, canton_policy, get_cantonal_rates, get_cantonal_scales, is_married, is_single,
};
use log::{debug, info, trace, warn};
use plot::plot_income_tax;
use schema::{Deductions, OtherDeductions, Rates, Scales, TableType, Target, TaxType};
use std::fs::File;
use std::io::BufReader;
use table::Table;

fn main() -> Result<()> {
    env_logger::init();

    check_data(2010, 2025)?;

    fetch_examples(2010..=2025)?;
    check_all_tests(2010..=2025)?;

    if let Err(e) = Database::new(2010..=2025)?.serialize() {
        warn!("Failed to serialize database: {e:?}");
    }

    for year in [2010, 2015, 2020, 2025] {
        plot_year(year)?;
    }

    for year in [2010, 2025] {
        process_scales(year)?;
    }
    Ok(())
}

fn check_data(start_year: u32, end_year: u32) -> Result<()> {
    for year in start_year..=end_year {
        info!("Validating year {year}...");
        let _: Rates = serde_json::from_reader(BufReader::new(File::open(format!(
            "data/rates-{year}.json"
        ))?))?;
        let _: Scales = serde_json::from_reader(BufReader::new(File::open(format!(
            "data/scales-{year}.json"
        ))?))?;
        let _: Deductions = serde_json::from_reader(BufReader::new(File::open(format!(
            "data/deductions-{year}.json"
        ))?))?;
        let _: OtherDeductions = serde_json::from_reader(BufReader::new(File::open(format!(
            "data/other-deductions-{year}.json"
        ))?))?;
    }

    Ok(())
}

fn plot_year(year: u32) -> Result<()> {
    let cantonal_rates = get_cantonal_rates(year)?;
    let cantonal_scales = get_cantonal_scales(year)?;

    for (canton, cantonal_rate) in cantonal_rates {
        if let Some(cantonal_scale) = cantonal_scales.get(&canton)
            && let Err(e) = plot_income_tax(
                &canton,
                year,
                cantonal_rate,
                *cantonal_scale.splitting,
                &cantonal_scale.single,
                &cantonal_scale.married,
            )
        {
            warn!("Failed to plot {canton} in {year}: {e:?}");
        }
    }

    Ok(())
}

fn process_scales(year: u32) -> Result<()> {
    let cantonal_rates = get_cantonal_rates(year)?;
    debug!("Cantonal rates: {cantonal_rates:?}");

    let scales: Scales = serde_json::from_reader(BufReader::new(File::open(format!(
        "data/scales-{year}.json"
    ))?))?;

    println!("### Federal examples ###");
    println!(
        "     | S | M | split | 10'000 | 20'000 | 50'000 | 100'000 | 200'000 | 10'000 | 20'000 | 50'000 | 100'000 | 200'000 |"
    );
    scales
        .response
        .iter()
        .filter(|scale| {
            scale.tax_type == TaxType::EinkommensSteuer
                && scale.target == Target::Bund
                && scale.location.canton_id == 1
        })
        .try_for_each(|scale| -> Result<()> {
            if let Ok(table) = Table::try_from(scale, canton_policy("CH")?) {
                trace!("Groups: {:?}", scale.group);
                print_table(
                    "CH",
                    is_single(&scale.group),
                    is_married(&scale.group),
                    scale.splitting,
                    table.eval(10_000.0),
                    table.eval(20_000.0),
                    table.eval(50_000.0),
                    table.eval(100_000.0),
                    table.eval(200_000.0),
                    table.eval_split(10_000.0, scale.splitting),
                    table.eval_split(20_000.0, scale.splitting),
                    table.eval_split(50_000.0, scale.splitting),
                    table.eval_split(100_000.0, scale.splitting),
                    table.eval_split(200_000.0, scale.splitting),
                );
            } else {
                println!("| CH | ???");
            }

            Ok(())
        })?;

    println!("### Cantonal examples ###");
    println!(
        "     | S | M | split | 10'000 | 20'000 | 50'000 | 100'000 | 200'000 | 10'000 | 20'000 | 50'000 | 100'000 | 200'000 |"
    );
    for table_type in [
        TableType::Bund,
        TableType::Flattax,
        TableType::Formel,
        TableType::Freiburg,
        TableType::Zuerich,
    ] {
        println!("{:?} table type:", table_type);
        scales
            .response
            .iter()
            .filter(|scale| {
                scale.tax_type == TaxType::EinkommensSteuer
                    && scale.target == Target::Kanton
                    && scale.table_type == table_type
            })
            .try_for_each(|scale| -> Result<()> {
                let cantonal_rate = cantonal_rates.get(&scale.location.canton).unwrap();
                if scale.location.canton != "VS"
                    && let Ok(table) =
                        Table::try_from(scale, canton_policy(&scale.location.canton)?)
                {
                    trace!("Groups: {:?}", scale.group);
                    print_table(
                        &scale.location.canton,
                        is_single(&scale.group),
                        is_married(&scale.group),
                        scale.splitting,
                        table.eval(10_000.0) * cantonal_rate / 100.0,
                        table.eval(20_000.0) * cantonal_rate / 100.0,
                        table.eval(50_000.0) * cantonal_rate / 100.0,
                        table.eval(100_000.0) * cantonal_rate / 100.0,
                        table.eval(200_000.0) * cantonal_rate / 100.0,
                        table.eval_split(10_000.0, scale.splitting) * cantonal_rate / 100.0,
                        table.eval_split(20_000.0, scale.splitting) * cantonal_rate / 100.0,
                        table.eval_split(50_000.0, scale.splitting) * cantonal_rate / 100.0,
                        table.eval_split(100_000.0, scale.splitting) * cantonal_rate / 100.0,
                        table.eval_split(200_000.0, scale.splitting) * cantonal_rate / 100.0,
                    );
                } else {
                    println!("| {} | ???", scale.location.canton);
                }

                Ok(())
            })?;
    }

    Ok(())
}

#[expect(clippy::print_literal, clippy::too_many_arguments)]
fn print_table(
    canton: &str,
    single: bool,
    married: bool,
    splitting: f64,
    single_10000: f64,
    single_20000: f64,
    single_50000: f64,
    single_100000: f64,
    single_200000: f64,
    married_10000: f64,
    married_20000: f64,
    married_50000: f64,
    married_100000: f64,
    married_200000: f64,
) {
    match (single, married) {
        (false, false) => {
            println!(
                "| {} | {} | {} | {:.03} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
                canton,
                if single { "x" } else { " " },
                if married { "x" } else { " " },
                splitting,
                "      ",
                "      ",
                "      ",
                "       ",
                "       ",
                "      ",
                "      ",
                "      ",
                "       ",
                "       ",
            );
        }
        (false, true) => {
            println!(
                "| {} | {} | {} | {:.03} | {} | {} | {} | {} | {} | {:>6.00?} | {:>6.00?} | {:>6.00?} | {:>7.00?} | {:>7.00?} |",
                canton,
                if single { "x" } else { " " },
                if married { "x" } else { " " },
                splitting,
                "      ",
                "      ",
                "      ",
                "       ",
                "       ",
                married_10000,
                married_20000,
                married_50000,
                married_100000,
                married_200000,
            );
        }
        (true, false) => {
            println!(
                "| {} | {} | {} | {:.03} | {:>6.00?} | {:>6.00?} | {:>6.00?} | {:>7.00?} | {:>7.00?} | {} | {} | {} | {} | {} |",
                canton,
                if single { "x" } else { " " },
                if married { "x" } else { " " },
                splitting,
                single_10000,
                single_20000,
                single_50000,
                single_100000,
                single_200000,
                "      ",
                "      ",
                "      ",
                "       ",
                "       ",
            );
        }
        (true, true) => {
            println!(
                "| {} | {} | {} | {:.03} | {:>6.00?} | {:>6.00?} | {:>6.00?} | {:>7.00?} | {:>7.00?} | {:>6.00?} | {:>6.00?} | {:>6.00?} | {:>7.00?} | {:>7.00?} |",
                canton,
                if single { "x" } else { " " },
                if married { "x" } else { " " },
                splitting,
                single_10000,
                single_20000,
                single_50000,
                single_100000,
                single_200000,
                married_10000,
                married_20000,
                married_50000,
                married_100000,
                married_200000,
            );
        }
    }
}
