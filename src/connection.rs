//! Single-pair interconnection by elimination of two internal wave unknowns.

use crate::{
    data_line::parsed_data_line_from_ri_matrix,
    data_pairs::{RealImaginary, RealImaginaryMatrix},
    numerics::ComplexLu,
    validate_reference_impedance, Complex, Network, ReferenceImpedance, TouchstoneError,
    TouchstoneWarning,
};

/// Which input network owns an original port.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkSide {
    /// The receiver of `connect`.
    Left,
    /// The argument to `connect`.
    Right,
}

/// Original identity of a surviving external port.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourcePort {
    /// Input network containing this port.
    pub side: NetworkSide,
    /// Original 1-indexed port number.
    pub port: usize,
}

/// A connected network and its external-port identities.
#[derive(Debug, Clone)]
pub struct ConnectedNetwork {
    /// Resulting S-parameter network.
    pub network: Network,
    /// Output port order: left ascending, then right ascending, excluding joined ports.
    pub surviving_ports: Vec<SourcePort>,
}

fn invalid(reason: impl Into<String>) -> TouchstoneError {
    TouchstoneError::InvalidConnection {
        reason: reason.into(),
    }
}

impl Network {
    /// Join one 1-indexed port of this network to one port of `other`.
    ///
    /// Both sources must be single-ended S data on the exact same Hz grid (display
    /// units may differ). Positive finite common or per-port real references are
    /// supported; only the joined references must match. No interpolation or
    /// renormalization is implicit. A one-port `other` is a reflection termination.
    /// At least one external port must remain, and source/result ranks are limited
    /// to 127. Public metadata and stored data are validated before elimination.
    ///
    /// The two internal waves are solved using partial-pivot LU with an absolute
    /// pivot cutoff of `1e-12`. There is no pseudoinverse or accuracy guarantee near
    /// resonance. Numerical failures carry operation, point index, and Hz context.
    /// Comments and warnings from both sources are retained left first. The output
    /// uses the left display unit/format and coherent surviving-port references.
    /// See `docs/nport-connection.md` for the derivation and limits.
    ///
    /// ```
    /// use touchstone::{Network, NetworkSide, SourcePort};
    /// let through = Network::from_str("through.s2p", "# Hz S RI R 50\n1 0 0 1 0 1 0 0 0\n")?;
    /// let load = Network::from_str("load.s1p", "# Hz S RI R 50\n1 0.2 0\n")?;
    /// let connected = through.connect(&load, 2, 1)?;
    /// assert_eq!(connected.surviving_ports, vec![SourcePort { side: NetworkSide::Left, port: 1 }]);
    /// assert_eq!(connected.network.s_matrix_at(0)?.get(1, 1)?.re, 0.2);
    /// # Ok::<(), touchstone::TouchstoneError>(())
    /// ```
    pub fn connect(
        &self,
        other: &Network,
        left_port: usize,
        right_port: usize,
    ) -> Result<ConnectedNetwork, TouchstoneError> {
        let left_refs = validate_source(self)?;
        let right_refs = validate_source(other)?;
        for (port, rank) in [(left_port, left_refs.len()), (right_port, right_refs.len())] {
            crate::validate_port_indexes(port, port, rank)?;
        }
        let rank = left_refs.len() + right_refs.len() - 2;
        if rank == 0 || rank > 127 {
            return Err(TouchstoneError::InvalidNetworkRank { rank });
        }
        if self.f != other.f {
            return Err(invalid("source frequency grids must match exactly in Hz"));
        }
        if left_refs[left_port - 1] != right_refs[right_port - 1] {
            return Err(invalid("joined-port reference impedances must match"));
        }
        let surviving_ports: Vec<_> = [
            (NetworkSide::Left, self.rank, left_port),
            (NetworkSide::Right, other.rank, right_port),
        ]
        .into_iter()
        .flat_map(|(side, count, joined)| {
            (1..=count as usize)
                .filter(move |&port| port != joined)
                .map(move |port| SourcePort { side, port })
        })
        .collect();
        let references: Vec<_> = surviving_ports
            .iter()
            .map(|port| match port.side {
                NetworkSide::Left => left_refs[port.port - 1],
                NetworkSide::Right => right_refs[port.port - 1],
            })
            .collect();
        let reference_impedance = if references.iter().all(|&z| z == references[0]) {
            ReferenceImpedance::Common(references[0])
        } else {
            ReferenceImpedance::PerPort(references.clone())
        };
        let mut s = Vec::with_capacity(self.s.len());
        for (point_index, &frequency_hz) in self.f.iter().enumerate() {
            let result = connect_point(
                self,
                other,
                point_index,
                left_port,
                right_port,
                &surviving_ports,
            )
            .map_err(|source| TouchstoneError::Connection {
                operation: "single-pair internal-wave connection".into(),
                point_index,
                frequency_hz,
                source: Box::new(source),
            })?;
            s.push(parsed_data_line_from_ri_matrix(frequency_hz, result));
        }
        let network = Network {
            name: format!(
                "Connected({}:{left_port},{}:{right_port})",
                self.name, other.name
            ),
            rank: rank as i32,
            frequency_unit: self.frequency_unit.clone(),
            parameter: "S".into(),
            format: self.format.clone(),
            resistance_string: "R".into(),
            z0: references[0],
            reference_impedance,
            comments: [self.comments.clone(), other.comments.clone()].concat(),
            comments_after_option_line: [
                self.comments_after_option_line.clone(),
                other.comments_after_option_line.clone(),
            ]
            .concat(),
            warnings: [self.warnings.clone(), other.warnings.clone()].concat(),
            f: self.f.clone(),
            s,
        };
        Ok(ConnectedNetwork {
            network,
            surviving_ports,
        })
    }
}

fn validate_source(network: &Network) -> Result<Vec<f64>, TouchstoneError> {
    if network.rank < 1 || network.rank > 127 {
        return Err(invalid("source rank must be in 1..=127"));
    }
    let rank = network.rank as usize;
    if !network.parameter.eq_ignore_ascii_case("S") {
        return Err(TouchstoneError::UnsupportedNetworkParameter {
            parameter: network.parameter.clone(),
        });
    }
    if network.warnings.iter().any(|warning| {
        matches!(warning,
        TouchstoneWarning::UnknownKeywordIgnored { keyword, .. }
        if keyword.replace('-', " ").eq_ignore_ascii_case("mixed mode order"))
    }) {
        return Err(invalid(
            "ignored Mixed-Mode Order does not establish a single-ended basis",
        ));
    }
    if !["Hz", "kHz", "MHz", "GHz", "THz"].contains(&network.frequency_unit.as_str()) {
        return Err(TouchstoneError::UnsupportedFrequencyUnit {
            unit: network.frequency_unit.clone(),
        });
    }
    if !["RI", "MA", "DB"].contains(&network.format.as_str()) {
        return Err(TouchstoneError::UnsupportedFormat {
            format: network.format.clone(),
        });
    }
    let references = match &network.reference_impedance {
        ReferenceImpedance::Common(z0) => vec![*z0; rank],
        ReferenceImpedance::PerPort(values) if values.len() == rank => values.clone(),
        ReferenceImpedance::PerPort(values) => {
            return Err(TouchstoneError::InvalidReferenceImpedanceCount {
                ports: rank,
                actual: values.len(),
            })
        }
    };
    for &z0 in &references {
        validate_reference_impedance(z0)?;
    }
    validate_reference_impedance(network.z0)?;
    if network.z0 != references[0] {
        return Err(invalid(
            "scalar z0 must equal the first reference impedance",
        ));
    }
    network.validate_frequency_data()?;
    for (point_index, row) in network.s.iter().enumerate() {
        if row.frequency != network.f[point_index] {
            return Err(TouchstoneError::FrequencyRowMismatch {
                point_index,
                vector_frequency: network.f[point_index],
                row_frequency: row.frequency,
            });
        }
        if row.s_ri.size() != rank {
            return Err(TouchstoneError::InvalidMatrixRank {
                point_index,
                matrix_rank: row.s_ri.size(),
                expected_rank: rank,
            });
        }
        crate::validate_sample_values(row).map_err(|source| TouchstoneError::Connection {
            operation: "connection source validation".into(),
            point_index,
            frequency_hz: row.frequency,
            source: Box::new(source),
        })?;
    }
    Ok(references)
}

fn connect_point(
    left: &Network,
    right: &Network,
    point: usize,
    lp: usize,
    rp: usize,
    external: &[SourcePort],
) -> Result<RealImaginaryMatrix, TouchstoneError> {
    let get = |side, to, from| {
        let source = match side {
            NetworkSide::Left => left,
            NetworkSide::Right => right,
        };
        crate::complex_from_real_imaginary(source.s[point].s_ri.get(to, from))
    };
    // P swaps internal members. No combined (n+m)-square matrix is allocated.
    let denominator = vec![
        vec![Complex::one(), -get(NetworkSide::Right, rp, rp)],
        vec![-get(NetworkSide::Left, lp, lp), Complex::one()],
    ];
    let mut rhs = vec![vec![Complex::zero(); external.len()]; 2];
    for (j, port) in external.iter().enumerate() {
        match port.side {
            NetworkSide::Left => rhs[1][j] = get(port.side, lp, port.port),
            NetworkSide::Right => rhs[0][j] = get(port.side, rp, port.port),
        }
    }
    let operation = "connection internal-wave solve";
    let x = ComplexLu::factor(denominator, operation, 1e-12)?.solve(rhs, operation)?;
    let mut data = Vec::with_capacity(external.len());
    for (i, to) in external.iter().enumerate() {
        let (internal, joined) = match to.side {
            NetworkSide::Left => (0, lp),
            NetworkSide::Right => (1, rp),
        };
        let coupling = get(to.side, to.port, joined);
        let mut row = Vec::with_capacity(external.len());
        for (j, from) in external.iter().enumerate() {
            let base = if to.side == from.side {
                get(to.side, to.port, from.port)
            } else {
                Complex::zero()
            };
            let wave = x[internal][j];
            // Fused accumulation reduces rounding/cancellation in the Schur update.
            let re = coupling
                .re
                .mul_add(wave.re, (-coupling.im).mul_add(wave.im, base.re));
            let im = coupling
                .re
                .mul_add(wave.im, coupling.im.mul_add(wave.re, base.im));
            if !re.is_finite() || !im.is_finite() {
                return Err(TouchstoneError::InvalidParameterMatrixValue {
                    matrix: "connected S".into(),
                    row: i + 1,
                    column: j + 1,
                    re,
                    im,
                });
            }
            row.push(RealImaginary(re, im));
        }
        data.push(row);
    }
    Ok(RealImaginaryMatrix::from_vec(data))
}
