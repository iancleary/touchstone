//! Common positive-real reference changes, without an S -> Z detour.

use crate::data_pairs::{RealImaginary, RealImaginaryMatrix};
use crate::{
    data_line, numerics, validate_matrix_data, validate_reference_impedance, Complex, Network,
    ReferenceImpedance, TouchstoneError, TouchstoneWarning,
};

impl Network {
    /// Transform S parameters to a new common positive, finite, real reference in ohms.
    ///
    /// Uses `(S - g I) (I - g S)^-1`, with `g = (new_z0 - old_z0)/(new_z0 + old_z0)`,
    /// evaluated by a right-sided solve, not an inverse or an S-to-Z conversion.
    /// Preserves comments, warnings, format, name, and the frequency grid/unit.
    /// Updates both reference fields and rebuilds the RI/MA/DB caches.
    ///
    /// # Errors
    ///
    /// Rejects non-S source data, per-port references (even equal ones), ignored
    /// mixed-mode semantics, unsupported source format/unit metadata, invalid
    /// grids/ranks/values, and invalid references. Formats must be exactly RI/MA/DB;
    /// units must be exactly Hz/kHz/MHz/GHz. THz is rejected because the parser
    /// does not support its roundtrip, despite writer support for that extension.
    /// Ratios whose `g` rounds to +/-1 are explicitly unsupported. Denominator
    /// pivots must exceed the absolute `1e-12` cutoff; results must be finite.
    /// These checks do not guarantee forward accuracy near singularities.
    /// A same-reference call validates the source and returns an unchanged clone.
    /// The input is never modified, including on failure.
    ///
    /// ```
    /// use touchstone::Network;
    /// let load = Network::from_str("load.s1p", "# Hz S RI R 50\n1 0.2 0\n")?;
    /// let at_75 = load.renormalize(75.0)?;
    /// assert!(at_75.try_s_ri_at(0, 1, 1)?.re.abs() < 1e-14);
    /// assert_eq!(at_75.z0, 75.0);
    /// # Ok::<(), touchstone::TouchstoneError>(())
    /// ```
    pub fn renormalize(&self, new_z0: f64) -> Result<Self, TouchstoneError> {
        let old_z0 = self.scalar_reference_impedance_for_conversions()?;
        validate_reference_impedance(new_z0)?;
        for warning in &self.warnings {
            if let TouchstoneWarning::UnknownKeywordIgnored { keyword, .. } = warning {
                let normalized = keyword
                    .to_ascii_lowercase()
                    .replace('-', " ")
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ");
                if normalized == "mixed mode order" {
                    return Err(TouchstoneError::UnsupportedRenormalizationKeyword {
                        keyword: keyword.clone(),
                    });
                }
            }
        }
        self.validate_renormalization_source()?;
        if old_z0 == new_z0 {
            return Ok(self.clone());
        }

        // Scale the sum instead of forming new+old, which can overflow.
        // The difference of positive finite values cannot overflow; taking it
        // before scaling avoids subtracting rounded ratios for close references.
        let scale = old_z0.max(new_z0);
        let ratio = old_z0.min(new_z0) / scale;
        let g = ((new_z0 - old_z0) / scale) / (1.0 + ratio);
        // Do not silently replace a finite reference with zero/infinity.
        if g.abs() >= 1.0 {
            return Err(TouchstoneError::UnrepresentableReferenceRatio { old_z0, new_z0 });
        }

        let mut transformed = Vec::with_capacity(self.s.len());
        for (point_index, point) in self.s.iter().enumerate() {
            let operation = format!(
                "renormalization {} point {point_index} ({} Hz, {old_z0} -> {new_z0} ohms)",
                self.name, point.frequency
            );
            let s = self.s_matrix_at(point_index)?;
            let mut numerator = s.data.clone();
            let mut denominator = s.data;
            for (i, (n_row, d_row)) in numerator.iter_mut().zip(&mut denominator).enumerate() {
                for (j, (n, d)) in n_row.iter_mut().zip(d_row).enumerate() {
                    let identity = if i == j {
                        Complex::one()
                    } else {
                        Complex::zero()
                    };
                    *n = *n - identity * g;
                    *d = identity - *d * g;
                }
            }
            let (values, _) =
                numerics::solve_right(denominator, numerator, &operation, 1e-12, false)?;
            let ri = RealImaginaryMatrix::from_vec(
                values
                    .into_iter()
                    .map(|row| row.into_iter().map(|v| RealImaginary(v.re, v.im)).collect())
                    .collect(),
            );
            transformed.push(data_line::parsed_data_line_from_ri_matrix(
                point.frequency,
                ri,
            ));
        }
        let mut output = self.clone();
        output.s = transformed;
        output.z0 = new_z0;
        output.reference_impedance = ReferenceImpedance::Common(new_z0);
        Ok(output)
    }

    fn validate_renormalization_source(&self) -> Result<(), TouchstoneError> {
        if !matches!(self.format.as_str(), "RI" | "MA" | "DB") {
            return Err(TouchstoneError::UnsupportedFormat {
                format: self.format.clone(),
            });
        }
        if !matches!(self.frequency_unit.as_str(), "Hz" | "kHz" | "MHz" | "GHz") {
            return Err(TouchstoneError::UnsupportedFrequencyUnit {
                unit: self.frequency_unit.clone(),
            });
        }
        let rank = usize::try_from(self.rank).unwrap_or(0);
        if rank == 0 {
            return Err(TouchstoneError::InvalidNetworkRank { rank });
        }
        self.validate_frequency_data()?;
        for (point_index, point) in self.s.iter().enumerate() {
            if point.frequency != self.f[point_index] {
                return Err(TouchstoneError::FrequencyRowMismatch {
                    point_index,
                    vector_frequency: self.f[point_index],
                    row_frequency: point.frequency,
                });
            }
            let stored_rank = point.s_ri.size();
            if stored_rank != rank {
                return Err(TouchstoneError::InvalidMatrixRank {
                    point_index,
                    matrix_rank: stored_rank,
                    expected_rank: rank,
                });
            }
            let matrix = self.s_matrix_at(point_index)?;
            validate_matrix_data(
                &format!("renormalization source point {point_index}"),
                rank,
                &matrix.data,
            )?;
        }
        Ok(())
    }
}
