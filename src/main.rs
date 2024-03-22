extern crate gpx;

use std::io::BufReader;
use std::fs::File;
use std::f64::consts::PI;

use clap::Parser;
use euclid::{Angle, Vector2D};
use itertools::izip;
use gpx::read;
use gpx::{Gpx, Waypoint};
// use svg::Document;
// use svg::node::element::Path;
// use svg::node::element::path::{Data, Command, Parameters, Position, Number};
use time::Duration;
use time::format_description::well_known::Iso8601;

use crate::stamp::Stamp;

pub mod stat;
pub mod stamp;


const UNKNOWN_LABEL: &str = "Неизвестно";

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


// Для некоторых задач достаточно приближенной модели gps-трека, которая
// содержит лишь часть показаний оригинальных данных. Основная идея данного
// алгоритма это - выбросить как можно больше точек на относительно прямых участках,
// которые не сильно влияют на геометрию трека, но при этом сохранить достаточно на изогнутых.
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
    let stamp = Stamp::from(&gpx);
    // let opt_way: Vec<Waypoint> = minimize_way(way, 12.0 / 90.0);

    let unknown = UNKNOWN_LABEL.to_string();

    let head = &stamp.header;
    let mut date = unknown.clone();
    if head.date.is_some() {
        date = head.date.unwrap().format(&Iso8601::DEFAULT).unwrap()
    }
    println!("Трек: {}", head.track.clone().unwrap_or(unknown.clone()));
    println!("Дата(UTC): {}", date);
    println!("Тип активности: {}", head.activity.to_string());
    println!("Протяженность: {:.2} км", head.length as f64 / 1000.0);
    println!("Создано: {}", head.device.clone().unwrap_or(unknown.clone()));

    println!("\nВремя:");
    let time = &stamp.timing;
    let mut total_dur = unknown.clone();
    let mut pure_dur = unknown.clone();
    if time.is_some() {
        total_dur = format_duration(time.unwrap().total);
        pure_dur = format_duration(time.unwrap().pure);
    }
    println!("Общее: {}", total_dur);
    println!("Чистое: {}", pure_dur);

    println!("\nСкорость:");
    let velo = &stamp.velocity;
    let mut avg_speed = unknown.clone();
    let mut max_speed = unknown.clone();
    if velo.is_some() {
        avg_speed = format!("{:.2}", velo.unwrap().average as f64 / 1000.0);
        max_speed = format!("{:.2}", velo.unwrap().maximum as f64 / 1000.0);
    }
    println!("Средняя: {} км/ч", avg_speed);
    println!("Максимальная: {} км/ч", max_speed);

    println!("\nПодъем:");
    let elev = &stamp.elevation;
    let mut total_elev = unknown.clone();
    let mut max_elev = unknown.clone();
    if elev.is_some() {
        total_elev = elev.unwrap().total.to_string();
        max_elev = elev.unwrap().maximum.to_string();
    }
    println!("Общий: {} м", total_elev);
    println!("Максимальный: {} м", max_elev);

    println!("\nGPS-показаний на км: {:?}", head.gps_density);


    // let first = &opt_way[0].point();
    // let mut v: Vec<Command> = vec![
    //     Command::Move(
    //         Position::Absolute,
    //         Parameters::from(vec![first.x() as Number * 10.0, first.y() as Number * 10.0]))
    // ];
    // for p in opt_way {
    //     let x = p.point().x() as Number * 10.0;
    //     let y = p.point().y() as Number * 10.0;

    //     v.push(Command::Line(Position::Absolute, Parameters::from(vec![x.clone(), y.clone()])));
    // }
    // let data = Data::from(v);

    // let path = Path::new()
    //     .set("fill", "none")
    //     .set("stroke", "black")
    //     .set("stroke-width", 0.01)
    //     .set("stroke-opacity", 1)
    //     .set("stroke-linecap", "round")
    //     .set("stroke-linejoin", "round")
    //     .set("fill", "none")
    //     .set("d", data);

    // let document = Document::new()
    //     .set("viewBox", (414, 525, 5, 5))
    //     .add(path);

    // svg::save("image.svg", &document).unwrap();
}
