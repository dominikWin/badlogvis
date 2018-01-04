macro_rules! error {
    ($fmt:expr) => {
        println!(concat!("{}: ", $fmt), "error".bold().red());
        std::process::exit(1);
    };
    ($fmt:expr, $($arg:tt)*) => {
        println!(concat!("{}: ", $fmt), "error".bold().red(), $($arg)*);
        std::process::exit(1);
    };
}

macro_rules! warning {
    ($fmt:expr) => {
        println!(concat!("{}: ", $fmt), "warning".bold().yellow());
    };
    ($fmt:expr, $($arg:tt)*) => {
        println!(concat!("{}: ", $fmt), "warning".bold().yellow(), $($arg)*);
    };
}

pub fn split_name(name: &str) -> (String, String) {
    let mut parts: Vec<&str> = name.split("/").collect();

    assert!(parts.len() > 0);

    if parts.len() == 1 {
        return ("".to_string(), parts[0].to_string());
    }

    let base = parts.pop().unwrap().to_string();
    let folder = parts.join("/");

    (folder, base)
}

pub fn fake_x_axis(data: Vec<f64>) -> Vec<(f64, f64)> {
    let mut points = Vec::with_capacity(data.len());
    let mut x = 0;
    for point in data {
        points.push((x as f64, point));
        x += 1;
    }
    points
}