use time::Duration;
use time::format_description::well_known::Iso8601;

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
