# touchstone

Touchstone (SNP) parser for RF Engineering - Full N-Port Support

Parse, analyze, and manipulate Touchstone files with any number of ports (1-port, 2-port, 3-port, 4-port, and beyond).

## Installation

```bash
cargo install touchstone
```

This installs an executable in your `.cargo/bin` directory (`.cargo/bin/touchstone`).

## Features

### Full N-Port Support
- **Parse any N-port file**: .s1p, .s2p, .s3p, .s4p, and beyond (tested up to 32-port)
- **Auto-format detection**: Automatically handles single-line (1-2 port) and multi-line (3+ port) formats
- **Save N-port files**: Write networks back to disk with automatic format selection
- **All data formats**: RI (Real-Imaginary), MA (Magnitude-Angle), DB (Decibel-Angle)
- **All frequency units**: Hz, kHz, MHz, GHz, THz
- **Any reference impedance**: 50Ω, 75Ω, or custom values

### Network Operations
- **Cascade 2-port networks**: Combine networks with ABCD parameter method
- **Access S-parameters**: Extract any Sij parameter in any format
- **Interactive plotting**: Generate standalone HTML plots for visualization
- **Batch processing**: Process entire directories of Touchstone files

### Production Ready
- **94 comprehensive tests** with 100% pass rate
- **Zero regressions** - All original functionality preserved
- **Well-documented** - Extensive inline documentation and examples
- **Robust error handling** - Clear, actionable error messages

## Usage

The `touchstone` executable can be run with a file path as an argument or a directory path as an argument.

### Help

```bash
touchstone --help
```

Outputs:

```bash
📡 Touchstone (sNp) file parser, plotter, and more - https://github.com/iancleary/touchstone

VERSION:
    0.10.4

USAGE:
     touchstone <FILE_PATH>
     touchstone <DIRECTORY_PATH>
     touchstone cascade <FILE_1> <FILE_2> ... [--name <OUTPUT_NAME>]

     FILE_PATH: path to a Touchstone file (.s1p, .s2p, .s3p, .s4p, etc.)
     DIRECTORY_PATH: path to a directory containing Touchstone files

     The Touchstone file(s) are parsed and an interactive plot (html file and js/ folder)
     is created next to the source file(s).

OPTIONS:
      -v, --version    Print version information
      -h, --help       Print help information

EXAMPLES:
     # Single file - 2-port (Relative path)
     touchstone files/measurements.s2p

     # Single file - 3-port
     touchstone files/hfss_18.2.s3p

     # Single file - 4-port
     touchstone files/Agilent_E5071B.s4p

     # Directory (Plot all files in folder)
     touchstone files/data_folder

     # Cascade two 2-port networks
     touchstone cascade ntwk1.s2p ntwk2.s2p

     # Cascade with custom output name
     touchstone cascade ntwk1.s2p ntwk2.s2p --name result.s2p

     # Bare filename
     touchstone measurement.s2p

     # Windows absolute path
     touchstone C:\Users\data\measurements.s2p

     # Windows UNC path (network path)
     touchstone \\server\mount\folder\measurement.s2p

     # Unix absolute path
     touchstone /home/user/measurements.s2p
```

### File Path

```bash
touchstone files/ntwk3.s2p
```

which outputs:

```bash
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

> This works on Windows, MacOS, and Linux file systems!

* MacOS: `file:///Users/iancleary/touchstone/files/ntwk3.s2p.html`
* Windows: `file:///C:/Users/iancleary/touchstone/files/ntwk3.s2p.html`
* Linux: `file:///home/iancleary/touchstone/files/ntwk3.s2p.html`

This command created an html file that is interactive, and designed to not have any network dependence.

[![HTML file created for ntwk3.s2p by running `touchstone files/ntwk3.s2p` in the root of this directory](https://github.com/iancleary/touchstone/blob/main/examples/ntwk3.s2p.html.png?raw=true)](https://github.com/iancleary/touchstone/tree/main/examples/ntwk3.s2p.html)

You can view the HTML source file itself here directly: [examples/ntwk3.s2p.html](https://github.com/iancleary/touchstone/tree/main/examples/ntwk3.s2p.html).

> Note that the crate's execution templates in the data parsed from the touchstone file's network object.  So this example will only match for the [files/ntwk3.s2p](https://github.com/iancleary/touchstone/blob/main/files/ntwk3.s2p?raw=true) data.

### Directory Path

```bash
touchstone files/
```

which outputs:

```bash
============================
In directory: files/
Directory detected. Plotting all valid network files in directory.
Found network file: "files/ntwk1.s2p"
Found network file: "files/ntwk2.s2p"
Found network file: "files/ntwk3.s2p"
Plot HTML generated at files/combined_plot.html
You can open the plot in your browser at:
file:///Users/iancleary/Development/touchstone/files/combined_plot.html
Attempting to open plot in your default browser...
Success! Opening: file:///Users/iancleary/Development/touchstone/files/combined_plot.html
```


This command created an html file that is interactive, and designed to not have any network dependence.

[![HTML file created for the files directory by running `touchstone files/` in the root of this directory](https://github.com/iancleary/touchstone/blob/main/examples/combined_plot.html.png?raw=true)](https://github.com/iancleary/touchstone/tree/main/examples/combined_plot.html)

You can view the HTML source file itself here directly: [examples/combined_plot.html](https://github.com/iancleary/touchstone/tree/main/examples/combined_plot.html).

> Note that the crate's execution templates in the data parsed from the touchstone file's network object.   So this example will only match for the [files/ntwk1.s2p](https://github.com/iancleary/touchstone/blob/main/files/ntwk1.s2p?raw=true), [files/ntwk2.s2p](https://github.com/iancleary/touchstone/blob/main/files/ntwk2.s2p?raw=true), and [files/ntwk3.s2p](https://github.com/iancleary/touchstone/blob/main/files/ntwk3.s2p?raw=true) data.

## Library Usage

The `touchstone` crate can also be used as a library in your Rust projects:

```rust
use touchstone::Network;

// Load any N-port network
let network = Network::new("path/to/file.s3p".to_string());

// Access metadata
println!("Ports: {}", network.rank);
println!("Frequency unit: {}", network.frequency_unit);
println!("Format: {}", network.format);

// Access S-parameters (1-indexed)
let s11_db = network.s_db(1, 1);  // S11 in dB format
let s21_ri = network.s_ri(2, 1);  // S21 in Real-Imaginary format
let s32_ma = network.s_ma(3, 2);  // S32 in Magnitude-Angle format

// Save network (auto-selects single-line or multi-line format)
network.save("output.s3p").unwrap();

// Cascade 2-port networks
let net1 = Network::new("amp.s2p".to_string());
let net2 = Network::new("filter.s2p".to_string());

// Standard cascade (port 2 → port 1)
let cascaded = net1.cascade(&net2);

// Cascade with port specification
let cascaded = net1.cascade_ports(&net2, 2, 1);
```

### Supported File Types

- `.s1p` - 1-port networks (e.g., terminations, loads)
- `.s2p` - 2-port networks (e.g., amplifiers, filters, cables)
- `.s3p` - 3-port networks (e.g., power dividers, circulators)
- `.s4p` - 4-port networks (e.g., differential pairs, couplers)
- `.sNp` - Any N-port network (tested up to 32-port)

### Data Formats

All three Touchstone data formats are fully supported:

- **RI** (Real-Imaginary) - Cartesian coordinates
- **MA** (Magnitude-Angle) - Polar coordinates with angle in degrees
- **DB** (Decibel-Angle) - Magnitude in dB, angle in degrees

### Frequency Units

All Touchstone frequency units are supported with automatic conversion:

- **Hz** - Hertz
- **kHz** - Kilohertz
- **MHz** - Megahertz
- **GHz** - Gigahertz
- **THz** - Terahertz

## Recent Updates

### Full N-Port Support (v0.10.4)

The library has been completely refactored to support N-port networks:

**What's New:**
- ✅ Parse any N-port Touchstone file (1 to 32+ ports tested)
- ✅ Auto-detect single-line vs multi-line format
- ✅ Save N-port networks with automatic format selection
- ✅ Access any S-parameter for any port combination
- ✅ 94 comprehensive tests with 100% pass rate
- ✅ Zero regressions - all original 2-port functionality preserved

**Technical Improvements:**
- Refactored matrix structures from hardcoded 2×2 to dynamic N×N
- Value-based multi-line segment detection
- Robust error handling with helpful messages
- Comprehensive test coverage for 1, 2, 3, 4, and higher-port files

**Backwards Compatibility:**
- All existing 2-port APIs remain unchanged
- Existing code will continue to work without modifications
- Enhanced with new capabilities for N-port support
- Plotting for 1-port and 2-port networks only
- Plotting a directory of networks is supported, but only currently if the networks are all 2-port networks (haven't tested mixed rank in a directory).

## References

* [Touchstone Wikipedia entry](https://en.wikipedia.org/wiki/Touchstone_file)
* [Touchstone File Format Specification Version 2.1](https://ibis.org/touchstone_ver2.1/touchstone_ver2_1.pdf)
  * Local Version of [Touchstone File Format Specification Version 2.1](https://github.com/iancleary/touchstone/blob/main//docs/touchstone_ver2_1.pdf)
