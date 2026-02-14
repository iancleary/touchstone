# touchstone

Touchstone (SNP) parser for RF Engineering — Full N-Port Support

Parse, analyze, and manipulate Touchstone files with any number of ports (1-port, 2-port, 3-port, 4-port, and beyond).

## Installation

```bash
cargo install touchstone
```

This installs an executable in your `.cargo/bin` directory (`.cargo/bin/touchstone`).

---

## 1. What Are Touchstone Files?

Touchstone files (also called **SNP** files) are the industry-standard format for storing
**S-parameter** data measured or simulated for RF and microwave networks.

Each file describes how electromagnetic signals scatter through an N-port network — reflections,
transmissions, and coupling — across a range of frequencies.

The file extension encodes the port count: `.s1p` for a 1-port, `.s2p` for a 2-port, `.s3p`
for a 3-port, and so on up to `.s32p` and beyond.

A typical `.s2p` file looks like this:

```text
! Two-port network measurement
# GHz S RI R 50
1.0  0.5 -0.3  0.1 0.2  0.1 0.2  0.5 -0.3
2.0  0.4 -0.2  0.2 0.1  0.2 0.1  0.4 -0.2
```

The `#` line is the **option line**: it declares the frequency unit (`GHz`), parameter type (`S`),
data format (`RI` = Real-Imaginary), and reference impedance (`R 50` = 50 Ω).

---

## 2. Loading a Network

Use `Network::new` to parse any Touchstone file:

```rust
use touchstone::Network;

let ntwk = Network::new("files/ntwk1.s2p".to_string());

println!("Ports: {}", ntwk.rank);
println!("Frequency unit: {}", ntwk.frequency_unit);
println!("Format: {}", ntwk.format);
println!("Reference impedance: {} Ω", ntwk.z0);
println!("Data points: {}", ntwk.f.len());
```

`Network::new` auto-detects the port count, data format, and frequency unit from the file.

---

## 3. Accessing S-Parameters

S-parameters are accessed with **1-indexed** port numbers, matching the conventional
S₁₁, S₂₁, etc. notation used in RF engineering.

Three accessor methods return a `Vec` over all frequencies:

| Method  | Returns                   | Struct fields                          |
|---------|---------------------------|----------------------------------------|
| `s_db`  | dB magnitude + angle (°)  | `FrequencyDB { frequency, s_db }`      |
| `s_ri`  | Real + imaginary parts    | `FrequencyRI { frequency, s_ri }`      |
| `s_ma`  | Linear magnitude + angle  | `FrequencyMA { frequency, s_ma }`      |

```rust
use touchstone::Network;

let ntwk = Network::new("files/ntwk1.s2p".to_string());

// S11 in dB (return loss)
let s11_db = ntwk.s_db(1, 1);
for point in &s11_db {
    println!("f={} : dB={}, angle={}", point.frequency, point.s_db.decibel(), point.s_db.angle());
}

// S21 in Real-Imaginary
let s21_ri = ntwk.s_ri(2, 1);
for point in &s21_ri {
    println!("f={} : re={}, im={}", point.frequency, point.s_ri.real(), point.s_ri.imaginary());
}

// S21 in Magnitude-Angle
let s21_ma = ntwk.s_ma(2, 1);
for point in &s21_ma {
    println!("f={} : mag={}, angle={}", point.frequency, point.s_ma.magnitude(), point.s_ma.angle());
}
```

### Field Aliases

Each S-parameter data pair struct offers multiple accessors for the same underlying data:

| Struct            | Field aliases                                      |
|-------------------|----------------------------------------------------|
| `RealImaginary`   | `.real()`, `.imaginary()`, `.magnitude()`, `.decibel()`, `.angle()` |
| `DecibelAngle`    | `.decibel()`, `.angle()`, `.magnitude()`, `.real()`, `.imaginary()` |
| `MagnitudeAngle`  | `.magnitude()`, `.angle()`, `.decibel()`, `.real()`, `.imaginary()` |

You can also convert between representations:

| From → To         | Method                                 |
|-------------------|----------------------------------------|
| `RealImaginary`   | `.magnitude_angle()`, `.decibel_angle()` |
| `MagnitudeAngle`  | `.real_imaginary()`, `.decible_angle()` (sic) |
| `DecibelAngle`    | (convert via `RealImaginary::from_decibel_angle`) |

---

## 4. Saving Networks

Save a `Network` back to disk. The writer auto-selects single-line format (1–2 ports) or
multi-line format (3+ ports):

```rust
use touchstone::Network;

let ntwk = Network::new("files/ntwk1.s2p".to_string());
ntwk.save("output.s2p").unwrap();
```

---

## 5. Cascading 2-Port Networks

Combine two 2-port networks in series using the ABCD parameter method.
The standard `cascade` connects port 2 of the first network to port 1 of the second:

```rust
use touchstone::Network;

let net1 = Network::new("files/ntwk1.s2p".to_string());
let net2 = Network::new("files/ntwk2.s2p".to_string());

let cascaded = net1.cascade(&net2);
println!("Cascaded network has {} data points", cascaded.f.len());
```

For explicit port specification, use `cascade_ports`:

```rust
use touchstone::Network;

let net1 = Network::new("files/ntwk1.s2p".to_string());
let net2 = Network::new("files/ntwk2.s2p".to_string());

let cascaded = net1.cascade_ports(&net2, 2, 1);
```

---

## 6. CLI Usage

### File Path

Plot a single Touchstone file:

```bash
touchstone files/ntwk3.s2p
```

Output:

```text
============================
Single file detected. Plotting.
In file: files/ntwk3.s2p
'files/ntwk3.s2p' is a Relative path with separators (nested).
Plot HTML generated at files/ntwk3.s2p.html
You can open the plot in your browser at:
file:///Users/iancleary/Development/touchstone/files/ntwk3.s2p.html
Attempting to open plot in your default browser...
Success! Opening: file:///Users/iancleary/Development/touchstone/files/ntwk3.s2p.html
```

> Works on Windows, macOS, and Linux file systems!

[![HTML file created for ntwk3.s2p by running `touchstone files/ntwk3.s2p` in the root of this directory](https://github.com/iancleary/touchstone/blob/main/examples/ntwk3.s2p.html.png?raw=true)](https://github.com/iancleary/touchstone/tree/main/examples/ntwk3.s2p.html)

You can view the HTML source file itself here: [examples/ntwk3.s2p.html](https://github.com/iancleary/touchstone/tree/main/examples/ntwk3.s2p.html).

### Directory Path

Plot all Touchstone files in a directory:

```bash
touchstone files/
```

Output:

```text
============================
In directory: files/
Directory detected. Plotting all valid network files in directory.
Found network file: "files/ntwk1.s2p"
Found network file: "files/ntwk2.s2p"
Found network file: "files/ntwk3.s2p"
Plot HTML generated at files/combined_plot.html
```

[![HTML file created for the files directory by running `touchstone files/` in the root of this directory](https://github.com/iancleary/touchstone/blob/main/examples/combined_plot.html.png?raw=true)](https://github.com/iancleary/touchstone/tree/main/examples/combined_plot.html)

You can view the HTML source file itself here: [examples/combined_plot.html](https://github.com/iancleary/touchstone/tree/main/examples/combined_plot.html).

### Cascade Command

Cascade two or more 2-port networks from the command line:

```bash
# Cascade two networks
touchstone cascade ntwk1.s2p ntwk2.s2p

# Cascade with custom output name
touchstone cascade ntwk1.s2p ntwk2.s2p --name result.s2p
```

### Full Help

```bash
touchstone --help
```

---

## 7. Supported File Types, Data Formats, and Frequency Units

### File Types

| Extension | Ports | Example use case                       |
|-----------|-------|----------------------------------------|
| `.s1p`    | 1     | Terminations, loads, antennas          |
| `.s2p`    | 2     | Amplifiers, filters, cables            |
| `.s3p`    | 3     | Power dividers, circulators            |
| `.s4p`    | 4     | Differential pairs, couplers           |
| `.sNp`    | N     | Any N-port (tested up to 32-port)      |

### Data Formats

| Code | Name             | Pair values              |
|------|------------------|--------------------------|
| `RI` | Real-Imaginary   | real, imaginary          |
| `MA` | Magnitude-Angle  | linear magnitude, degrees|
| `DB` | Decibel-Angle    | dB magnitude, degrees    |

### Frequency Units

`Hz`, `kHz`, `MHz`, `GHz`, `THz` — all supported with automatic conversion.

---

## 8. API Summary

| Item                          | Description                                  |
|-------------------------------|----------------------------------------------|
| `Network::new(path)`          | Parse a Touchstone file into a `Network`     |
| `network.rank`                | Number of ports                              |
| `network.frequency_unit`      | Frequency unit string                        |
| `network.format`              | Data format (`RI`, `MA`, or `DB`)            |
| `network.z0`                  | Reference impedance (Ω)                      |
| `network.f`                   | Frequency vector (`Vec<f64>`)                |
| `network.f()`                 | Clone of frequency vector                    |
| `network.s_db(j, k)`         | S_jk in dB+angle — `Vec<FrequencyDB>`       |
| `network.s_ri(j, k)`         | S_jk in real+imag — `Vec<FrequencyRI>`       |
| `network.s_ma(j, k)`         | S_jk in mag+angle — `Vec<FrequencyMA>`       |
| `network.save(path)`         | Write network to file                        |
| `network.cascade(&other)`    | Cascade two 2-port networks                  |
| `network.cascade_ports(&other, from, to)` | Cascade with explicit port mapping |
| `network.print_summary()`    | Print metadata to stdout                     |

---

## References

* [Touchstone Wikipedia entry](https://en.wikipedia.org/wiki/Touchstone_file)
* [Touchstone File Format Specification Version 2.1](https://ibis.org/touchstone_ver2.1/touchstone_ver2_1.pdf)
  * Local Version of [Touchstone File Format Specification Version 2.1](https://github.com/iancleary/touchstone/blob/main//docs/touchstone_ver2_1.pdf)
