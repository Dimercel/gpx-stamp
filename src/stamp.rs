use time::{OffsetDateTime, Duration};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Activity {
    Cycling,
    Running
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Stamp {
    header: Header,
    timing: Option<Timing>,
    velocity: Option<Velocity>,
    elevation: Option<Elevation>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Header {
    track: String, // Название трека
    date: OffsetDateTime, // Дата начала активности
    activity: Activity, // Тип активности
    length: usize, // Протяженность трека в метрах
    device: String, // Идентификатор устройства, создавшего трек
    gps_density: usize, // Кол-во GPS-показаний на км пути
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Timing {
    total: Duration,
    pure: Duration, // Чистое время, исключая паузы
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Velocity {
    average: usize, // Средняя скорость, метров/час
    maximum: usize, // Максимальная скорость, метров/час
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Elevation {
    total: usize, // Общий подъем в метрах
    maximum: usize, // Максимальный непрерывный подъем в метрах
}
