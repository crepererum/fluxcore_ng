extern crate csv;

use std::f32;

#[derive(Clone, Copy)]
pub struct Point {
    pub position: [f32; 3],
}


pub struct Column {
    pub name: String,
    pub data: Vec<f32>,
    pub min: f32,
    pub max: f32,
}


impl Column {
    fn new(name: &String) -> Column {
        Column {
            name: name.clone(),
            data: vec![],
            min: f32::INFINITY,
            max: f32::NEG_INFINITY,
        }
    }

    fn push(&mut self, point: f32) {
        self.data.push(point);
        self.min = f32::min(self.min, point);
        self.max = f32::max(self.max, point);
    }
}


fn is_na_string(s: &String) -> bool {
    let lower = s.to_lowercase();
    if lower == "?" {
        true
    } else if lower == "na" {
        true
    } else {
        false
    }
}


pub fn columns_from_file(fname: &String) -> Result<Vec<Column>, String> {
    let mut rdr = match csv::Reader::from_file(fname) {
        Ok(f) => f.has_headers(true),
        Err(_) => {
            return Err(String::from("cannot open file!"));
        }
    };

    let headers = rdr.headers().unwrap();
    let m = headers.len();
    if m < 2 {
        return Err(String::from("we need at least 2 CSV columns"));
    }

    let mut columns = headers.iter().map(|name| {
        Column::new(name)
    }).collect::<Vec<Column>>();

    for (i, row) in rdr.records().enumerate() {
        let row = row.unwrap();
        if row.len() != m {
            return Err(format!("row {} has {} entries but should have {}", i + 1, row.len(), m));
        }

        for (j, cell) in row.iter().enumerate() {
            let value = if is_na_string(cell) {
                f32::NAN
            } else {
                match cell.parse::<f32>() {
                    Ok(v) => v,
                    Err(_) => {
                        return Err(format!("cannot parse column {} in row {}", j + 1, i + 1));
                    }
                }
            };
            columns[j].push(value);
        }
    }

    Ok(columns)
}

pub fn points_from_columns(cols: &Vec<Column>, a: usize, b: usize, c: usize) -> Vec<Point> {
    cols[a].data.iter().zip(cols[b].data.iter()).zip(cols[c].data.iter()).map(|((x, y), z)| {
        Point {
            position: [x.clone(), y.clone(), z.clone()]
        }
    }).collect()
}
