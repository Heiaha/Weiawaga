use crate::evaluation::score::*;
use std::fs::OpenOptions;
use std::io::Write;

pub struct Parameter<'a> {
    values: &'a mut [Score],
    best_values: Vec<Score>,
    pub name: &'a str,
}

impl<'a> Parameter<'a> {
    pub fn new(values: &'a mut [Score], name: &'a str) -> Self {
        Parameter {
            best_values: values.to_vec(),
            values,
            name,
        }
    }

    pub fn update(&mut self, i: usize, step: Value, phase: usize) {
        let score = self.best_values[i];
        let mg = score.mg();
        let eg = score.eg();

        self.values[i] = if phase == Self::MG {
            Score::new(mg + step, eg)
        } else {
            Score::new(mg, eg + step)
        }
    }

    pub fn mark_best(&mut self) {
        self.best_values = self.values.to_vec();
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn print_and_save(&self) {
        let mut file = OpenOptions::new()
            .write(true)
            .append(true)
            .open("results.txt")
            .unwrap();
        let mut output = "Name: ".to_owned() + self.name + &*"\n".to_owned();
        for (i, value) in self.best_values.iter().enumerate() {
            output += &*format!("S!({:>4}, {:>4}), ", value.mg(), value.eg());
            if (i + 1) % 8 == 0 {
                output += "\n"
            }
        }
        output += "\n";
        println!("{}", output);
        file.write_all(output.as_bytes());
    }
}

impl<'a> Parameter<'a> {
    pub const MG: usize = 0;
    pub const EG: usize = 1;
}
