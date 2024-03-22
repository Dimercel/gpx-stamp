use std::fmt;

use gpx::{Gpx, Track, Waypoint};
use time::{OffsetDateTime, Duration};

use crate::stat::{way_distance, way_durations, max_speed, avg_speed, way_elevations};


#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Activity {
    Cycling,
    Running
}

impl fmt::Display for Activity {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Stamp {
    pub header: Header,
    pub timing: Option<Timing>,
    pub velocity: Option<Velocity>,
    pub elevation: Option<Elevation>,
}


impl From<&Gpx> for Stamp {
    fn from(gpx: &Gpx) -> Stamp {
        let track: &Track = &gpx.tracks[0];
        let way: &Vec<Waypoint> = &track.segments[0].points;

        Stamp {
            header: Header::from(gpx),
            timing: Timing::try_from(way).ok(),
            velocity: Velocity::try_from(way).ok(),
            elevation: Elevation::try_from(way).ok(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Header {
    pub track: Option<String>, // Название трека
    pub date: Option<OffsetDateTime>, // Дата начала активности
    pub activity: Activity, // Тип активности
    pub length: usize, // Протяженность трека в метрах
    pub device: Option<String>, // Идентификатор устройства, создавшего трек
    pub gps_density: usize, // Кол-во GPS-показаний на км пути
}

impl From<&Gpx> for Header {
    fn from(gpx: &Gpx) -> Header {
        let track: &Track = &gpx.tracks[0];
        let way: &Vec<Waypoint> = &track.segments[0].points;
        let date = way[0].time;

        Header {
            track: track.name.clone(),
            date: if date.is_some() { Some(OffsetDateTime::from(date.unwrap())) } else { None },
            activity: Activity::Cycling,
            length: way_distance(way) as usize,
            device: gpx.creator.clone(),
            gps_density: way.len() / (way_distance(way) / 1000.0) as usize,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Timing {
    pub total: Duration,
    pub pure: Duration, // Чистое время, исключая паузы
}

impl TryFrom<&Vec<Waypoint>> for Timing {
    type Error = &'static str;

    fn try_from(way: &Vec<Waypoint>) -> Result<Self, Self::Error> {
        match way_durations(way) {
            Some((total, pure)) => {
                Ok(Timing {
                    total,
                    pure,
                })
            },
            _ => Err("Not correct timing data!"),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Velocity {
    pub average: usize, // Средняя скорость, метров/час
    pub maximum: usize, // Максимальная скорость, метров/час
}

impl TryFrom<&Vec<Waypoint>> for Velocity {
    type Error = &'static str;

    fn try_from(way: &Vec<Waypoint>) -> Result<Self, Self::Error> {
        let average = avg_speed(way);
        let maximum = max_speed(way);

        match (average, maximum) {
            (Some(avg), Some(max)) => {
                Ok(Velocity {
                    average: avg as usize,
                    maximum: max as usize,
                })
            },
            _ => Err("Not correct velocity data!"),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Elevation {
    pub total: usize, // Общий подъем в метрах
    pub maximum: usize, // Максимальный непрерывный подъем в метрах
}

impl TryFrom<&Vec<Waypoint>> for Elevation {
    type Error = &'static str;

    fn try_from(way: &Vec<Waypoint>) -> Result<Self, Self::Error> {
        match way_elevations(way) {
            Some((total, maximum)) => {
                Ok(Elevation {
                    total: total as usize,
                    maximum: maximum as usize,
                })
            },
            _ => Err("Not correct elevation data!"),
        }
    }
}
