# touchstone

Touchstone (SNP) parser for RF Engineering — Full N-Port Support

Parse, analyze, and manipulate Touchstone files with any number of ports (1-port, 2-port, 3-port, 4-port, and beyond).

## When To Use This Crate

Use `touchstone` when the task starts from measured or simulated S-parameter
data: `.s1p`, `.s2p`, `.sNp` parsing/writing, plotting, resampling,
reference-impedance metadata, S/Y/Z/ABCD conversion, or two-port network
cascading.

If the task is scalar RF unit math, use `rfconversions`. If it is a block-level
gain/NF/P1dB/IP3 lineup, use `gainlineup`. If it is an end-to-end radio link
question involving EIRP, path loss, C/No, Eb/No, BER, margin, orbit, Doppler,
PFD, or modulation, use `linkbudget`.

S-parameter port indices are 1-indexed: `s_db(2, 1)` is S21. Parsed frequencies
are stored in Hz, and interpolation is performed in real/imaginary space before
rebuilding magnitude/angle or dB/angle views.

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

fn main() -> Result<(), touchstone::TouchstoneError> {
    let ntwk = Network::new("files/ntwk1.s2p")?;

    println!("Ports: {}", ntwk.rank);
    println!("Frequency unit: {}", ntwk.frequency_unit);
    println!("Format: {}", ntwk.format);
    println!("Reference impedance: {} Ω", ntwk.z0);
    println!("Data points: {}", ntwk.f.len());
    Ok(())
}
```

`Network::new` auto-detects the port count, data format, and frequency unit from the file, and
returns I/O or parse errors instead of panicking.

For uploaded data or API endpoints, parse Touchstone content directly from memory. The
`source_name` argument is used as the network name and for `.sNp` extension inference:

```rust
use touchstone::Network;

fn main() -> Result<(), touchstone::TouchstoneError> {
    let body = b"# GHz S RI R 50\n1.0 0.1 0.0 4.0 0.0 0.01 0.0 0.2 0.0\n";
    let ntwk = Network::from_bytes("uploaded.s2p", body)?;

    assert_eq!(ntwk.rank, 2);
    Ok(())
}
```

Non-fatal parser diagnostics are stored in `network.warnings`:

```rust
use touchstone::{Network, TouchstoneWarning};

fn main() -> Result<(), touchstone::TouchstoneError> {
    let ntwk = Network::from_str("uploaded.s1p", "1.0 0.5 0.0\n")?;

    assert!(matches!(
        ntwk.warnings.as_slice(),
        [TouchstoneWarning::MissingOptionLine { .. }]
    ));
    Ok(())
}
```

Touchstone v2 reference impedance metadata is available through
`network.reference_impedance()`. Networks with one scalar reference impedance return
`ReferenceImpedance::Common(z0)`, while files with per-port `[Reference]` values return
`ReferenceImpedance::PerPort(values)`.

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

let ntwk = Network::new("files/ntwk1.s2p")?;

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

For matrix-oriented workflows, use `s_matrix_at(point_index)` to get a stable full S-parameter
matrix for one frequency point. `NetworkPoint` is returned by `sample_at`, and also exposes the
full `SMatrix` at the requested frequency.

### Interpolation and Resampling

`sample_at(frequency_hz, interpolation, extrapolation)` samples a network at one frequency.
`resample(frequencies_hz, interpolation, extrapolation)` returns a new `Network` on a requested
frequency grid. Linear interpolation is performed in real/imaginary space, with magnitude/angle and
dB/angle values rebuilt from the interpolated complex values.

| Item | Description |
|------|-------------|
| `Interpolation::Linear` | Linear interpolation of each real and imaginary component |
| `Interpolation::Nearest` | Select nearest parsed frequency point; ties choose the lower point |
| `Extrapolation::Error` | Error outside the parsed frequency range |
| `Extrapolation::Clamp` | Hold nearest boundary S-parameters at the requested frequency |

### Network Parameter Conversions

For scalar reference impedance networks, stable matrix conversion APIs are available for common RF
and circuit-simulation workflows:

| Item | Description |
|------|-------------|
| `SMatrix::to_y_matrix(z0)` | Convert S-parameters to admittance parameters |
| `SMatrix::to_z_matrix(z0)` | Convert S-parameters to impedance parameters |
| `SMatrix::to_abcd(z0)` | Convert a two-port S matrix to ABCD parameters |
| `SMatrix::try_from_y_matrix(matrix, z0)` | Convert Y parameters back to S-parameters |
| `SMatrix::try_from_z_matrix(matrix, z0)` | Convert Z parameters back to S-parameters |
| `SMatrix::try_from_abcd(matrix, z0)` | Convert ABCD parameters back to a two-port S matrix |
| `network.y_matrix_at(point_index)` | Y matrix for one parsed frequency point |
| `network.z_matrix_at(point_index)` | Z matrix for one parsed frequency point |
| `network.abcd_at(point_index)` | ABCD matrix for one two-port frequency point |

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

let ntwk = Network::new("files/ntwk1.s2p")?;
ntwk.save("output.s2p").unwrap();
```

For generated data, build a network from in-memory matrices and serialize without writing a file:

```rust
use touchstone::{Complex, NetworkBuilder, SMatrix};

let ntwk = NetworkBuilder::new("generated.s1p", 1)
    .point(
        1.0e9,
        SMatrix {
            rank: 1,
            data: vec![vec![Complex { re: 0.5, im: -0.1 }]],
        },
    )
    .build()?;

let touchstone = ntwk.to_touchstone_string()?;
```

---

## 5. Cascading 2-Port Networks

Combine two 2-port networks in series using the ABCD parameter method.
The standard `cascade` connects port 2 of the first network to port 1 of the second:

```rust
use touchstone::Network;

let net1 = Network::new("files/ntwk1.s2p")?;
let net2 = Network::new("files/ntwk2.s2p")?;

let cascaded = net1.cascade(&net2);
println!("Cascaded network has {} data points", cascaded.f.len());
```

For explicit port specification, use `cascade_ports`:

```rust
use touchstone::Network;

let net1 = Network::new("files/ntwk1.s2p")?;
let net2 = Network::new("files/ntwk2.s2p")?;

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

### Diagnostics (Tracing)

`touchstone` uses [`tracing`](https://docs.rs/tracing) for structured, runtime-controllable diagnostics. Set the `RUST_LOG` environment variable to see what the CLI is doing:

```bash
# See file detection, plot generation, and cascade output paths
RUST_LOG=touchstone=info touchstone files/ntwk1.s2p

# See all diagnostics including per-file discovery in directories
RUST_LOG=touchstone=debug touchstone files/data_folder/

# Only warnings and errors (quiet mode)
RUST_LOG=touchstone=warn touchstone files/ntwk1.s2p
```

If you use `touchstone` as a library, install any `tracing` subscriber in your application to capture events. Without a subscriber, all tracing calls are zero-cost no-ops.

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
| `Network::new(path)`          | Parse a Touchstone file and return errors    |
| `Network::from_bytes(name, bytes)` | Parse in-memory UTF-8 Touchstone bytes  |
| `Network::from_str(name, contents)` | Parse an in-memory Touchstone string    |
| `NetworkBuilder::new(name, rank)` | Build generated S-parameter networks     |
| `ReferenceImpedance::Common(z0)` | One scalar reference impedance             |
| `ReferenceImpedance::PerPort(values)` | Per-port Touchstone v2 reference impedances |
| `Complex { re, im }`         | Stable complex value used by public matrices |
| `SMatrix`                    | Stable full S-parameter matrix for one frequency |
| `ParameterMatrix`            | Stable Y- or Z-parameter matrix              |
| `ABCDMatrix`                 | Stable two-port ABCD transmission matrix     |
| `Interpolation`              | `Linear` or `Nearest` sampling policy        |
| `Extrapolation`              | `Error` or `Clamp` out-of-range policy       |
| `network.rank`                | Number of ports                              |
| `network.frequency_unit`      | Frequency unit string                        |
| `network.format`              | Data format (`RI`, `MA`, or `DB`)            |
| `network.z0`                  | Reference impedance (Ω)                      |
| `network.reference_impedance()` | Common or per-port reference metadata      |
| `network.warnings`            | Non-fatal parser diagnostics                 |
| `network.f`                   | Frequency vector (`Vec<f64>`)                |
| `network.f()`                 | Clone of frequency vector                    |
| `network.s_db(j, k)`         | S_jk in dB+angle — `Vec<FrequencyDB>`       |
| `network.s_ri(j, k)`         | S_jk in real+imag — `Vec<FrequencyRI>`       |
| `network.s_ma(j, k)`         | S_jk in mag+angle — `Vec<FrequencyMA>`       |
| `network.s_matrix_at(point_index)` | Full S matrix for one frequency point |
| `network.sample_at(frequency_hz, interpolation, extrapolation)` | Sample at one frequency |
| `network.resample(frequencies_hz, interpolation, extrapolation)` | Return a new frequency grid |
| `network.y_matrix_at(point_index)` | Full Y matrix for one frequency point |
| `network.z_matrix_at(point_index)` | Full Z matrix for one frequency point |
| `network.abcd_at(point_index)` | Two-port ABCD matrix for one frequency point |
| `network.to_touchstone_string()` | Serialize Touchstone text in memory       |
| `network.write_touchstone(writer)` | Write Touchstone text to any writer      |
| `network.save(path)`         | Write network to file                        |
| `network.cascade(&other)`    | Cascade two 2-port networks                  |
| `network.cascade_ports(&other, from, to)` | Cascade with explicit port mapping |
| `network.print_summary()`    | Print metadata to stdout                     |

---

## References

* [Touchstone Wikipedia entry](https://en.wikipedia.org/wiki/Touchstone_file)
* [Touchstone File Format Specification Version 2.1](https://ibis.org/touchstone_ver2.1/touchstone_ver2_1.pdf)
  * Local Version of [Touchstone File Format Specification Version 2.1](https://github.com/iancleary/touchstone/blob/main//docs/touchstone_ver2_1.pdf)
