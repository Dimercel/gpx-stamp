extern crate gpx;

use std::env;
use std::io::BufReader;
use std::fs::File;

use euclid::{Angle, Vector2D};
use itertools::izip;
use geoutils::Location;
use gpx::read;
use gpx::{Gpx, Track, Waypoint};
use svg::Document;
use svg::node::element::Path;
use svg::node::element::path::{Data, Command, Parameters, Position, Number};
use time::{OffsetDateTime, Duration};

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

fn minimize_way(way: &Vec<Waypoint>, angle_limit: Angle<f64>) -> Vec<Waypoint> {
    if way.len() < 6 { return way.clone() };

    let mut opt_way: Vec<Waypoint> = vec!(way[0].clone(), way[1].clone());
    let prelast = way.len() - 2;
    let mut angle_gup: f64 = angle_limit.get();
    let zero_vec = Vector2D::new(0.0, 0.0);

    // Первая и последняя точка не должны участвовать в
    // алгоритме, но всегда должны присутствовать в результате
    let triple_way = izip!(way[0..prelast].iter(),
                           way[1..prelast].iter(),
                           way[2..prelast].iter());

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

    opt_way.push(way[prelast + 1].to_owned());

    opt_way
}

fn find_pauses(way: &Vec<Waypoint>) -> Vec<(Duration, &Waypoint, &Waypoint)> {
    let mut pauses: Vec<(Duration, &Waypoint, &Waypoint)> = vec!();
    for (p1, p2) in way.iter().zip(way[1..].iter()) {
        match (p1.time, p2.time) {
            (Some(t1), Some(t2)) => {
                let time_point1: OffsetDateTime = t1.into();
                let time_point2: OffsetDateTime = t2.into();
                let pause_duration = (time_point2 - time_point1).abs();

                // TODO порог в 5 минут требует коррекции в
                // зависимости от типа активности
                if  pause_duration > 5 * Duration::MINUTE {
                    pauses.push((pause_duration, &p1, &p2));
                }

            },
            _ => {},
        }
    }

    pauses
}

fn format_duration(dur: Duration) -> String {
    let hours = dur.whole_hours();
    let minutes = dur.whole_minutes() - (hours * 60);
    let seconds = dur.whole_seconds() - (dur.whole_minutes() * 60);

    format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let file = File::open(&args[1]).unwrap();
    let reader = BufReader::new(file);

    let gpx: Gpx = read(reader).unwrap();
    let track: &Track = &gpx.tracks[0];

    let way: &Vec<Waypoint> = &track.segments[0].points;
    let opt_way: Vec<Waypoint> = minimize_way(way, Angle { radians: 0.2 });

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

    let start_point = &way[0];
    let finish_point = &way[way.len() - 1];

    let start_time: OffsetDateTime = start_point.time.unwrap().into();
    let finish_time: OffsetDateTime = finish_point.time.unwrap().into();

    let total_duration = finish_time - start_time;
    let mut clean_duration = total_duration;

    for (dur, _, _) in find_pauses(way) {
        clean_duration -= dur;
    }


    println!("Название: {:?}", track.name.clone().unwrap_or("Неизвестно".to_string()));
    println!("Протяженность: {:.2} км", way_distance(way) / 1000.0);
    println!("Общий подъем: {:.2} м", total_elevation);
    println!("GPS-точек на км: {:?}", way.len() / (way_distance(way) / 1000.0) as usize);
    println!("Общее время: {}", format_duration(total_duration));
    println!("Чистое время: {}", format_duration(clean_duration));

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
