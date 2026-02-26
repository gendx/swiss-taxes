use crate::Table;
use anyhow::Result;
use log::{debug, info};
use plotters::prelude::*;
use std::fs;

pub fn plot_income_tax(
    canton: &str,
    year: u32,
    cantonal_rate: f64,
    splitting: f64,
    table_single: &Table,
    table_married: &Table,
) -> Result<()> {
    if canton != "VS" {
        plot_income_diff_png(
            canton,
            year,
            cantonal_rate,
            splitting,
            table_single,
            table_married,
        )?;
    }
    Ok(())
}

fn plot_income_diff_png(
    canton: &str,
    year: u32,
    cantonal_rate: f64,
    splitting: f64,
    table_single: &Table,
    table_married: &Table,
) -> Result<()> {
    info!("Creating plot for {canton} in {year} (rate={cantonal_rate}, split={splitting})");
    debug!("Single table: {table_single:?}");
    debug!("Married table: {table_married:?}");
    fs::create_dir_all("plots")?;

    let path = format!("plots/income-diff-{canton}-{year}.png");
    let root = BitMapBackend::new(&path, (1000, 900)).into_drawing_area();
    root.fill(&WHITE)?;

    let (chart_area, legend_area) = root.split_horizontally(900);

    let max_salary = 500_000;
    let mut chart = ChartBuilder::on(&chart_area)
        .margin(50)
        .x_label_area_size(60)
        .y_label_area_size(60)
        .build_cartesian_2d(0..max_salary, 0..max_salary)?;

    chart
        .configure_mesh()
        .disable_mesh()
        .label_style(("sans-serif", 22))
        .x_labels(6)
        .y_labels(6)
        .x_desc("Taxable income (person 1)")
        .y_desc("Person 2")
        .axis_desc_style(("sans-serif", 26))
        .draw()?;

    let plotting_area = chart.plotting_area().strip_coord_spec();

    let (range_x, range_y) = plotting_area.get_pixel_range();
    let x_len = range_x.end - range_x.start;
    let y_len = range_y.end - range_y.start;

    let mut min: f64 = -10.0;
    let mut max: f64 = 10.0;
    for i in 0..x_len {
        let x = (max_salary as f64 * i as f64) / x_len as f64;
        for j in 0..y_len {
            let y = (max_salary as f64 * j as f64) / y_len as f64;

            let diff = get_diff(x, y, cantonal_rate, splitting, table_single, table_married);
            if diff.is_nan() {
                panic!("NaN in get_color({x}, {y}, {cantonal_rate}, {splitting}): diff={diff}");
            } else {
                min = min.min(diff);
                max = max.max(diff);
            }

            plotting_area.draw_pixel((i, y_len - j - 1), &colorize(diff))?;
        }
    }

    let mut legend = ChartBuilder::on(&legend_area)
        .caption("Tax diff.", ("sans-serif", 26))
        .margin_right(25)
        .margin_top(200)
        .margin_bottom(200)
        .y_label_area_size(25)
        .x_label_area_size(25)
        .build_cartesian_2d(0..100, min.round() as i32..max.round() as i32)?;
    legend
        .configure_mesh()
        .disable_mesh()
        .disable_x_axis()
        .label_style(("sans-serif", 22))
        .draw()?;
    let plotting_area = legend.plotting_area().strip_coord_spec();

    let (range_x, range_y) = plotting_area.get_pixel_range();
    let x_len = range_x.end - range_x.start;
    let y_len = range_y.end - range_y.start;

    for j in 0..y_len {
        let salary = (max - min) * j as f64 / y_len as f64 + min;
        for i in 0..x_len {
            plotting_area.draw_pixel((i, y_len - j - 1), &colorize(salary))?;
        }
    }

    root.present()?;

    Ok(())
}

fn get_diff(
    x: f64,
    y: f64,
    cantonal_rate: f64,
    splitting: f64,
    table_single: &Table,
    table_married: &Table,
) -> f64 {
    let tax_married = table_married.eval_split(x + y, splitting);
    let tax_singles = table_single.eval(x) + table_single.eval(y);
    (tax_singles - tax_married) * cantonal_rate / 100.0
}

fn colorize(diff: f64) -> RGBColor {
    if (-10.0..=10.0).contains(&diff) {
        RGBColor(0xc0, 0xc0, 0xc0)
    } else if (-3000.0..=-10.0).contains(&diff) {
        interpolate(
            RGBColor(0xc0, 0xa0, 0xa0),
            RGBColor(0xe0, 0xa0, 0x00),
            -10.0,
            -3000.0,
            diff,
        )
    } else if (-8000.0..=-3000.0).contains(&diff) {
        interpolate(
            RGBColor(0xe0, 0xa0, 0x00),
            RGBColor(0xc0, 0x40, 0x40),
            -3000.0,
            -8000.0,
            diff,
        )
    } else if diff < 0.0 {
        interpolate(
            RGBColor(0xc0, 0x40, 0x40),
            RGBColor(0xa0, 0x60, 0x80),
            -8000.0,
            -12000.0,
            diff,
        )
    } else if (10.0..=3000.0).contains(&diff) {
        interpolate(
            RGBColor(0x80, 0xa0, 0xc0),
            RGBColor(0x20, 0xa0, 0xa0),
            10.0,
            3000.0,
            diff,
        )
    } else if (3000.0..=6000.0).contains(&diff) {
        interpolate(
            RGBColor(0x20, 0xa0, 0xa0),
            RGBColor(0x80, 0xc0, 0x00),
            3000.0,
            6000.0,
            diff,
        )
    } else if (6000.0..=9000.0).contains(&diff) {
        interpolate(
            RGBColor(0x80, 0xc0, 0x00),
            RGBColor(0x40, 0xc0, 0x00),
            6000.0,
            9000.0,
            diff,
        )
    } else if diff > 0.0 {
        interpolate(
            RGBColor(0x40, 0xc0, 0x00),
            RGBColor(0x50, 0xa0, 0x60),
            9000.0,
            12000.0,
            diff,
        )
    } else {
        panic!("NaN in colorize(diff={diff})");
    }
}

fn interpolate(color1: RGBColor, color2: RGBColor, start: f64, end: f64, value: f64) -> RGBColor {
    let x = (value - start) / (end - start);
    if x <= 0.0 {
        color1
    } else if x >= 1.0 {
        color2
    } else {
        RGBColor(
            (color1.0 as f64 * (1.0 - x) + color2.0 as f64 * x) as u8,
            (color1.1 as f64 * (1.0 - x) + color2.1 as f64 * x) as u8,
            (color1.2 as f64 * (1.0 - x) + color2.2 as f64 * x) as u8,
        )
    }
}
