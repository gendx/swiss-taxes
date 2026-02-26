use crate::table::Table;
use plotters::prelude::*;
use plotters_canvas::CanvasBackend;
use wasm_bindgen::JsValue;
use web_sys::{HtmlCanvasElement, console};

pub fn plot_income_tax_diff(
    canvas: HtmlCanvasElement,
    max_salary: i32,
    cantonal_rate: f64,
    splitting: f64,
    table_single: &Table,
    table_married: &Table,
) -> Result<(), JsValue> {
    let backend = CanvasBackend::with_canvas_object(canvas).ok_or("Failed to create backend")?;

    let root = backend.into_drawing_area();
    root.fill(&WHITE)
        .map_err(|e| format!("Failed to clear background: {e:?}"))?;

    let (width, height) = root.dim_in_pixel();
    let (chart_area, legend_area) = root.split_horizontally(width - 100);

    let mut chart = ChartBuilder::on(&chart_area)
        .margin(50)
        .margin_left(80)
        .x_label_area_size(20)
        .y_label_area_size(20)
        .build_cartesian_2d(0..max_salary, 0..max_salary)
        .map_err(|e| format!("Failed to create chart: {e:?}"))?;

    chart
        .configure_mesh()
        .disable_mesh()
        .label_style(("sans-serif", 22))
        .x_labels(6)
        .y_labels(6)
        .draw()
        .map_err(|e| format!("Failed to draw mesh: {e:?}"))?;

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
                console::error_1(&JsValue::from_str(&format!(
                    "NaN in get_color({x}, {y}, {cantonal_rate}, {splitting}): diff={diff}"
                )));
            } else {
                min = min.min(diff);
                max = max.max(diff);
            }

            plotting_area
                .draw_pixel((i, y_len - j - 1), &colorize(diff))
                .map_err(|e| format!("Failed to draw pixel: {e:?}"))?;
        }
    }

    let vertical_margin = height / 5;
    let mut legend = ChartBuilder::on(&legend_area)
        .caption("Tax diff.", ("sans-serif", 26))
        .margin_right(25)
        .margin_top(vertical_margin)
        .margin_bottom(vertical_margin)
        .y_label_area_size(30)
        .x_label_area_size(0)
        .build_cartesian_2d(0..100, min.round() as i32..max.round() as i32)
        .map_err(|e| format!("Failed to create chart: {e:?}"))?;
    legend
        .configure_mesh()
        .disable_mesh()
        .disable_x_axis()
        .label_style(("sans-serif", 22))
        .draw()
        .map_err(|e| format!("Failed to draw mesh: {e:?}"))?;
    let plotting_area = legend.plotting_area().strip_coord_spec();

    let (range_x, range_y) = plotting_area.get_pixel_range();
    let x_len = range_x.end - range_x.start;
    let y_len = range_y.end - range_y.start;

    for j in 0..y_len {
        let salary = (max - min) * j as f64 / y_len as f64 + min;
        for i in 0..x_len {
            plotting_area
                .draw_pixel((i, y_len - j - 1), &colorize(salary))
                .map_err(|e| format!("Failed to draw pixel: {e:?}"))?;
        }
    }

    root.present()
        .map_err(|e| format!("Failed to present chart: {e:?}"))?;
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
        RGBColor(128, 128, 128)
    } else if diff < 0.0 {
        interpolate(
            RGBColor(192, 0, 0),
            RGBColor(128, 96, 96),
            -5000.0,
            0.0,
            diff,
        )
    } else if diff > 0.0 {
        interpolate(
            RGBColor(96, 128, 96),
            RGBColor(0, 128, 0),
            0.0,
            5000.0,
            diff,
        )
    } else {
        console::error_1(&JsValue::from_str(&format!("NaN in colorize: diff={diff}")));
        RGBColor(0, 0, 0)
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
