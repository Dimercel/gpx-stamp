use geoutils::Location;
use gpx::Waypoint;
use phf::phf_map;
use time::{OffsetDateTime, Duration};


static PAUSE_GAPS: phf::Map<&'static str, (Duration, f64)> = phf_map! {
    "cycling" => (Duration::minutes(2), 5000.0 * (2.0 / 60.0)),
};


pub fn way_distance(way: &Vec<Waypoint>) -> f64 {
    let mut distance: f64 = 0.0;

    if way.len() > 1 {
        for (p1, p2) in way.iter().zip(way[1..].iter()) {
            let from = Location::new(p1.point().y(), p1.point().x());
            let to = Location::new(p2.point().y(), p2.point().x());

            distance += from.distance_to(&to).unwrap().meters()
        }
    }

    distance
}

// Возвращает статистику относительно суммарного подъема, а
// также максимального непрерывного подъема в метрах
pub fn way_elevations(way: &Vec<Waypoint>) -> Option<(f64, f64)> {
    let mut max_elev: f64 = 0.0;
    let mut cur_elev: f64 = 0.0;
    let mut total_elev: f64 = 0.0;

    if way.len() > 1 {
        for (p1, p2) in way.iter().zip(way[1..].iter()) {
            if p2.elevation? >= p1.elevation? {
                let diff = p2.elevation? - p1.elevation?;

                cur_elev += diff;
                total_elev += diff;
            } else {
                max_elev =  if cur_elev > max_elev { cur_elev } else { max_elev };
                cur_elev = 0.0;
            }
        }
    }

    Some((total_elev, max_elev))
}

// Максимальный показатель скорости между двумя
// последовательными gps-показателями
pub fn max_speed(way: &Vec<Waypoint>) -> Option<f64> {
    let mut max_speed: f64 = 0.0;

    for (p1, p2) in way.iter().zip(way[1..].iter()) {
        let from = Location::new(p1.point().y(), p1.point().x());
        let to = Location::new(p2.point().y(), p2.point().x());

        // TODO возможно стоит ввести ограничение на минимальное
        // расстояние между точками для поиска макс. скорости
        let distance = from.distance_to(&to).unwrap().meters();
        let t1: OffsetDateTime = p1.time?.into();
        let t2: OffsetDateTime = p2.time?.into();
        let duration = (t2 - t1).abs().whole_seconds();

        let speed = distance / (duration as f64 / Duration::HOUR.whole_seconds() as f64);

        if speed > max_speed {
            max_speed = speed;
        }
    }

    Some(max_speed)
}

// Средняя скорость прохождения всего пути исключая паузы.
// Показатель измеряется в метры/секунды
pub fn avg_speed(way: &Vec<Waypoint>) -> Option<f64> {
    let start_point = &way[0];
    let finish_point = &way[way.len() - 1];
    let start_time = start_point.time;
    let finish_time = finish_point.time;

    let mut distance: f64 = way_distance(way);

    match (start_time, finish_time) {
        (Some(x), Some(y)) => {
            let st: OffsetDateTime = x.into();
            let ft: OffsetDateTime = y.into();
            let mut clean_duration = Some(ft - st);

            let (dur_gap, dist_gap) = PAUSE_GAPS.get("cycling").unwrap().clone();
            for (dur, _, _) in find_pauses(way, dur_gap, dist_gap) {
                clean_duration = Some(clean_duration.unwrap() - dur);
                distance -= dist_gap;
            }

            let clean_dur_hours = Some(clean_duration.unwrap().whole_seconds() as f64 /
                                   Duration::HOUR.whole_seconds() as f64);

            Some(distance / clean_dur_hours.unwrap())
        },
        _ => None,
    }
}

pub fn find_pauses(
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

pub fn way_durations(way: &Vec<Waypoint>) -> Option<(Duration, Duration)> {
    let start_point = &way[0];
    let finish_point = &way[way.len() - 1];

    let start_time = start_point.time;
    let finish_time = finish_point.time;

    match (start_time, finish_time) {
        (Some(x), Some(y)) => {
            let st: OffsetDateTime = x.into();
            let ft: OffsetDateTime = y.into();

            let total_duration = Some(ft - st);
            let mut clean_duration = total_duration.clone();

            let (dur_gap, dist_gap) = PAUSE_GAPS.get("cycling").unwrap().clone();
            for (dur, _, _) in find_pauses(way, dur_gap, dist_gap) {
                clean_duration = Some(clean_duration.unwrap() - dur);
            }

            Some((total_duration.unwrap(), clean_duration.unwrap()))

        },
        _ => None,
    }
}
