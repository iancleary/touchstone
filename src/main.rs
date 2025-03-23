use std::env;
use std::process;
use touchstone::parser;


fn main() {
  let args: Vec<String> = env::args().collect();

  let config = Config::build(&args).unwrap_or_else(|err| {
    println!("Problem parsing arguments: {err}");
    process::exit(1);
  });

  println!("In file {}", config.file_path);

  run(config.file_path);

}

struct Config {
    file_path: String,
}

#[derive(Debug)]
struct OptionLine {
  frequency_unit: String,
  parameter: String,
  format: String,
  resistance_string: String, // "R"
  reference_resistance: String,  // If "R" is not present, this is 50
}

impl Config {
  fn build(args: &[String]) -> Result<Config, &'static str> {
      if args.len() < 2 {
          return Err("not enough arguments");
      }
      let file_path = args[1].clone();

      Ok(Config { file_path })
  }
}

#[derive(Debug)]
struct Options {
  frequency_unit: [String; 4],
  parameter: [String; 3],
  format: [String; 3],
  resistance_string: String, // "R"
}

fn run(file_path: String) {
  let contents = parser::read_file(file_path);

  // Though specific cases are used for the units above and throughout this specification (e.g., “kHz”), 
  // Touchstone files are case-insensitive. 
  let options = Options {
    frequency_unit: ["Hz", "kHz", "MHz", "GHz"].map(|x| { x.to_string() }),
    parameter: ["S", "Y", "Z"].map(|x| { x.to_string() }),
    format: ["DB", "MA", "RI"].map(|x| { x.to_string() }),
    resistance_string:  "R".to_string(),
  };

  let case_insensitive_options = Options {
    frequency_unit: options.frequency_unit.map(|x| { x.to_uppercase() }),
    parameter: options.parameter.map(|x| { x.to_uppercase() }),
    format: options.format.map(|x| { x.to_uppercase() }),
    resistance_string: options.resistance_string
  };


  // ["foo", "bar"].contains(&test_string) 

  // println!("With text:\n{contents}");

  for line in contents.lines() {
    // println!("\nWith line: {line}");
    // println!("\nWith line: ");
    if line.starts_with("#") {
      println!("Header: {line}");
      let parts = line.split_whitespace().collect::<Vec<_>>();

      
      let header = OptionLine {
        // parts[0] is "#"
        frequency_unit: parts[1].to_string(),
        parameter: parts[2].to_string(),
        format: parts[3].to_string(),
        resistance_string: parts[4].to_string(),
        reference_resistance: parts[5].to_string(),
      };
      print!("{:?}", header);
    }

    // for part in line.split_whitespace() {
    //   print!("{part},")
      
    // }
  }
}