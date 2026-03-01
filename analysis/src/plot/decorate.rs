use plotters::element::{Circle, Cross, Drawable, DynElement, IntoDynElement, PointCollection};
use plotters::style::{Color, Palette, Palette99, ShapeStyle, SizeDesc};
use plotters_backend::{BackendCoord, DrawingBackend, DrawingErrorKind};

pub fn make_line_styles() -> [LineStyle; 11] {
    [
        LineStyle {
            color: ShapeStyle::from(Palette99::pick(0).to_rgba()),
            decorator: Decorator::Circle {
                size: 4,
                filled: false,
            },
        },
        LineStyle {
            color: ShapeStyle::from(Palette99::pick(1).to_rgba()),
            decorator: Decorator::Square {
                size: 4,
                filling: SquareFilling::No,
            },
        },
        LineStyle {
            color: ShapeStyle::from(Palette99::pick(3).to_rgba()),
            decorator: Decorator::Triangle {
                size: 6,
                filled: false,
            },
        },
        LineStyle {
            color: ShapeStyle::from(Palette99::pick(4).to_rgba()),
            decorator: Decorator::Cross { size: 4 },
        },
        LineStyle {
            color: ShapeStyle::from(Palette99::pick(5).to_rgba()),
            decorator: Decorator::Circle {
                size: 5,
                filled: true,
            },
        },
        LineStyle {
            color: ShapeStyle::from(Palette99::pick(7).to_rgba()),
            decorator: Decorator::Square {
                size: 5,
                filling: SquareFilling::Filled,
            },
        },
        LineStyle {
            color: ShapeStyle::from(Palette99::pick(10).to_rgba()),
            decorator: Decorator::Triangle {
                size: 6,
                filled: true,
            },
        },
        LineStyle {
            color: ShapeStyle::from(Palette99::pick(12).to_rgba()),
            decorator: Decorator::Star {
                size: 6,
                filled: false,
            },
        },
        LineStyle {
            color: ShapeStyle::from(Palette99::pick(14).to_rgba()),
            decorator: Decorator::Star {
                size: 8,
                filled: true,
            },
        },
        LineStyle {
            color: ShapeStyle::from(Palette99::pick(16).to_rgba()),
            decorator: Decorator::Square {
                size: 4,
                filling: SquareFilling::Cross,
            },
        },
        LineStyle {
            color: ShapeStyle::from(Palette99::pick(18).to_rgba()),
            decorator: Decorator::Square {
                size: 4,
                filling: SquareFilling::Plus,
            },
        },
    ]
}

#[derive(Clone, Copy)]
pub struct LineStyle {
    pub color: ShapeStyle,
    pub decorator: Decorator,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SquareFilling {
    No,
    Filled,
    Plus,
    Cross,
}

#[derive(Clone, Copy)]
pub enum Decorator {
    Circle { size: u32, filled: bool },
    Cross { size: u32 },
    Triangle { size: u32, filled: bool },
    Square { size: u32, filling: SquareFilling },
    Star { size: u32, filled: bool },
}

impl Decorator {
    pub fn decorate<DB, Coord>(
        &self,
        coord: Coord,
        color: impl Into<ShapeStyle>,
    ) -> DynElement<'static, DB, Coord>
    where
        DB: DrawingBackend,
        Coord: Clone + 'static,
    {
        let color = color.into();
        match self {
            Decorator::Circle { size, filled } => {
                Circle::new(coord, *size, if *filled { color.filled() } else { color }).into_dyn()
            }
            Decorator::Cross { size } => Cross::new(coord, *size, color.stroke_width(2)).into_dyn(),
            Decorator::Triangle { size, filled } => {
                Triangle::new(coord, *size, if *filled { color.filled() } else { color }).into_dyn()
            }
            Decorator::Square { size, filling } => Square::new(
                coord,
                *size,
                if let SquareFilling::Filled = filling {
                    color.filled()
                } else {
                    color
                },
                *filling,
            )
            .into_dyn(),
            Decorator::Star { size, filled } => {
                Star::new(coord, *size, if *filled { color.filled() } else { color }).into_dyn()
            }
        }
    }
}

struct Square<Coord, Size: SizeDesc> {
    center: Coord,
    size: Size,
    style: ShapeStyle,
    filling: SquareFilling,
}

impl<Coord, Size: SizeDesc> Square<Coord, Size> {
    fn new<T: Into<ShapeStyle>>(
        coord: Coord,
        size: Size,
        style: T,
        filling: SquareFilling,
    ) -> Self {
        Self {
            center: coord,
            size,
            style: style.into(),
            filling,
        }
    }
}

impl<'a, Coord: 'a, Size: SizeDesc> PointCollection<'a, Coord> for &'a Square<Coord, Size> {
    type Point = &'a Coord;
    type IntoIter = std::iter::Once<&'a Coord>;
    fn point_iter(self) -> std::iter::Once<&'a Coord> {
        std::iter::once(&self.center)
    }
}

impl<Coord, DB: DrawingBackend, Size: SizeDesc> Drawable<DB> for Square<Coord, Size> {
    fn draw<I: Iterator<Item = BackendCoord>>(
        &self,
        mut points: I,
        backend: &mut DB,
        ps: (u32, u32),
    ) -> Result<(), DrawingErrorKind<DB::ErrorType>> {
        if let Some((x, y)) = points.next() {
            let size = self.size.in_pixels(&ps);
            backend.draw_rect(
                (x - size, y - size),
                (x + size, y + size),
                &self.style,
                self.style.filled,
            )?;

            match self.filling {
                SquareFilling::No | SquareFilling::Filled => (),
                SquareFilling::Plus => {
                    backend.draw_line((x - size, y), (x + size, y), &self.style)?;
                    backend.draw_line((x, y - size), (x, y + size), &self.style)?;
                }
                SquareFilling::Cross => {
                    backend.draw_line((x - size, y - size), (x + size, y + size), &self.style)?;
                    backend.draw_line((x + size, y - size), (x - size, y + size), &self.style)?;
                }
            }
        }
        Ok(())
    }
}

struct Triangle<Coord, Size: SizeDesc> {
    center: Coord,
    size: Size,
    style: ShapeStyle,
}

impl<Coord, Size: SizeDesc> Triangle<Coord, Size> {
    fn new<T: Into<ShapeStyle>>(coord: Coord, size: Size, style: T) -> Self {
        Self {
            center: coord,
            size,
            style: style.into(),
        }
    }
}

impl<'a, Coord: 'a, Size: SizeDesc> PointCollection<'a, Coord> for &'a Triangle<Coord, Size> {
    type Point = &'a Coord;
    type IntoIter = std::iter::Once<&'a Coord>;
    fn point_iter(self) -> std::iter::Once<&'a Coord> {
        std::iter::once(&self.center)
    }
}

impl<Coord, DB: DrawingBackend, Size: SizeDesc> Drawable<DB> for Triangle<Coord, Size> {
    fn draw<I: Iterator<Item = BackendCoord>>(
        &self,
        mut points: I,
        backend: &mut DB,
        ps: (u32, u32),
    ) -> Result<(), DrawingErrorKind<DB::ErrorType>> {
        if let Some((x, y)) = points.next() {
            let size = self.size.in_pixels(&ps);
            let points = [-90, -210, -330, -90]
                .iter()
                .map(|deg| f64::from(*deg) * std::f64::consts::PI / 180.0)
                .map(|rad| {
                    (
                        (rad.cos() * f64::from(size) + f64::from(x)).ceil() as i32,
                        (rad.sin() * f64::from(size) + f64::from(y)).ceil() as i32,
                    )
                });
            if self.style.filled {
                backend.fill_polygon(points.into_iter().take(3), &self.style)?;
            } else {
                backend.draw_path(points, &self.style)?;
            }
        }
        Ok(())
    }
}

struct Star<Coord, Size: SizeDesc> {
    center: Coord,
    size: Size,
    style: ShapeStyle,
}

impl<Coord, Size: SizeDesc> Star<Coord, Size> {
    fn new<T: Into<ShapeStyle>>(coord: Coord, size: Size, style: T) -> Self {
        Self {
            center: coord,
            size,
            style: style.into(),
        }
    }
}

impl<'a, Coord: 'a, Size: SizeDesc> PointCollection<'a, Coord> for &'a Star<Coord, Size> {
    type Point = &'a Coord;
    type IntoIter = std::iter::Once<&'a Coord>;
    fn point_iter(self) -> std::iter::Once<&'a Coord> {
        std::iter::once(&self.center)
    }
}

impl<Coord, DB: DrawingBackend, Size: SizeDesc> Drawable<DB> for Star<Coord, Size> {
    fn draw<I: Iterator<Item = BackendCoord>>(
        &self,
        mut points: I,
        backend: &mut DB,
        ps: (u32, u32),
    ) -> Result<(), DrawingErrorKind<DB::ErrorType>> {
        if let Some((x, y)) = points.next() {
            let size = self.size.in_pixels(&ps);
            let points = (0..=10).map(|i| {
                let rad = std::f64::consts::PI * (-0.5 + f64::from(i) / 5.0);
                let scale = if i % 2 == 0 {
                    f64::from(size)
                } else {
                    f64::from(size) / 3.0
                };
                (
                    (rad.cos() * scale + f64::from(x)).round() as i32,
                    (rad.sin() * scale + f64::from(y)).round() as i32,
                )
            });
            if self.style.filled {
                backend.fill_polygon(points.into_iter().take(10), &self.style)?;
            } else {
                backend.draw_path(points, &self.style)?;
            }
        }
        Ok(())
    }
}
