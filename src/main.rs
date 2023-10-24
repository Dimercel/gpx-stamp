extern crate gpx;

use std::env;
use std::io::BufReader;
use std::fs::File;

use euclid::{Angle, Vector2D};
use itertools::izip;
use geoutils::Location;
use gpx::read;
use gpx::{Gpx, Track, TrackSegment, Waypoint};
use svg::Document;
use svg::node::element::Path;
use svg::node::element::path::{Data, Command, Parameters, Position, Number};

fn segment_distance(segment: &TrackSegment) -> f64 {
    let mut distance: f64 = 0.0;

    for (p1, p2) in segment.points.iter().zip(segment.points[1..].iter()) {
        let from = Location::new(p1.point().y(), p1.point().x());
        let to = Location::new(p2.point().y(), p2.point().x());

        // TODO Не учитывает высоту
        distance += from.distance_to(&to).unwrap().meters()
    }

    distance
}

fn minimize_segment(segment: &TrackSegment, angle_limit: Angle<f64>) -> TrackSegment {
    let mut min_seg: TrackSegment = TrackSegment::new();
    let mut angle_gup: f64 = angle_limit.get();
    let points = &segment.points;

    min_seg.points.push(points[0].to_owned());

    for (p1, p2, p3) in izip!(points.iter(), points[1..].iter(), points[2..].iter()) {
        let v1: Vector2D<f64, ()> = Vector2D::new(
            p3.point().x() - p1.point().x(),
            p3.point().y() - p1.point().y()
        );

        let v2: Vector2D<f64, ()> = Vector2D::new(
            p3.point().x() - p2.point().x(),
            p3.point().y() - p2.point().y()
        );

        let between = v1.angle_to(v2);
        angle_gup -= between.get().abs();

        if angle_gup <= 0.0 {
            min_seg.points.push(p3.to_owned());
            angle_gup = angle_limit.get();
        }
    }

    min_seg
}

fn main() {
    let args: Vec<String> = env::args().collect();
    // This XML file actually exists — try it for yourself!
    let file = File::open(&args[1]).unwrap();
    let reader = BufReader::new(file);

    // read takes any io::Read and gives a Result<Gpx, Error>.
    let gpx: Gpx = read(reader).unwrap();

    // Each GPX file has multiple "tracks", this takes the first one.
    let track: &Track = &gpx.tracks[0];

    // Each track will have different segments full of waypoints, where a
    // waypoint contains info like latitude, longitude, and elevation.
    let segment: &TrackSegment = &minimize_segment(&track.segments[0], Angle { radians: 0.2 });
    // let segment: &TrackSegment = &track.segments[0];

    println!("Название: {:?}", track.name.clone().unwrap_or("Неизвестно".to_string()));
    println!("Протяженность: {:.2} км", segment_distance(segment) / 1000.0);
    println!("Количество точек пути: {:?}", segment.points.len());

    let first = &segment.points[0].point();
    let mut v: Vec<Command> = vec![
        Command::Move(
            Position::Absolute,
            Parameters::from(vec![first.x() as Number * 10.0, first.y() as Number * 10.0]))
    ];
    for p in &segment.points {
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
