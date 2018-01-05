macro_rules! error {
    ($fmt:expr) => {
        use std;
        use colored::*;
        println!(concat!("{}: ", $fmt), "error".bold().red());
        std::process::exit(1);
    };
    ($fmt:expr, $($arg:tt)*) => {
        use std;
        use colored::*;
        println!(concat!("{}: ", $fmt), "error".bold().red(), $($arg)*);
        std::process::exit(1);
    };
}

macro_rules! warning {
    ($fmt:expr) => {
        use colored::*;
        println!(concat!("{}: ", $fmt), "warning".bold().yellow());
    };
    ($fmt:expr, $($arg:tt)*) => {
        use colored::*;
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

pub fn bind_axis(x: Vec<f64>, y: Vec<f64>) -> Vec<(f64, f64)> {
    assert_eq!(x.len(), y.len());
    let mut points = Vec::with_capacity(x.len());
    for i in 0..x.len() {
        points.push((x[i], y[i]));
    }
    points
}

pub fn differention(orig: &Vec<(f64, f64)>) -> Vec<(f64, f64)> {
    let mut out = Vec::with_capacity(orig.len() - 1);
    for i in 0..orig.len() - 1 {
        let (x1, y1) = orig[i];
        let (x2, y2) = orig[i + 1];
        let x = (x1 + x2) / 2f64;
        let slope = (y2 - y1) / (x2 - x1);
        out.push((x, slope));
    }
    out
}

pub fn delta(orig: &Vec<(f64, f64)>) -> Vec<(f64, f64)> {
    let mut out = Vec::with_capacity(orig.len() - 1);
    for i in 0..orig.len() - 1 {
        let (x1, y1) = orig[i];
        let (x2, y2) = orig[i + 1];
        let x = (x1 + x2) / 2f64;
        let delta = y2 - y1;
        out.push((x, delta));
    }
    out
}

pub fn integration(orig: &Vec<(f64, f64)>) -> (Vec<(f64, f64)>, f64) {
    let mut out = Vec::with_capacity(orig.len() - 1);
    let mut total_area = 0f64;
    for i in 1..orig.len() {
        // Trapazoid rule integration
        let (x1, y1) = orig[i - 1];
        let (x2, y2) = orig[i];
        let delta_x = x2 - x1;
        let average = (y1 + y2) / 2f64;
        let area = delta_x * average;

        total_area += area;
        out.push((x1, total_area));
     }
    (out, total_area)
}