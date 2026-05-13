//! Columnar transposition: converts row-oriented data to column-oriented.
//! 
//! Used by writer to reorganize data before encoding/compression.

use crate::error::Result;

/// Represents columnar data: Vec of columns, each column is Vec of bytes.
pub type ColumnSet = Vec<Vec<u8>>;

/// Transposes row-oriented data into column-oriented format.
/// 
/// Input: rows where each row is a Vec of column values (as bytes)
/// Output: columns where each column is a Vec of all values for that column
pub fn transpose(rows: &[Vec<Vec<u8>>], column_count: usize) -> Result<ColumnSet> {
    if rows.is_empty() {
        return Ok(vec![Vec::new(); column_count]);
    }

    // Validate all rows have correct column count
    for row in rows {
        if row.len() != column_count {
            return Err(crate::error::Error::FileTooSmall { file_size: 0 });
        }
    }

    // Create columns
    let mut columns: ColumnSet = vec![Vec::new(); column_count];

    // Fill columns by iterating rows
    for row in rows {
        for (col_idx, col_value) in row.iter().enumerate() {
            columns[col_idx].extend_from_slice(col_value);
        }
    }

    Ok(columns)
}

/// Reverse of transpose: converts column-oriented data back to row-oriented.
/// 
/// Input: columns where each column is a Vec of bytes for all rows
/// Output: rows where each row is a Vec of column values
pub fn untranspose(columns: &ColumnSet, row_count: usize) -> Result<Vec<Vec<u8>>> {
    if columns.is_empty() {
        return Ok(vec![Vec::new(); row_count]);
    }

    // For simplicity, assume each column has row_count entries of equal size
    // (In real implementation, would parse variable-length entries)
    let entry_size = if !columns[0].is_empty() {
        columns[0].len() / row_count
    } else {
        0
    };

    let mut rows = vec![Vec::new(); row_count];

    for (_col_idx, column) in columns.iter().enumerate() {
        for row_idx in 0..row_count {
            let start = row_idx * entry_size;
            let end = start + entry_size;
            if start < column.len() {
                rows[row_idx].extend_from_slice(&column[start..end.min(column.len())]);
            }
        }
    }

    Ok(rows)
}
