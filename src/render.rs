use gpx::Waypoint;
use svg::node::Text as NodeText;
use time::Duration;
use time::format_description::well_known::Iso8601;
use svg::Document;
use svg::node::element::{Line, Path, Rectangle, Text};
use svg::node::element::path::{Data, Command, Parameters, Position, Number};

use crate::stamp::Stamp;


const UNKNOWN_LABEL: &str = "Неизвестно";


fn format_duration(dur: Duration) -> String {
    let hours = dur.whole_hours();
    let minutes = dur.whole_minutes() - (hours * 60);
    let seconds = dur.whole_seconds() - (dur.whole_minutes() * 60);

    format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
}

pub fn to_text(stamp: &Stamp) -> String {
    let unknown = UNKNOWN_LABEL.to_string();

    let head = &stamp.header;
    let mut date = unknown.clone();
    if head.date.is_some() {
        date = head.date.unwrap().format(&Iso8601::DEFAULT).unwrap()
    }

    let head_info = format!("Трек: {} \
                             \nДата(UTC): {} \
                             \nТип активности: {} \
                             \nПротяженность: {:.2} км \
                             \nGPS-показаний на км: {} \
                             \nСоздано: {}",
                            head.track.clone().unwrap_or(unknown.clone()),
                            date,
                            head.activity.to_string(),
                            head.length as f64 / 1000.0,
                            head.gps_density,
                            head.device.clone().unwrap_or(unknown.clone())
    );

    let time = &stamp.timing;
    let mut total_dur = unknown.clone();
    let mut pure_dur = unknown.clone();
    if time.is_some() {
        total_dur = format_duration(time.unwrap().total);
        pure_dur = format_duration(time.unwrap().pure);
    }

    let time_info = format!("\nВремя: \
                             \nОбщее: {} \
                             \nЧистое: {}",
                            total_dur,
                            pure_dur
    );

    let velo = &stamp.velocity;
    let mut avg_speed = unknown.clone();
    let mut max_speed = unknown.clone();
    if velo.is_some() {
        avg_speed = format!("{:.2}", velo.unwrap().average as f64 / 1000.0);
        max_speed = format!("{:.2}", velo.unwrap().maximum as f64 / 1000.0);
    }
    let velo_info = format!("\nСкорость: \
                             \nСредняя: {} км/ч \
                             \nМаксимальная: {} км/ч",
                            avg_speed,
                            max_speed
    );

    let elev = &stamp.elevation;
    let mut total_elev = unknown.clone();
    let mut max_elev = unknown.clone();
    if elev.is_some() {
        total_elev = elev.unwrap().total.to_string();
        max_elev = elev.unwrap().maximum.to_string();
    }
    let elev_info = format!("\nПодъем: \
                             \nОбщий: {} м \
                             \nМаксимальный(непрерывный): {} м",
                            total_elev,
                            max_elev
    );

    format!("{}\n{}\n{}\n{}", head_info, time_info, velo_info, elev_info)
}

fn border_rect(way: &Vec<Waypoint>) -> Option<(f64, f64, f64, f64)> {
    let first = &way[0].point();

    let (mut maxx, mut minx) = (first.x(), first.x());
    let (mut maxy, mut miny) = (first.y(), first.y());

    for p in way {
        let x = p.point().x();
        let y = p.point().y();

        maxx = if x > maxx { x } else { maxx };
        minx = if x < minx { x } else { minx };
        maxy = if y > maxy { y } else { maxy };
        miny = if y < miny { y } else { miny };

    }

    Some((maxx, minx, maxy, miny))
}


fn svg_route(way: &Vec<Waypoint>, width: f64) -> (Data, f64) {
    let (maxx, minx, maxy, miny) = border_rect(way).unwrap();
    let border_width = (maxx - minx).abs();
    let border_height = (maxy - miny).abs();
    let scale_factor = width / border_width;

    let first = &way[0].point();
    let mut pipeline: Vec<Command> = vec![
        Command::Move(
            Position::Absolute,
            Parameters::from(vec![((first.x() - minx) * scale_factor) as Number,
                                  ((first.y() - miny) * scale_factor) as Number]))
    ];
    for p in way {
        let x = ((p.point().x() - minx) * scale_factor) as Number;
        let y = ((p.point().y() - miny) * scale_factor) as Number;

        pipeline.push(Command::Line(Position::Absolute,
                                    Parameters::from(vec![x.clone(), y.clone()])));
    }

    (Data::from(pipeline), border_height * scale_factor)
}

fn svg_elevation(way: &Vec<Waypoint>, width: f64) -> (Data, f64) {
    let first = &way[0];
    let (mut max_elev, mut min_elev) = (first.elevation.unwrap(), first.elevation.unwrap());
    for p in way {
        max_elev = if p.elevation.unwrap() > max_elev { p.elevation.unwrap() } else { max_elev };
        min_elev = if p.elevation.unwrap() < min_elev { p.elevation.unwrap() } else { min_elev };
    }

    let height = width;
    let scale_factor: f64 = height / max_elev;
    let step: f64 = width / way.len() as f64;

    let mut pipeline: Vec<Command> = vec![
        Command::Move(
            Position::Absolute,
            Parameters::from((0.0f64, 0.0f64))
        )
    ];
    let mut step_num = 0;
    for p in way {
        let x = step_num as f64 * step;
        let y = p.elevation.unwrap() as f64 * scale_factor;

        pipeline.push(Command::Line(Position::Absolute,
                                    Parameters::from((x, y))));

        step_num += 1;
    }

    pipeline.push(Command::Line(Position::Absolute, Parameters::from((width, 0.0f64))));
    pipeline.push(Command::Line(Position::Absolute, Parameters::from((0.0f64, 0.0f64))));

    (Data::from(pipeline), width)
}

pub fn to_svg(stamp: &Stamp, way: &Vec<Waypoint>) -> Document {
    let width = 300.0f64;
    let padding = 10.0f64;
    let (way_points, way_height) = svg_route(way, width - padding);
    let (elev_points, elev_height) = svg_elevation(way, width);

    let way_graph = Path::new()
        .set("stroke", "purple")
        .set("stroke-width", 0.8)
        .set("stroke-opacity", 1)
        .set("stroke-linecap", "round")
        .set("stroke-linejoin", "round")
        .set("fill", "none")
        .set("transform", format!("translate({}, {}), scale(1, -1)", padding * 1.5, way_height + padding * 1.5))
        .set("d", way_points);

    let elev_graph = Path::new()
        .set("stroke", "purple")
        .set("stroke-width", 0.8)
        .set("stroke-opacity", 1)
        .set("stroke-linecap", "square")
        .set("stroke-linejoin", "square")
        .set("fill", "purple")
        .set("transform", format!("translate({}, {}), scale(1, -1)", padding, way_height + elev_height + padding * 4.0))
        .set("d", elev_points);

    let document = Document::new()
        .set("viewBox", (0, 0, width + padding * 2.0, width * 2.5))
        // Подложка
        .add(Rectangle::new()
             .set("width", "100%")
             .set("height", "100%")
             .set("fill", "white")
        )
        .add(Rectangle::new()
             .set("x", padding)
             .set("y", padding)
             .set("width", width)
             .set("height", way_height + padding)
             .set("fill", "lavender")
        )
        .add(way_graph)
        .add(Rectangle::new()
             .set("x", padding)
             .set("y", way_height + padding * 3.5)
             .set("width", width)
             .set("height", elev_height + padding * 0.5)
             .set("fill", "lavender")
        )
        .add(elev_graph)
        .add(Text::new()
             .set("x", padding)
             .set("y", padding * 5.5 + way_height + elev_height)
             .set("font-size", "0.6em")
             .set("fill", "black")
             .add(NodeText::new(stamp.header.track.clone().unwrap()))
        )
        .add(Line::new()
             .set("stroke", "grey")
             .set("stroke-width", 0.8)
             .set("stroke-opacity", 0.7)
             .set("x1", padding)
             .set("y1", padding * 6.5 + way_height + elev_height)
             .set("x2", width + padding)
             .set("y2", padding * 6.5 + way_height + elev_height)
        );

    document
}
