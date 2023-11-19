extern crate gpx;

use std::io::BufReader;
use std::fs::File;
use std::f64::consts::PI;

use clap::Parser;
use euclid::{Angle, Vector2D};
use itertools::izip;
use geoutils::Location;
use gpx::read;
use gpx::{Gpx, Track, Waypoint};
use phf::phf_map;
use svg::Document;
use svg::node::element::Path;
use svg::node::element::path::{Data, Command, Parameters, Position, Number};
use time::{OffsetDateTime, Duration};


const UNKNOWN_LABEL: &str = "Неизвестно";
static PAUSE_GAPS: phf::Map<&'static str, (Duration, f64)> = phf_map! {
    "cycling" => (Duration::minutes(2), 5000.0 * (2.0 / 60.0)),
};


#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to GPX-file
    #[arg()]
    path: String,

    /// Text mode
    #[arg(long, default_value_t = false)]
    text: bool,
}


fn way_distance(way: &Vec<Waypoint>) -> f64 {
    let mut distance: f64 = 0.0;

    for (p1, p2) in way.iter().zip(way[1..].iter()) {
        let from = Location::new(p1.point().y(), p1.point().x());
        let to = Location::new(p2.point().y(), p2.point().x());

        // TODO Не учитывает высоту
        distance += from.distance_to(&to).unwrap().meters()
    }

    distance
}

// Для некоторых задач достаточно приближенной модели gps-трека, которая
// содержит лишь часть показаний оригинальных данных. Основная идея данного
// алгоритма это - выбросить как можно больше точек на относительно прямых участках,
// но при этом сохранить достаточно на изогнутых.
// Параметр angle_mul отвечает за уровень спрямления выходного трека. Чем он больше,
// тем больше будут спрямляться неровности. Определяет угол спрямления в диапозоне [0 .. PI / 2]
// angle_mul = 0.3 => PI/2 * 0.3 = 0.471 радиан(27 градусов)
fn minimize_way(way: &Vec<Waypoint>, angle_mul: f64) -> Vec<Waypoint> {
    if way.len() < 6 { return way.clone() };

    let angle_limit = Angle { radians: PI / 2.0 * angle_mul };
    let prelast = way.len() - 2;
    let zero_vec = Vector2D::new(0.0, 0.0);
    let triple_way = izip!(way[0..prelast].iter(),
                           way[1..prelast].iter(),
                           way[2..prelast].iter());
    let mut angle_gup: f64 = angle_limit.get();
    // Если последовательно применять алгоритм к его же результату, то вторая
    // точка всегда будет выбрасываться, пока в пути не останутся только две точки.
    // Такое поведение нам не нужно.
    // Первая и последняя точки должны обязательно содержаться в результате
    let mut opt_way: Vec<Waypoint> = vec!(way[0].clone(), way[1].clone());

    for (p1, p2, p3) in triple_way {
        let v1: Vector2D<f64, ()> = Vector2D::new(
            p3.point().x() - p1.point().x(),
            p3.point().y() - p1.point().y()
        );

        let v2: Vector2D<f64, ()> = Vector2D::new(
            p3.point().x() - p2.point().x(),
            p3.point().y() - p2.point().y()
        );

        if v1 != zero_vec && v2 != zero_vec {
            let between = v1.angle_to(v2);
            angle_gup -= between.get().abs();

            if angle_gup <= 0.0 {
                opt_way.push(p3.clone());
                angle_gup = angle_limit.get();
            }
        }

    }

    opt_way.push(way[prelast + 1].clone());

    opt_way
}

// Максимальный непрерывный подъем на треке
fn max_elevation(way: &Vec<Waypoint>) -> f64 {
    let mut max_elev: f64 = 0.0;
    let mut cur_elev: f64 = 0.0;

    for (p1, p2) in way.iter().zip(way[1..].iter()) {
        match (p1.elevation, p2.elevation) {
            (Some(e1), Some(e2)) => {
                if e2 >= e1 {
                    cur_elev += e2 - e1
                } else {
                    max_elev =  if cur_elev > max_elev { cur_elev } else { max_elev };
                    cur_elev = 0.0;
                }
            },
            _ => {},
        }

    }

    max_elev
}

// Максимальный показатель скорости между двумя
// последовательными gps-показателями
fn max_speed(way: &Vec<Waypoint>) -> Option<f64> {
    let mut max_speed: f64 = 0.0;

    for (p1, p2) in way.iter().zip(way[1..].iter()) {
        let from = Location::new(p1.point().y(), p1.point().x());
        let to = Location::new(p2.point().y(), p2.point().x());

        // TODO возможно стоит ввести ограничение на минимальное
        // расстояние между точками для поиска макс. скорости
        let distance = from.distance_to(&to).unwrap().meters();
        let mut duration: i64 = 0;
        match (p1.time, p2.time) {
            (Some(tp1), Some(tp2)) => {
                let t1: OffsetDateTime = tp1.into();
                let t2: OffsetDateTime = tp2.into();
                duration = (t2 - t1).abs().whole_seconds();
            },
            _ => {return None},
        }

        let speed = distance / (duration as f64 / Duration::HOUR.whole_seconds() as f64);

        if speed > max_speed {
            max_speed = speed;
        }
    }

    Some(max_speed)
}

fn find_pauses(
    way: &Vec<Waypoint>,
    dur_gap: Duration,
    dist_gap: f64
) -> Vec<(Duration, &Waypoint, &Waypoint)> {
    let mut pauses: Vec<(Duration, &Waypoint, &Waypoint)> = vec!();
    for (p1, p2) in way.iter().zip(way[1..].iter()) {
        match (p1.time, p2.time) {
            (Some(t1), Some(t2)) => {
                let time_point1: OffsetDateTime = t1.into();
                let time_point2: OffsetDateTime = t2.into();
                let pause_duration = (time_point2 - time_point1).abs();

                if  pause_duration >= dur_gap {
                    let pair = vec!(p1.clone(), p2.clone());
                    let dist = way_distance(&pair);

                    if dist <= dist_gap {
                        pauses.push((pause_duration, &p1, &p2));
                    };
                };

            },
            _ => {},
        }
    }

    pauses
}

// TODO переделать на trait в Duration
fn format_duration(dur: Duration) -> String {
    let hours = dur.whole_hours();
    let minutes = dur.whole_minutes() - (hours * 60);
    let seconds = dur.whole_seconds() - (dur.whole_minutes() * 60);

    format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
}

fn main() {
    let args = Args::parse();

    if !args.text {
        println!("На данный момент поддерживается только текстовый режим!");
        return ();
    }

    let file = File::open(args.path);
    if file.is_err() {
        println!("GPX-файл не корректный или не существует!");
        return ();
    }
    let reader = BufReader::new(file.unwrap());

    let gpx: Gpx = read(reader).unwrap();
    let track: &Track = &gpx.tracks[0];

    let way: &Vec<Waypoint> = &track.segments[0].points;
    let opt_way: Vec<Waypoint> = minimize_way(way, 12.0 / 90.0);

    let mut total_elevation: f64 = 0.0;
    for (p1, p2) in way.iter().zip(way[1..].iter()) {
        match (p1.elevation, p2.elevation) {
            (Some(e1), Some(e2)) => {
                if e2 > e1 {
                    total_elevation += e2 - e1;
                }
            },
            _ => {},
        }
    }

    let distance = way_distance(way);
    let start_point = &way[0];
    let finish_point = &way[way.len() - 1];

    let start_time: OffsetDateTime = start_point.time.unwrap().into();
    let finish_time: OffsetDateTime = finish_point.time.unwrap().into();

    let total_duration = finish_time - start_time;
    let mut clean_duration = total_duration;

    let (dur_gap, dist_gap) = PAUSE_GAPS.get("cycling").unwrap().clone();
    for (dur, _, _) in find_pauses(way, dur_gap, dist_gap) {
        clean_duration -= dur;
    }

    let clean_dur_hours = clean_duration.whole_seconds() as f64 /
        Duration::HOUR.whole_seconds() as f64;
    let avg_speed = distance / clean_dur_hours / 1000.0;
    let max_speed = max_speed(way);
    let unknown = UNKNOWN_LABEL.to_string();
    let date = way[0].time;


    println!("Трек: {}", track.name.clone().unwrap_or(unknown.clone()));
    if date.is_some() {
        println!("Дата: {}", date.unwrap().format().unwrap());
    } else {
        println!("Дата: {}", unknown.clone());
    }
    println!("Тип активности: {}", track.type_.clone().unwrap_or(unknown.clone()));
    println!("Протяженность: {:.2} км", distance / 1000.0);
    println!("Создано: {}", gpx.creator.clone().unwrap_or(unknown.clone()));

    println!("\nВремя: \n");
    println!("Общее время: {}", format_duration(total_duration));
    println!("Чистое время: {}", format_duration(clean_duration));

    println!("\nСкорость: \n");
    println!("Средняя скорость: {:.2} км/ч", avg_speed);

    if max_speed.is_some() {
        println!("Макс. скорость: {:.2} км/ч", max_speed.unwrap() / 1000.0);
    } else {
        println!("Макс. скорость: {}", unknown);
    }

    println!("\nПодъем: \n");
    println!("Общий подъем: {:.2} м", total_elevation);
    println!("Максимальный подъем: {:.2} м", max_elevation(way));

    println!("\nGPS-точек на км: {:?}", way.len() / (way_distance(way) / 1000.0) as usize);


    let first = &opt_way[0].point();
    let mut v: Vec<Command> = vec![
        Command::Move(
            Position::Absolute,
            Parameters::from(vec![first.x() as Number * 10.0, first.y() as Number * 10.0]))
    ];
    for p in opt_way {
        let x = p.point().x() as Number * 10.0;
        let y = p.point().y() as Number * 10.0;

        v.push(Command::Line(Position::Absolute, Parameters::from(vec![x.clone(), y.clone()])));
    }
    let data = Data::from(v);

    let path = Path::new()
        .set("fill", "none")
        .set("stroke", "black")
        .set("stroke-width", 0.01)
        .set("stroke-opacity", 1)
        .set("stroke-linecap", "round")
        .set("stroke-linejoin", "round")
        .set("fill", "none")
        .set("d", data);

    let document = Document::new()
        .set("viewBox", (414, 525, 5, 5))
        .add(path);

    svg::save("image.svg", &document).unwrap();
}
